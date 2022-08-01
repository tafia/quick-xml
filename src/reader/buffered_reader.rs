//! This is an implementation of [`Reader`] for reading from a [`BufRead`] as
//! underlying byte stream.

use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

use memchr;

use crate::errors::{Error, Result};
use crate::events::Event;
use crate::name::QName;
use crate::reader::{is_whitespace, BangType, ReadElementState, Reader, XmlSource};

macro_rules! impl_buffered_source {
    ($($lf:lifetime, $reader:tt, $async:ident, $await:ident)?) => {
        #[inline]
        $($async)? fn read_bytes_until $(<$lf>)? (
            &mut self,
            byte: u8,
            buf: &'b mut Vec<u8>,
            position: &mut usize,
        ) -> Result<Option<&'b [u8]>> {
            // search byte must be within the ascii range
            debug_assert!(byte.is_ascii());

            let mut read = 0;
            let mut done = false;
            let start = buf.len();
            while !done {
                let used = {
                    let available = match self $(.$reader)? .fill_buf() $(.$await)? {
                        Ok(n) if n.is_empty() => break,
                        Ok(n) => n,
                        Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                        Err(e) => {
                            *position += read;
                            return Err(Error::Io(e));
                        }
                    };

                    match memchr::memchr(byte, available) {
                        Some(i) => {
                            buf.extend_from_slice(&available[..i]);
                            done = true;
                            i + 1
                        }
                        None => {
                            buf.extend_from_slice(available);
                            available.len()
                        }
                    }
                };
                self $(.$reader)? .consume(used);
                read += used;
            }
            *position += read;

            if read == 0 {
                Ok(None)
            } else {
                Ok(Some(&buf[start..]))
            }
        }

        $($async)? fn read_bang_element $(<$lf>)? (
            &mut self,
            buf: &'b mut Vec<u8>,
            position: &mut usize,
        ) -> Result<Option<(BangType, &'b [u8])>> {
            // Peeked one bang ('!') before being called, so it's guaranteed to
            // start with it.
            let start = buf.len();
            let mut read = 1;
            buf.push(b'!');
            self $(.$reader)? .consume(1);

            let bang_type = BangType::new(self.peek_one() $(.$await)? ?)?;

            loop {
                match self $(.$reader)? .fill_buf() $(.$await)? {
                    // Note: Do not update position, so the error points to
                    // somewhere sane rather than at the EOF
                    Ok(n) if n.is_empty() => return Err(bang_type.to_err()),
                    Ok(available) => {
                        if let Some((consumed, used)) = bang_type.parse(available, read) {
                            buf.extend_from_slice(consumed);

                            self $(.$reader)? .consume(used);
                            read += used;

                            *position += read;
                            break;
                        } else {
                            buf.extend_from_slice(available);

                            let used = available.len();
                            self $(.$reader)? .consume(used);
                            read += used;
                        }
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => {
                        *position += read;
                        return Err(Error::Io(e));
                    }
                }
            }

            if read == 0 {
                Ok(None)
            } else {
                Ok(Some((bang_type, &buf[start..])))
            }
        }

        #[inline]
        $($async)? fn read_element $(<$lf>)? (
            &mut self,
            buf: &'b mut Vec<u8>,
            position: &mut usize,
        ) -> Result<Option<&'b [u8]>> {
            let mut state = ReadElementState::Elem;
            let mut read = 0;

            let start = buf.len();
            loop {
                match self $(.$reader)? .fill_buf() $(.$await)? {
                    Ok(n) if n.is_empty() => break,
                    Ok(available) => {
                        if let Some((consumed, used)) = state.change(available) {
                            buf.extend_from_slice(consumed);

                            self $(.$reader)? .consume(used);
                            read += used;

                            *position += read;
                            break;
                        } else {
                            buf.extend_from_slice(available);

                            let used = available.len();
                            self $(.$reader)? .consume(used);
                            read += used;
                        }
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => {
                        *position += read;
                        return Err(Error::Io(e));
                    }
                };
            }

            if read == 0 {
                Ok(None)
            } else {
                Ok(Some(&buf[start..]))
            }
        }

        /// Consume and discard all the whitespace until the next non-whitespace
        /// character or EOF.
        $($async)? fn skip_whitespace(&mut self, position: &mut usize) -> Result<()> {
            loop {
                break match self $(.$reader)? .fill_buf() $(.$await)? {
                    Ok(n) => {
                        let count = n.iter().position(|b| !is_whitespace(*b)).unwrap_or(n.len());
                        if count > 0 {
                            self $(.$reader)? .consume(count);
                            *position += count;
                            continue;
                        } else {
                            Ok(())
                        }
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => Err(Error::Io(e)),
                };
            }
        }

        /// Consume and discard one character if it matches the given byte. Return
        /// true if it matched.
        $($async)? fn skip_one(&mut self, byte: u8, position: &mut usize) -> Result<bool> {
            // search byte must be within the ascii range
            debug_assert!(byte.is_ascii());

            match self.peek_one() $(.$await)? ? {
                Some(b) if b == byte => {
                    *position += 1;
                    self $(.$reader)? .consume(1);
                    Ok(true)
                }
                _ => Ok(false),
            }
        }

        /// Return one character without consuming it, so that future `read_*` calls
        /// will still include it. On EOF, return None.
        $($async)? fn peek_one(&mut self) -> Result<Option<u8>> {
            loop {
                break match self $(.$reader)? .fill_buf() $(.$await)? {
                    Ok(n) if n.is_empty() => Ok(None),
                    Ok(n) => Ok(Some(n[0])),
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => Err(Error::Io(e)),
                };
            }
        }
    };
}

/// Implementation of `XmlSource` for any `BufRead` reader using a user-given
/// `Vec<u8>` as buffer that will be borrowed by events.
impl<'b, R: BufRead> XmlSource<'b, &'b mut Vec<u8>> for R {
    impl_buffered_source!();
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// This is an implementation of [`Reader`] for reading from a [`BufRead`] as
/// underlying byte stream.
impl<R: BufRead> Reader<R> {
    /// Reads the next `Event`.
    ///
    /// This is the main entry point for reading XML `Event`s.
    ///
    /// `Event`s borrow `buf` and can be converted to own their data if needed (uses `Cow`
    /// internally).
    ///
    /// Having the possibility to control the internal buffers gives you some additional benefits
    /// such as:
    ///
    /// - Reduce the number of allocations by reusing the same buffer. For constrained systems,
    ///   you can call `buf.clear()` once you are done with processing the event (typically at the
    ///   end of your loop).
    /// - Reserve the buffer length if you know the file size (using `Vec::with_capacity`).
    ///
    /// # Examples
    ///
    /// ```
    /// use quick_xml::Reader;
    /// use quick_xml::events::Event;
    ///
    /// let xml = r#"<tag1 att1 = "test">
    ///                 <tag2><!--Test comment-->Test</tag2>
    ///                 <tag2>Test 2</tag2>
    ///             </tag1>"#;
    /// let mut reader = Reader::from_str(xml);
    /// reader.trim_text(true);
    /// let mut count = 0;
    /// let mut buf = Vec::new();
    /// let mut txt = Vec::new();
    /// loop {
    ///     match reader.read_event_into(&mut buf) {
    ///         Ok(Event::Start(ref e)) => count += 1,
    ///         Ok(Event::Text(e)) => txt.push(e.unescape().unwrap().into_owned()),
    ///         Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
    ///         Ok(Event::Eof) => break,
    ///         _ => (),
    ///     }
    ///     buf.clear();
    /// }
    /// println!("Found {} start events", count);
    /// println!("Text events: {:?}", txt);
    /// ```
    #[inline]
    pub fn read_event_into<'b>(&mut self, buf: &'b mut Vec<u8>) -> Result<Event<'b>> {
        self.read_event_impl(buf)
    }

    /// Reads until end element is found using provided buffer as intermediate
    /// storage for events content. This function is supposed to be called after
    /// you already read a [`Start`] event.
    ///
    /// Manages nested cases where parent and child elements have the same name.
    ///
    /// If corresponding [`End`] event will not be found, the [`Error::UnexpectedEof`]
    /// will be returned. In particularly, that error will be returned if you call
    /// this method without consuming the corresponding [`Start`] event first.
    ///
    /// If your reader created from a string slice or byte array slice, it is
    /// better to use [`read_to_end()`] method, because it will not copy bytes
    /// into intermediate buffer.
    ///
    /// The provided `buf` buffer will be filled only by one event content at time.
    /// Before reading of each event the buffer will be cleared. If you know an
    /// appropriate size of each event, you can preallocate the buffer to reduce
    /// number of reallocations.
    ///
    /// The `end` parameter should contain name of the end element _in the reader
    /// encoding_. It is good practice to always get that parameter using
    /// [`BytesStart::to_end()`] method.
    ///
    /// The correctness of the skipped events does not checked, if you disabled
    /// the [`check_end_names`] option.
    ///
    /// # Namespaces
    ///
    /// While the [`Reader`] does not support namespace resolution, namespaces
    /// does not change the algorithm for comparing names. Although the names
    /// `a:name` and `b:name` where both prefixes `a` and `b` resolves to the
    /// same namespace, are semantically equivalent, `</b:name>` cannot close
    /// `<a:name>`, because according to [the specification]
    ///
    /// > The end of every element that begins with a **start-tag** MUST be marked
    /// > by an **end-tag** containing a name that echoes the element's type as
    /// > given in the **start-tag**
    ///
    /// # Examples
    ///
    /// This example shows, how you can skip XML content after you read the
    /// start event.
    ///
    /// ```
    /// # use pretty_assertions::assert_eq;
    /// use quick_xml::events::{BytesStart, Event};
    /// use quick_xml::Reader;
    ///
    /// let mut reader = Reader::from_str(r#"
    ///     <outer>
    ///         <inner>
    ///             <inner></inner>
    ///             <inner/>
    ///             <outer></outer>
    ///             <outer/>
    ///         </inner>
    ///     </outer>
    /// "#);
    /// reader.trim_text(true);
    /// let mut buf = Vec::new();
    ///
    /// let start = BytesStart::new("outer");
    /// let end   = start.to_end().into_owned();
    ///
    /// // First, we read a start event...
    /// assert_eq!(reader.read_event_into(&mut buf).unwrap(), Event::Start(start));
    ///
    /// //...then, we could skip all events to the corresponding end event.
    /// // This call will correctly handle nested <outer> elements.
    /// // Note, however, that this method does not handle namespaces.
    /// reader.read_to_end_into(end.name(), &mut buf).unwrap();
    ///
    /// // At the end we should get an Eof event, because we ate the whole XML
    /// assert_eq!(reader.read_event_into(&mut buf).unwrap(), Event::Eof);
    /// ```
    ///
    /// [`Start`]: Event::Start
    /// [`End`]: Event::End
    /// [`BytesStart::to_end()`]: crate::events::BytesStart::to_end
    /// [`read_to_end()`]: Self::read_to_end
    /// [`check_end_names`]: Self::check_end_names
    /// [the specification]: https://www.w3.org/TR/xml11/#dt-etag
    pub fn read_to_end_into(&mut self, end: QName, buf: &mut Vec<u8>) -> Result<()> {
        read_to_end!(self, end, buf, read_event_impl, {
            buf.clear();
        })
    }

    /// Reads optional text between start and end tags.
    ///
    /// If the next event is a [`Text`] event, returns the decoded and unescaped content as a
    /// `String`. If the next event is an [`End`] event, returns the empty string. In all other
    /// cases, returns an error.
    ///
    /// Any text will be decoded using the XML encoding specified in the XML declaration (or UTF-8
    /// if none is specified).
    ///
    /// # Examples
    ///
    /// ```
    /// # use pretty_assertions::assert_eq;
    /// use quick_xml::Reader;
    /// use quick_xml::events::Event;
    ///
    /// let mut xml = Reader::from_reader(b"
    ///     <a>&lt;b&gt;</a>
    ///     <a></a>
    /// " as &[u8]);
    /// xml.trim_text(true);
    ///
    /// let expected = ["<b>", ""];
    /// for &content in expected.iter() {
    ///     match xml.read_event_into(&mut Vec::new()) {
    ///         Ok(Event::Start(ref e)) => {
    ///             assert_eq!(&xml.read_text_into(e.name(), &mut Vec::new()).unwrap(), content);
    ///         },
    ///         e => panic!("Expecting Start event, found {:?}", e),
    ///     }
    /// }
    /// ```
    ///
    /// [`Text`]: Event::Text
    /// [`End`]: Event::End
    pub fn read_text_into(&mut self, end: QName, buf: &mut Vec<u8>) -> Result<String> {
        let s = match self.read_event_into(buf) {
            Err(e) => return Err(e),

            Ok(Event::Text(e)) => e.unescape()?.into_owned(),
            Ok(Event::End(e)) if e.name() == end => return Ok("".to_string()),
            Ok(Event::Eof) => return Err(Error::UnexpectedEof("Text".to_string())),
            _ => return Err(Error::TextNotFound),
        };
        self.read_to_end_into(end, buf)?;
        Ok(s)
    }
}

impl Reader<BufReader<File>> {
    /// Creates an XML reader from a file path.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path).map_err(Error::Io)?;
        let reader = BufReader::new(file);
        Ok(Self::from_reader(reader))
    }
}

#[cfg(test)]
mod test {
    use crate::reader::test::check;
    use crate::reader::XmlSource;

    /// Default buffer constructor just pass the byte array from the test
    fn identity<T>(input: T) -> T {
        input
    }

    check!(
        #[test]
        read_event_impl,
        read_until_close,
        identity,
        &mut Vec::new()
    );

    #[cfg(feature = "encoding")]
    mod encoding {
        use crate::events::Event;
        use crate::reader::Reader;
        use encoding_rs::{UTF_16LE, UTF_8, WINDOWS_1251};
        use pretty_assertions::assert_eq;

        /// Checks that encoding is detected by BOM and changed after XML declaration
        #[test]
        fn bom_detected() {
            let mut reader =
                Reader::from_reader(b"\xFF\xFE<?xml encoding='windows-1251'?>".as_ref());
            let mut buf = Vec::new();

            assert_eq!(reader.decoder().encoding(), UTF_8);
            reader.read_event_into(&mut buf).unwrap();
            assert_eq!(reader.decoder().encoding(), UTF_16LE);

            reader.read_event_into(&mut buf).unwrap();
            assert_eq!(reader.decoder().encoding(), WINDOWS_1251);

            assert_eq!(reader.read_event_into(&mut buf).unwrap(), Event::Eof);
        }

        /// Checks that encoding is changed by XML declaration, but only once
        #[test]
        fn xml_declaration() {
            let mut reader = Reader::from_reader(
                b"<?xml encoding='UTF-16'?><?xml encoding='windows-1251'?>".as_ref(),
            );
            let mut buf = Vec::new();

            assert_eq!(reader.decoder().encoding(), UTF_8);
            reader.read_event_into(&mut buf).unwrap();
            assert_eq!(reader.decoder().encoding(), UTF_16LE);

            reader.read_event_into(&mut buf).unwrap();
            assert_eq!(reader.decoder().encoding(), UTF_16LE);

            assert_eq!(reader.read_event_into(&mut buf).unwrap(), Event::Eof);
        }
    }
}

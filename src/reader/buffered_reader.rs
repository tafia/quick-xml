//! This is an implementation of [`Reader`] for reading from a [`BufRead`] as
//! underlying byte stream.

use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

use crate::errors::{Error, Result};
use crate::events::Event;
use crate::name::{QName, ResolveResult};
use crate::reader::{is_whitespace, BangType, ReadElementState, Reader, XmlSource};

use memchr;

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
    ///         Ok(Event::Text(e)) => txt.push(e.decode_and_unescape(&reader).unwrap().into_owned()),
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

    /// Reads the next event and resolves its namespace (if applicable).
    ///
    /// # Examples
    ///
    /// ```
    /// use std::str::from_utf8;
    /// use quick_xml::Reader;
    /// use quick_xml::events::Event;
    /// use quick_xml::name::ResolveResult::*;
    ///
    /// let xml = r#"<x:tag1 xmlns:x="www.xxxx" xmlns:y="www.yyyy" att1 = "test">
    ///                 <y:tag2><!--Test comment-->Test</y:tag2>
    ///                 <y:tag2>Test 2</y:tag2>
    ///             </x:tag1>"#;
    /// let mut reader = Reader::from_str(xml);
    /// reader.trim_text(true);
    /// let mut count = 0;
    /// let mut buf = Vec::new();
    /// let mut ns_buf = Vec::new();
    /// let mut txt = Vec::new();
    /// loop {
    ///     match reader.read_namespaced_event_into(&mut buf, &mut ns_buf) {
    ///         Ok((Bound(ns), Event::Start(e))) => {
    ///             count += 1;
    ///             match (ns.as_ref(), e.local_name().as_ref()) {
    ///                 (b"www.xxxx", b"tag1") => (),
    ///                 (b"www.yyyy", b"tag2") => (),
    ///                 (ns, n) => panic!("Namespace and local name mismatch"),
    ///             }
    ///             println!("Resolved namespace: {:?}", ns);
    ///         }
    ///         Ok((Unbound, Event::Start(_))) => {
    ///             panic!("Element not in any namespace")
    ///         },
    ///         Ok((Unknown(p), Event::Start(_))) => {
    ///             panic!("Undeclared namespace prefix {:?}", String::from_utf8(p))
    ///         }
    ///         Ok((_, Event::Text(e))) => {
    ///             txt.push(e.decode_and_unescape(&reader).unwrap().into_owned())
    ///         },
    ///         Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
    ///         Ok((_, Event::Eof)) => break,
    ///         _ => (),
    ///     }
    ///     buf.clear();
    /// }
    /// println!("Found {} start events", count);
    /// println!("Text events: {:?}", txt);
    /// ```
    pub fn read_namespaced_event_into<'b, 'ns>(
        &mut self,
        buf: &'b mut Vec<u8>,
        namespace_buffer: &'ns mut Vec<u8>,
    ) -> Result<(ResolveResult<'ns>, Event<'b>)> {
        if self.pending_pop {
            self.ns_resolver.pop(namespace_buffer);
        }
        self.pending_pop = false;
        let event = self.read_event_into(buf);
        self.resolve_namespaced_event_inner(event, namespace_buffer)
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
    /// let start = BytesStart::borrowed_name(b"outer");
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
        let mut depth = 0;
        loop {
            buf.clear();
            match self.read_event_into(buf) {
                Err(e) => return Err(e),

                Ok(Event::Start(e)) if e.name() == end => depth += 1,
                Ok(Event::End(e)) if e.name() == end => {
                    if depth == 0 {
                        return Ok(());
                    }
                    depth -= 1;
                }
                Ok(Event::Eof) => {
                    let name = self.decoder().decode(end.as_ref());
                    return Err(Error::UnexpectedEof(format!("</{:?}>", name)));
                }
                _ => (),
            }
        }
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

            Ok(Event::Text(e)) => e.decode_and_unescape(self)?.into_owned(),
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

////////////////////////////////////////////////////////////////////////////////////////////////////

/// Implementation of `XmlSource` for any `BufRead` reader using a user-given
/// `Vec<u8>` as buffer that will be borrowed by events.
impl<'b, R: BufRead> XmlSource<'b, &'b mut Vec<u8>> for R {
    #[inline]
    fn read_bytes_until(
        &mut self,
        byte: u8,
        buf: &'b mut Vec<u8>,
        position: &mut usize,
    ) -> Result<Option<&'b [u8]>> {
        let mut read = 0;
        let mut done = false;
        let start = buf.len();
        while !done {
            let used = {
                let available = match self.fill_buf() {
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
            self.consume(used);
            read += used;
        }
        *position += read;

        if read == 0 {
            Ok(None)
        } else {
            Ok(Some(&buf[start..]))
        }
    }

    fn read_bang_element(
        &mut self,
        buf: &'b mut Vec<u8>,
        position: &mut usize,
    ) -> Result<Option<(BangType, &'b [u8])>> {
        // Peeked one bang ('!') before being called, so it's guaranteed to
        // start with it.
        let start = buf.len();
        let mut read = 1;
        buf.push(b'!');
        self.consume(1);

        let bang_type = BangType::new(self.peek_one()?)?;

        loop {
            match self.fill_buf() {
                // Note: Do not update position, so the error points to
                // somewhere sane rather than at the EOF
                Ok(n) if n.is_empty() => return Err(bang_type.to_err()),
                Ok(available) => {
                    if let Some((consumed, used)) = bang_type.parse(available, read) {
                        buf.extend_from_slice(consumed);

                        self.consume(used);
                        read += used;

                        *position += read;
                        break;
                    } else {
                        buf.extend_from_slice(available);

                        let used = available.len();
                        self.consume(used);
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
    fn read_element(
        &mut self,
        buf: &'b mut Vec<u8>,
        position: &mut usize,
    ) -> Result<Option<&'b [u8]>> {
        let mut state = ReadElementState::Elem;
        let mut read = 0;

        let start = buf.len();
        loop {
            match self.fill_buf() {
                Ok(n) if n.is_empty() => break,
                Ok(available) => {
                    if let Some((consumed, used)) = state.change(available) {
                        buf.extend_from_slice(consumed);

                        self.consume(used);
                        read += used;

                        *position += read;
                        break;
                    } else {
                        buf.extend_from_slice(available);

                        let used = available.len();
                        self.consume(used);
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
    fn skip_whitespace(&mut self, position: &mut usize) -> Result<()> {
        loop {
            break match self.fill_buf() {
                Ok(n) => {
                    let count = n.iter().position(|b| !is_whitespace(*b)).unwrap_or(n.len());
                    if count > 0 {
                        self.consume(count);
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
    fn skip_one(&mut self, byte: u8, position: &mut usize) -> Result<bool> {
        match self.peek_one()? {
            Some(b) if b == byte => {
                *position += 1;
                self.consume(1);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// Return one character without consuming it, so that future `read_*` calls
    /// will still include it. On EOF, return None.
    fn peek_one(&mut self) -> Result<Option<u8>> {
        loop {
            break match self.fill_buf() {
                Ok(n) if n.is_empty() => Ok(None),
                Ok(n) => Ok(Some(n[0])),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => Err(Error::Io(e)),
            };
        }
    }
}

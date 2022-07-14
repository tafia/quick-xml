//! This is an implementation of [`Reader`] for reading from a `&[u8]` as
//! underlying byte stream. This implementation supports not using an
//! intermediate buffer as the byte slice itself can be used to borrow from.

use std::ops::{Deref, DerefMut};

#[cfg(feature = "encoding")]
use encoding_rs::UTF_8;

use crate::events::{BytesText, Event};
use crate::name::{QName, ResolveResult};
use crate::{Error, Result};

#[cfg(feature = "encoding")]
use super::{detect_encoding, EncodingRef};
use super::{is_whitespace, BangType, InnerReader, ReadElementState, Reader, TagState};

/// A struct for handling reading functions based on reading from a byte slice.
#[derive(Debug, Clone, Copy)]
pub struct SliceReader<'buf>(&'buf [u8]);

impl<'buf> Deref for SliceReader<'buf> {
    type Target = &'buf [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'buf> DerefMut for SliceReader<'buf> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'buf> InnerReader for SliceReader<'buf> {
    type Reader = &'buf [u8];

    fn into_inner(self) -> Self::Reader {
        self.0
    }
}

/// Private reading functions for a [`SliceReader`].
impl<'buf> SliceReader<'buf> {
    fn read_bytes_until(
        &mut self,
        byte: u8,
        _buf: &mut (),
        position: &mut usize,
    ) -> Result<Option<&'buf [u8]>> {
        if self.is_empty() {
            return Ok(None);
        }

        Ok(Some(if let Some(i) = memchr::memchr(byte, self) {
            *position += i + 1;
            let bytes = &self[..i];
            self.0 = &self[i + 1..];
            bytes
        } else {
            *position += self.len();
            let bytes = &self[..];
            self.0 = &[];
            bytes
        }))
    }

    fn read_bang_element(
        &mut self,
        _buf: &mut (),
        position: &mut usize,
    ) -> Result<Option<(BangType, &'buf [u8])>> {
        // Peeked one bang ('!') before being called, so it's guaranteed to
        // start with it.
        debug_assert_eq!(self[0], b'!');

        let bang_type = BangType::new(self[1..].first().copied())?;

        if let Some((bytes, i)) = bang_type.parse(self, 0) {
            *position += i;
            self.0 = &self[i..];
            return Ok(Some((bang_type, bytes)));
        }

        // Note: Do not update position, so the error points to
        // somewhere sane rather than at the EOF
        Err(bang_type.to_err())
    }

    fn read_element(&mut self, _buf: &mut (), position: &mut usize) -> Result<Option<&'buf [u8]>> {
        if self.is_empty() {
            return Ok(None);
        }

        let mut state = ReadElementState::Elem;

        if let Some((bytes, i)) = state.change(self) {
            *position += i;
            self.0 = &self[i..];
            return Ok(Some(bytes));
        }

        // Note: Do not update position, so the error points to a sane place
        // rather than at the EOF.
        Err(Error::UnexpectedEof("Element".to_string()))

        // FIXME: Figure out why the other one works without UnexpectedEof
    }

    fn skip_whitespace(&mut self, position: &mut usize) -> Result<()> {
        let whitespaces = self
            .iter()
            .position(|b| !is_whitespace(*b))
            .unwrap_or(self.len());
        *position += whitespaces;
        self.0 = &self[whitespaces..];
        Ok(())
    }

    fn skip_one(&mut self, byte: u8, position: &mut usize) -> Result<bool> {
        if self.first() == Some(&byte) {
            self.0 = &self[1..];
            *position += 1;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn peek_one(&mut self) -> Result<Option<u8>> {
        Ok(self.first().copied())
    }
}

/// Private functions for a [`Reader`] based on a [`SliceReader`].
impl<'buf> Reader<SliceReader<'buf>> {
    /// Read text into the given buffer, and return an event that borrows from
    /// either that buffer or from the input itself, based on the type of the
    /// reader.
    fn read_event_impl(&mut self, _buf: &mut ()) -> Result<Event<'buf>> {
        let event = match self.tag_state {
            TagState::Init => self.read_until_open(&mut (), true),
            TagState::Closed => self.read_until_open(&mut (), false),
            TagState::Opened => self.read_until_close(&mut ()),
            TagState::Empty => self.close_expanded_empty(),
            TagState::Exit => return Ok(Event::Eof),
        };
        match event {
            Err(_) | Ok(Event::Eof) => self.tag_state = TagState::Exit,
            _ => {}
        }
        event
    }

    /// Read until '<' is found and moves reader to an `Opened` state.
    ///
    /// Return a `StartText` event if `first` is `true` and a `Text` event otherwise
    fn read_until_open(&mut self, _buf: &mut (), first: bool) -> Result<Event<'buf>> {
        self.tag_state = TagState::Opened;

        if self.trim_text_start {
            self.reader.skip_whitespace(&mut self.buf_position)?;
        }

        // If we already at the `<` symbol, do not try to return an empty Text event
        if self.reader.skip_one(b'<', &mut self.buf_position)? {
            return self.read_event_impl(&mut ());
        }

        match self
            .reader
            .read_bytes_until(b'<', &mut (), &mut self.buf_position)
        {
            Ok(Some(bytes)) => {
                #[cfg(feature = "encoding")]
                if first && self.encoding.can_be_refined() {
                    if let Some(encoding) = detect_encoding(bytes) {
                        self.encoding = EncodingRef::BomDetected(encoding);
                    }
                }

                let content = if self.trim_text_end {
                    // Skip the ending '<
                    let len = bytes
                        .iter()
                        .rposition(|&b| !is_whitespace(b))
                        .map_or_else(|| bytes.len(), |p| p + 1);
                    &bytes[..len]
                } else {
                    bytes
                };

                Ok(if first {
                    Event::StartText(BytesText::from_escaped(content).into())
                } else {
                    Event::Text(BytesText::from_escaped(content))
                })
            }
            Ok(None) => Ok(Event::Eof),
            Err(e) => Err(e),
        }
    }

    /// Private function to read until `>` is found. This function expects that
    /// it was called just after encounter a `<` symbol.
    fn read_until_close(&mut self, _buf: &mut ()) -> Result<Event<'buf>> {
        self.tag_state = TagState::Closed;

        match self.reader.peek_one() {
            // `<!` - comment, CDATA or DOCTYPE declaration
            Ok(Some(b'!')) => match self
                .reader
                .read_bang_element(&mut (), &mut self.buf_position)
            {
                Ok(None) => Ok(Event::Eof),
                Ok(Some((bang_type, bytes))) => self.read_bang(bang_type, bytes),
                Err(e) => Err(e),
            },
            // `</` - closing tag
            Ok(Some(b'/')) => {
                match self
                    .reader
                    .read_bytes_until(b'>', &mut (), &mut self.buf_position)
                {
                    Ok(None) => Ok(Event::Eof),
                    Ok(Some(bytes)) => self.read_end(bytes),
                    Err(e) => Err(e),
                }
            }
            // `<?` - processing instruction
            Ok(Some(b'?')) => {
                match self
                    .reader
                    .read_bytes_until(b'>', &mut (), &mut self.buf_position)
                {
                    Ok(None) => Ok(Event::Eof),
                    Ok(Some(bytes)) => self.read_question_mark(bytes),
                    Err(e) => Err(e),
                }
            }
            // `<...` - opening or self-closed tag
            Ok(Some(_)) => match self.reader.read_element(&mut (), &mut self.buf_position) {
                Ok(None) => Ok(Event::Eof),
                Ok(Some(bytes)) => self.read_start(bytes),
                Err(e) => Err(e),
            },
            Ok(None) => Ok(Event::Eof),
            Err(e) => Err(e),
        }
    }
}

/// Builder for reading from a slice of bytes.
impl<'buf> Reader<SliceReader<'buf>> {
    /// Creates an XML reader from a string slice.
    pub fn from_str(s: &'buf str) -> Self {
        #[cfg_attr(not(feature = "encoding"), allow(unused_mut))]
        let mut reader = Self::from_reader_internal(SliceReader(s.as_bytes()));

        // Rust strings are guaranteed to be UTF-8, so lock the encoding
        #[cfg(feature = "encoding")]
        {
            reader.encoding = EncodingRef::Explicit(UTF_8);
        }

        reader
    }

    /// Creates an XML reader from a slice of bytes.
    pub fn from_bytes(s: &'buf [u8]) -> Self {
        Self::from_reader_internal(SliceReader(s))
    }
}

/// Public reading methods for a [`Reader`] based on an [`SliceReader`].
impl<'buf> Reader<SliceReader<'buf>> {
    /// Read an event that borrows from the input rather than a buffer.
    #[inline]
    pub fn read_event(&mut self) -> Result<Event<'buf>> {
        self.read_event_impl(&mut ())
    }

    /// Temporary helper to keep both `read_event` and `read_event_into` available for reading
    /// from `&[u8]`.
    #[inline]
    pub fn read_event_into(&mut self, _buf: &mut Vec<u8>) -> Result<Event<'buf>> {
        self.read_event()
    }

    /// Reads until end element is found. This function is supposed to be called
    /// after you already read a [`Start`] event.
    ///
    /// Manages nested cases where parent and child elements have the same name.
    ///
    /// If corresponding [`End`] event will not be found, the [`Error::UnexpectedEof`]
    /// will be returned. In particularly, that error will be returned if you call
    /// this method without consuming the corresponding [`Start`] event first.
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
    ///
    /// let start = BytesStart::borrowed_name(b"outer");
    /// let end   = start.to_end().into_owned();
    ///
    /// // First, we read a start event...
    /// assert_eq!(reader.read_event().unwrap(), Event::Start(start));
    ///
    /// //...then, we could skip all events to the corresponding end event.
    /// // This call will correctly handle nested <outer> elements.
    /// // Note, however, that this method does not handle namespaces.
    /// reader.read_to_end(end.name()).unwrap();
    ///
    /// // At the end we should get an Eof event, because we ate the whole XML
    /// assert_eq!(reader.read_event().unwrap(), Event::Eof);
    /// ```
    ///
    /// [`Start`]: Event::Start
    /// [`End`]: Event::End
    /// [`check_end_names`]: Self::check_end_names
    /// [the specification]: https://www.w3.org/TR/xml11/#dt-etag
    pub fn read_to_end(&mut self, end: QName) -> Result<()> {
        let mut depth = 0;
        loop {
            match self.read_event() {
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

    /// Temporary helper to keep both `read_to_end` and `read_to_end_into` available for reading
    /// from `&[u8]`.
    pub fn read_to_end_into(&mut self, end: QName, _buf: &mut Vec<u8>) -> Result<()> {
        self.read_to_end(end)
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
    pub fn read_text(&mut self, end: QName) -> Result<String> {
        let s = match self.read_event() {
            Err(e) => return Err(e),

            Ok(Event::Text(e)) => e.decode_and_unescape(self)?.into_owned(),
            Ok(Event::End(e)) if e.name() == end => return Ok("".to_string()),
            Ok(Event::Eof) => return Err(Error::UnexpectedEof("Text".to_string())),
            _ => return Err(Error::TextNotFound),
        };
        self.read_to_end(end)?;
        Ok(s)
    }

    /// Temporary helper to keep both `read_text` and `read_text_into` available for reading
    /// from `&[u8]`.
    pub fn read_text_into(&mut self, end: QName, _buf: &mut Vec<u8>) -> Result<String> {
        self.read_text(end)
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
    ///     match reader.read_namespaced_event(&mut buf, &mut ns_buf) {
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
    /// }
    /// println!("Found {} start events", count);
    /// println!("Text events: {:?}", txt);
    /// ```
    pub fn read_namespaced_event<'ns>(
        &mut self,
        _buf: &mut Vec<u8>,
        namespace_buffer: &'ns mut Vec<u8>,
    ) -> Result<(ResolveResult<'ns>, Event<'buf>)> {
        if self.pending_pop {
            self.ns_resolver.pop(namespace_buffer);
        }
        self.pending_pop = false;
        let event = self.read_event();
        self.resolve_namespaced_event_inner(event, namespace_buffer)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::reader::test::check;

    fn input_from_bytes<'buf>(bytes: &'buf [u8]) -> SliceReader<'buf> {
        SliceReader(bytes)
    }

    fn reader_from_str<'buf>(s: &'buf str) -> Reader<SliceReader<'buf>> {
        Reader::from_str(s)
    }

    #[allow(dead_code)]
    fn reader_from_bytes<'buf>(s: &'buf [u8]) -> Reader<SliceReader<'buf>> {
        Reader::from_bytes(s)
    }

    check!(let mut buf = (););
}

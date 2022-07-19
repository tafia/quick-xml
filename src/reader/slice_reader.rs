//! This is an implementation of [`Reader`] for reading from a `&[u8]` as
//! underlying byte stream. This implementation supports not using an
//! intermediate buffer as the byte slice itself can be used to borrow from.

#[cfg(feature = "encoding")]
use crate::reader::EncodingRef;
#[cfg(feature = "encoding")]
use encoding_rs::UTF_8;

use crate::errors::{Error, Result};
use crate::events::Event;
use crate::name::{QName, ResolveResult};
use crate::reader::{is_whitespace, BangType, ReadElementState, Reader};

use memchr;

/// This is an implementation of [`Reader`] for reading from a `&[u8]` as
/// underlying byte stream. This implementation supports not using an
/// intermediate buffer as the byte slice itself can be used to borrow from.
impl<'a> Reader<&'a [u8]> {
    /// Creates an XML reader from a string slice.
    pub fn from_str(s: &'a str) -> Self {
        // Rust strings are guaranteed to be UTF-8, so lock the encoding
        #[cfg(feature = "encoding")]
        {
            let mut reader = Self::from_reader(s.as_bytes());
            reader.encoding = EncodingRef::Explicit(UTF_8);
            reader
        }

        #[cfg(not(feature = "encoding"))]
        Self::from_reader(s.as_bytes())
    }

    /// Creates an XML reader from a slice of bytes.
    pub fn from_bytes(s: &'a [u8]) -> Self {
        Self::from_reader(s)
    }

    /// Read an event that borrows from the input rather than a buffer.
    #[inline]
    pub fn read_event(&mut self) -> Result<Event<'a>> {
        self.read_event_impl(())
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
    /// [`BytesStart::to_end()`]: crate::events::BytesStart::to_end
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
    /// let mut ns_buf = Vec::new();
    /// let mut txt = Vec::new();
    /// loop {
    ///     match reader.read_namespaced_event(&mut ns_buf) {
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
        namespace_buffer: &'ns mut Vec<u8>,
    ) -> Result<(ResolveResult<'ns>, Event<'a>)> {
        if self.pending_pop {
            self.ns_resolver.pop(namespace_buffer);
        }
        self.pending_pop = false;
        let event = self.read_event();
        self.resolve_namespaced_event_inner(event, namespace_buffer)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// A struct for handling reading functions based on reading from a byte slice.
#[derive(Debug, Clone, Copy)]
pub struct SliceReader<'buf>(&'buf [u8]);

/// Private reading functions for a [`SliceReader`].
impl<'buf> SliceReader<'buf> {
    fn read_bytes_until(
        &mut self,
        byte: u8,
        _buf: (),
        position: &mut usize,
    ) -> Result<Option<&'buf [u8]>> {
        if self.0.is_empty() {
            return Ok(None);
        }

        Ok(Some(if let Some(i) = memchr::memchr(byte, self.0) {
            *position += i + 1;
            let bytes = &self.0[..i];
            self.0 = &self.0[i + 1..];
            bytes
        } else {
            *position += self.0.len();
            let bytes = &self.0[..];
            self.0 = &[];
            bytes
        }))
    }

    fn read_bang_element(
        &mut self,
        _buf: (),
        position: &mut usize,
    ) -> Result<Option<(BangType, &'buf [u8])>> {
        // Peeked one bang ('!') before being called, so it's guaranteed to
        // start with it.
        debug_assert_eq!(self.0[0], b'!');

        let bang_type = BangType::new(self.0[1..].first().copied())?;

        if let Some((bytes, i)) = bang_type.parse(self.0, 0) {
            *position += i;
            self.0 = &self.0[i..];
            return Ok(Some((bang_type, bytes)));
        }

        // Note: Do not update position, so the error points to
        // somewhere sane rather than at the EOF
        Err(bang_type.to_err())
    }

    fn read_element(&mut self, _buf: (), position: &mut usize) -> Result<Option<&'buf [u8]>> {
        if self.0.is_empty() {
            return Ok(None);
        }

        let mut state = ReadElementState::Elem;

        if let Some((bytes, i)) = state.change(self.0) {
            *position += i;
            self.0 = &self.0[i..];
            return Ok(Some(bytes));
        }

        // Note: Do not update position, so the error points to a sane place
        // rather than at the EOF.
        Err(Error::UnexpectedEof("Element".to_string()))

        // FIXME: Figure out why the other one works without UnexpectedEof
    }

    fn skip_whitespace(&mut self, position: &mut usize) -> Result<()> {
        let whitespaces = self
            .0
            .iter()
            .position(|b| !is_whitespace(*b))
            .unwrap_or(self.0.len());
        *position += whitespaces;
        self.0 = &self.0[whitespaces..];
        Ok(())
    }

    fn skip_one(&mut self, byte: u8, position: &mut usize) -> Result<bool> {
        if self.0.first() == Some(&byte) {
            self.0 = &self.0[1..];
            *position += 1;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn peek_one(&mut self) -> Result<Option<u8>> {
        Ok(self.0.first().copied())
    }
}

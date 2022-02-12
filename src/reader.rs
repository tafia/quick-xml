//! A module to handle `Reader`

#[cfg(feature = "encoding")]
use std::borrow::Cow;
use std::io::{self, BufRead, BufReader};
use std::{fs::File, path::Path, str::from_utf8};

#[cfg(feature = "encoding")]
use encoding_rs::{Encoding, UTF_16BE, UTF_16LE};

use crate::errors::{Error, Result};
use crate::events::{attributes::Attribute, BytesDecl, BytesEnd, BytesStart, BytesText, Event};

use memchr;

#[derive(Clone)]
enum TagState {
    Opened,
    Closed,
    Empty,
    /// Either Eof or Errored
    Exit,
}

/// A low level encoding-agnostic XML event reader.
///
/// Consumes a `BufRead` and streams XML `Event`s.
///
/// # Examples
///
/// ```
/// use fast_xml::Reader;
/// use fast_xml::events::Event;
///
/// let xml = r#"<tag1 att1 = "test">
///                 <tag2><!--Test comment-->Test</tag2>
///                 <tag2>Test 2</tag2>
///             </tag1>"#;
/// let mut reader = Reader::from_str(xml);
/// reader.trim_text(true);
/// let mut count = 0;
/// let mut txt = Vec::new();
/// let mut buf = Vec::new();
/// loop {
///     match reader.read_event(&mut buf) {
///         Ok(Event::Start(ref e)) => {
///             match e.name() {
///                 b"tag1" => println!("attributes values: {:?}",
///                                     e.attributes().map(|a| a.unwrap().value)
///                                     .collect::<Vec<_>>()),
///                 b"tag2" => count += 1,
///                 _ => (),
///             }
///         },
///         Ok(Event::Text(e)) => txt.push(e.unescape_and_decode(&reader).unwrap()),
///         Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
///         Ok(Event::Eof) => break,
///         _ => (),
///     }
///     buf.clear();
/// }
/// ```
#[derive(Clone)]
pub struct Reader<R: BufRead> {
    /// reader
    pub(crate) reader: R,
    /// current buffer position, useful for debugging errors
    buf_position: usize,
    /// current state Open/Close
    tag_state: TagState,
    /// expand empty element into an opening and closing element
    expand_empty_elements: bool,
    /// trims leading whitespace in Text events, skip the element if text is empty
    trim_text_start: bool,
    /// trims trailing whitespace in Text events.
    trim_text_end: bool,
    /// trims trailing whitespaces from markup names in closing tags `</a >`
    trim_markup_names_in_closing_tags: bool,
    /// check if End nodes match last Start node
    check_end_names: bool,
    /// check if comments contains `--` (false per default)
    check_comments: bool,
    /// all currently Started elements which didn't have a matching
    /// End element yet
    opened_buffer: Vec<u8>,
    /// opened name start indexes
    opened_starts: Vec<usize>,
    /// a buffer to manage namespaces
    ns_buffer: NamespaceBufferIndex,
    #[cfg(feature = "encoding")]
    /// the encoding specified in the xml, defaults to utf8
    encoding: &'static Encoding,
    #[cfg(feature = "encoding")]
    /// check if quick-rs could find out the encoding
    is_encoding_set: bool,
}

impl<R: BufRead> Reader<R> {
    /// Creates a `Reader` that reads from a reader implementing `BufRead`.
    pub fn from_reader(reader: R) -> Reader<R> {
        Reader {
            reader,
            opened_buffer: Vec::new(),
            opened_starts: Vec::new(),
            tag_state: TagState::Closed,
            expand_empty_elements: false,
            trim_text_start: false,
            trim_text_end: false,
            trim_markup_names_in_closing_tags: true,
            check_end_names: true,
            buf_position: 0,
            check_comments: false,
            ns_buffer: NamespaceBufferIndex::default(),
            #[cfg(feature = "encoding")]
            encoding: ::encoding_rs::UTF_8,
            #[cfg(feature = "encoding")]
            is_encoding_set: false,
        }
    }

    /// Changes whether empty elements should be split into an `Open` and a `Close` event.
    ///
    /// When set to `true`, all [`Empty`] events produced by a self-closing tag like `<tag/>` are
    /// expanded into a [`Start`] event followed by a [`End`] event. When set to `false` (the
    /// default), those tags are represented by an [`Empty`] event instead.
    ///
    /// (`false` by default)
    ///
    /// [`Empty`]: events/enum.Event.html#variant.Empty
    /// [`Start`]: events/enum.Event.html#variant.Start
    /// [`End`]: events/enum.Event.html#variant.End
    pub fn expand_empty_elements(&mut self, val: bool) -> &mut Reader<R> {
        self.expand_empty_elements = val;
        self
    }

    /// Changes whether whitespace before and after character data should be removed.
    ///
    /// When set to `true`, all [`Text`] events are trimmed. If they are empty, no event will be
    /// pushed.
    ///
    /// (`false` by default)
    ///
    /// [`Text`]: events/enum.Event.html#variant.Text
    pub fn trim_text(&mut self, val: bool) -> &mut Reader<R> {
        self.trim_text_start = val;
        self.trim_text_end = val;
        self
    }

    /// Changes whether whitespace after character data should be removed.
    ///
    /// When set to `true`, trailing whitespace is trimmed in [`Text`] events.
    ///
    /// (`false` by default)
    ///
    /// [`Text`]: events/enum.Event.html#variant.Text
    pub fn trim_text_end(&mut self, val: bool) -> &mut Reader<R> {
        self.trim_text_end = val;
        self
    }

    /// Changes whether trailing whitespaces after the markup name are trimmed in closing tags
    /// `</a >`.
    ///
    /// If true the emitted [`End`] event is stripped of trailing whitespace after the markup name.
    ///
    /// Note that if set to `false` and `check_end_names` is true the comparison of markup names is
    /// going to fail erronously if a closing tag contains trailing whitespaces.
    ///
    /// (`true` by default)
    ///
    /// [`End`]: events/enum.Event.html#variant.End
    pub fn trim_markup_names_in_closing_tags(&mut self, val: bool) -> &mut Reader<R> {
        self.trim_markup_names_in_closing_tags = val;
        self
    }

    /// Changes whether mismatched closing tag names should be detected.
    ///
    /// When set to `false`, it won't check if a closing tag matches the corresponding opening tag.
    /// For example, `<mytag></different_tag>` will be permitted.
    ///
    /// If the XML is known to be sane (already processed, etc.) this saves extra time.
    ///
    /// Note that the emitted [`End`] event will not be modified if this is disabled, ie. it will
    /// contain the data of the mismatched end tag.
    ///
    /// (`true` by default)
    ///
    /// [`End`]: events/enum.Event.html#variant.End
    pub fn check_end_names(&mut self, val: bool) -> &mut Reader<R> {
        self.check_end_names = val;
        self
    }

    /// Changes whether comments should be validated.
    ///
    /// When set to `true`, every [`Comment`] event will be checked for not containing `--`, which
    /// is not allowed in XML comments. Most of the time we don't want comments at all so we don't
    /// really care about comment correctness, thus the default value is `false` to improve
    /// performance.
    ///
    /// (`false` by default)
    ///
    /// [`Comment`]: events/enum.Event.html#variant.Comment
    pub fn check_comments(&mut self, val: bool) -> &mut Reader<R> {
        self.check_comments = val;
        self
    }

    /// Gets the current byte position in the input data.
    ///
    /// Useful when debugging errors.
    pub fn buffer_position(&self) -> usize {
        // when internal state is Opened, we have actually read until '<',
        // which we don't want to show
        if let TagState::Opened = self.tag_state {
            self.buf_position - 1
        } else {
            self.buf_position
        }
    }

    /// private function to read until '<' is found
    /// return a `Text` event
    fn read_until_open<'i, 'r, B>(&mut self, buf: B) -> Result<Event<'i>>
    where
        R: BufferedInput<'i, 'r, B>,
    {
        self.tag_state = TagState::Opened;

        if self.trim_text_start {
            self.reader.skip_whitespace(&mut self.buf_position)?;
            if self.reader.skip_one(b'<', &mut self.buf_position)? {
                return self.read_event_buffered(buf);
            }
        }

        match self
            .reader
            .read_bytes_until(b'<', buf, &mut self.buf_position)
        {
            Ok(Some(bytes)) if self.trim_text_end => {
                // Skip the ending '<
                let len = bytes
                    .iter()
                    .rposition(|&b| !is_whitespace(b))
                    .map_or_else(|| bytes.len(), |p| p + 1);
                Ok(Event::Text(BytesText::from_escaped(&bytes[..len])))
            }
            Ok(Some(bytes)) => Ok(Event::Text(BytesText::from_escaped(bytes))),
            Ok(None) => Ok(Event::Eof),
            Err(e) => Err(e),
        }
    }

    /// Private function to read until `>` is found. This function expects that
    /// it was called just after encounter a `<` symbol.
    fn read_until_close<'i, 'r, B>(&mut self, buf: B) -> Result<Event<'i>>
    where
        R: BufferedInput<'i, 'r, B>,
    {
        self.tag_state = TagState::Closed;

        // need to read 1 character to decide whether pay special attention to attribute values
        let start = match self.reader.peek_one() {
            Ok(None) => return Ok(Event::Eof),
            Ok(Some(byte)) => byte,
            Err(e) => return Err(e),
        };

        match start {
            // `<!` - comment, CDATA or DOCTYPE declaration
            b'!' => match self.reader.read_bang_element(buf, &mut self.buf_position) {
                Ok(None) => Ok(Event::Eof),
                Ok(Some((bang_type, bytes))) => self.read_bang(bang_type, bytes),
                Err(e) => Err(e),
            },
            // `</` - closing tag
            b'/' => match self
                .reader
                .read_bytes_until(b'>', buf, &mut self.buf_position)
            {
                Ok(None) => Ok(Event::Eof),
                Ok(Some(bytes)) => self.read_end(bytes),
                Err(e) => Err(e),
            },
            // `<?` - processing instruction
            b'?' => match self
                .reader
                .read_bytes_until(b'>', buf, &mut self.buf_position)
            {
                Ok(None) => Ok(Event::Eof),
                Ok(Some(bytes)) => self.read_question_mark(bytes),
                Err(e) => Err(e),
            },
            // `<...` - opening or self-closed tag
            _ => match self.reader.read_element(buf, &mut self.buf_position) {
                Ok(None) => Ok(Event::Eof),
                Ok(Some(bytes)) => self.read_start(bytes),
                Err(e) => Err(e),
            },
        }
    }

    /// reads `BytesElement` starting with a `/`,
    /// if `self.check_end_names`, checks that element matches last opened element
    /// return `End` event
    fn read_end<'a, 'b>(&'a mut self, buf: &'b [u8]) -> Result<Event<'b>> {
        // XML standard permits whitespaces after the markup name in closing tags.
        // Let's strip them from the buffer before comparing tag names.
        let name = if self.trim_markup_names_in_closing_tags {
            if let Some(pos_end_name) = buf[1..].iter().rposition(|&b| !b.is_ascii_whitespace()) {
                let (name, _) = buf[1..].split_at(pos_end_name + 1);
                name
            } else {
                &buf[1..]
            }
        } else {
            &buf[1..]
        };
        if self.check_end_names {
            let mismatch_err = |expected: &[u8], found: &[u8], buf_position: &mut usize| {
                *buf_position -= buf.len();
                Err(Error::EndEventMismatch {
                    expected: from_utf8(expected).unwrap_or("").to_owned(),
                    found: from_utf8(found).unwrap_or("").to_owned(),
                })
            };
            match self.opened_starts.pop() {
                Some(start) => {
                    if name != &self.opened_buffer[start..] {
                        let expected = &self.opened_buffer[start..];
                        mismatch_err(expected, name, &mut self.buf_position)
                    } else {
                        self.opened_buffer.truncate(start);
                        Ok(Event::End(BytesEnd::borrowed(name)))
                    }
                }
                None => mismatch_err(b"", &buf[1..], &mut self.buf_position),
            }
        } else {
            Ok(Event::End(BytesEnd::borrowed(name)))
        }
    }

    /// reads `BytesElement` starting with a `!`,
    /// return `Comment`, `CData` or `DocType` event
    fn read_bang<'a, 'b>(&'a mut self, bang_type: BangType, buf: &'b [u8]) -> Result<Event<'b>> {
        let uncased_starts_with = |string: &[u8], prefix: &[u8]| {
            string.len() >= prefix.len() && string[..prefix.len()].eq_ignore_ascii_case(prefix)
        };

        let len = buf.len();
        match bang_type {
            BangType::Comment if buf.starts_with(b"!--") => {
                if self.check_comments {
                    // search if '--' not in comments
                    if let Some(p) = memchr::memchr_iter(b'-', &buf[3..len - 2])
                        .position(|p| buf[3 + p + 1] == b'-')
                    {
                        self.buf_position += len - p;
                        return Err(Error::UnexpectedToken("--".to_string()));
                    }
                }
                Ok(Event::Comment(BytesText::from_escaped(&buf[3..len - 2])))
            }
            BangType::CData if uncased_starts_with(buf, b"![CDATA[") => {
                Ok(Event::CData(BytesText::from_plain(&buf[8..])))
            }
            BangType::DocType if uncased_starts_with(buf, b"!DOCTYPE") => {
                let start = buf[8..]
                    .iter()
                    .position(|b| !is_whitespace(*b))
                    .unwrap_or_else(|| len - 8);
                debug_assert!(start < len - 8, "DocType must have a name");
                Ok(Event::DocType(BytesText::from_escaped(&buf[8 + start..])))
            }
            _ => Err(bang_type.to_err()),
        }
    }

    /// reads `BytesElement` starting with a `?`,
    /// return `Decl` or `PI` event
    #[cfg(feature = "encoding")]
    fn read_question_mark<'a, 'b>(&'a mut self, buf: &'b [u8]) -> Result<Event<'b>> {
        let len = buf.len();
        if len > 2 && buf[len - 1] == b'?' {
            if len > 5 && &buf[1..4] == b"xml" && is_whitespace(buf[4]) {
                let event = BytesDecl::from_start(BytesStart::borrowed(&buf[1..len - 1], 3));
                // Try getting encoding from the declaration event
                if let Some(enc) = event.encoder() {
                    self.encoding = enc;
                    self.is_encoding_set = true;
                }
                Ok(Event::Decl(event))
            } else {
                Ok(Event::PI(BytesText::from_escaped(&buf[1..len - 1])))
            }
        } else {
            self.buf_position -= len;
            Err(Error::UnexpectedEof("XmlDecl".to_string()))
        }
    }

    /// reads `BytesElement` starting with a `?`,
    /// return `Decl` or `PI` event
    #[cfg(not(feature = "encoding"))]
    fn read_question_mark<'a, 'b>(&'a mut self, buf: &'b [u8]) -> Result<Event<'b>> {
        let len = buf.len();
        if len > 2 && buf[len - 1] == b'?' {
            if len > 5 && &buf[1..4] == b"xml" && is_whitespace(buf[4]) {
                let event = BytesDecl::from_start(BytesStart::borrowed(&buf[1..len - 1], 3));
                Ok(Event::Decl(event))
            } else {
                Ok(Event::PI(BytesText::from_escaped(&buf[1..len - 1])))
            }
        } else {
            self.buf_position -= len;
            Err(Error::UnexpectedEof("XmlDecl".to_string()))
        }
    }

    #[inline]
    fn close_expanded_empty(&mut self) -> Result<Event<'static>> {
        self.tag_state = TagState::Closed;
        let name = self
            .opened_buffer
            .split_off(self.opened_starts.pop().unwrap());
        Ok(Event::End(BytesEnd::owned(name)))
    }

    /// reads `BytesElement` starting with any character except `/`, `!` or ``?`
    /// return `Start` or `Empty` event
    fn read_start<'a, 'b>(&'a mut self, buf: &'b [u8]) -> Result<Event<'b>> {
        // TODO: do this directly when reading bufreader ...
        let len = buf.len();
        let name_end = buf.iter().position(|&b| is_whitespace(b)).unwrap_or(len);
        if let Some(&b'/') = buf.last() {
            let end = if name_end < len { name_end } else { len - 1 };
            if self.expand_empty_elements {
                self.tag_state = TagState::Empty;
                self.opened_starts.push(self.opened_buffer.len());
                self.opened_buffer.extend(&buf[..end]);
                Ok(Event::Start(BytesStart::borrowed(&buf[..len - 1], end)))
            } else {
                Ok(Event::Empty(BytesStart::borrowed(&buf[..len - 1], end)))
            }
        } else {
            if self.check_end_names {
                self.opened_starts.push(self.opened_buffer.len());
                self.opened_buffer.extend(&buf[..name_end]);
            }
            Ok(Event::Start(BytesStart::borrowed(buf, name_end)))
        }
    }

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
    /// use fast_xml::Reader;
    /// use fast_xml::events::Event;
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
    ///     match reader.read_event(&mut buf) {
    ///         Ok(Event::Start(ref e)) => count += 1,
    ///         Ok(Event::Text(e)) => txt.push(e.unescape_and_decode(&reader).expect("Error!")),
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
    pub fn read_event<'a, 'b>(&'a mut self, buf: &'b mut Vec<u8>) -> Result<Event<'b>> {
        self.read_event_buffered(buf)
    }

    /// Read text into the given buffer, and return an event that borrows from
    /// either that buffer or from the input itself, based on the type of the
    /// reader.
    fn read_event_buffered<'i, 'r, B>(&mut self, buf: B) -> Result<Event<'i>>
    where
        R: BufferedInput<'i, 'r, B>,
    {
        let event = match self.tag_state {
            TagState::Opened => self.read_until_close(buf),
            TagState::Closed => self.read_until_open(buf),
            TagState::Empty => self.close_expanded_empty(),
            TagState::Exit => return Ok(Event::Eof),
        };
        match event {
            Err(_) | Ok(Event::Eof) => self.tag_state = TagState::Exit,
            _ => {}
        }
        event
    }

    /// Resolves a potentially qualified **event name** into (namespace name, local name).
    ///
    /// *Qualified* attribute names have the form `prefix:local-name` where the`prefix` is defined
    /// on any containing XML element via `xmlns:prefix="the:namespace:uri"`. The namespace prefix
    /// can be defined on the same element as the attribute in question.
    ///
    /// *Unqualified* event inherits the current *default namespace*.
    #[inline]
    pub fn event_namespace<'a, 'b, 'c>(
        &'a self,
        qname: &'b [u8],
        namespace_buffer: &'c [u8],
    ) -> (Option<&'c [u8]>, &'b [u8]) {
        self.ns_buffer
            .resolve_namespace(qname, namespace_buffer, true)
    }

    /// Resolves a potentially qualified **attribute name** into (namespace name, local name).
    ///
    /// *Qualified* attribute names have the form `prefix:local-name` where the`prefix` is defined
    /// on any containing XML element via `xmlns:prefix="the:namespace:uri"`. The namespace prefix
    /// can be defined on the same element as the attribute in question.
    ///
    /// *Unqualified* attribute names do *not* inherit the current *default namespace*.
    #[inline]
    pub fn attribute_namespace<'a, 'b, 'c>(
        &'a self,
        qname: &'b [u8],
        namespace_buffer: &'c [u8],
    ) -> (Option<&'c [u8]>, &'b [u8]) {
        self.ns_buffer
            .resolve_namespace(qname, namespace_buffer, false)
    }

    /// Reads the next event and resolves its namespace (if applicable).
    ///
    /// # Examples
    ///
    /// ```
    /// use std::str::from_utf8;
    /// use fast_xml::Reader;
    /// use fast_xml::events::Event;
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
    ///         Ok((ref ns, Event::Start(ref e))) => {
    ///             count += 1;
    ///             match (*ns, e.local_name()) {
    ///                 (Some(b"www.xxxx"), b"tag1") => (),
    ///                 (Some(b"www.yyyy"), b"tag2") => (),
    ///                 (ns, n) => panic!("Namespace and local name mismatch"),
    ///             }
    ///             println!("Resolved namespace: {:?}", ns.and_then(|ns| from_utf8(ns).ok()));
    ///         }
    ///         Ok((_, Event::Text(e))) => {
    ///             txt.push(e.unescape_and_decode(&reader).expect("Error!"))
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
    pub fn read_namespaced_event<'a, 'b, 'c>(
        &'a mut self,
        buf: &'b mut Vec<u8>,
        namespace_buffer: &'c mut Vec<u8>,
    ) -> Result<(Option<&'c [u8]>, Event<'b>)> {
        self.ns_buffer.pop_empty_namespaces(namespace_buffer);
        match self.read_event(buf) {
            Ok(Event::Eof) => Ok((None, Event::Eof)),
            Ok(Event::Start(e)) => {
                self.ns_buffer.push_new_namespaces(&e, namespace_buffer);
                Ok((
                    self.ns_buffer
                        .find_namespace_value(e.name(), &**namespace_buffer),
                    Event::Start(e),
                ))
            }
            Ok(Event::Empty(e)) => {
                // For empty elements we need to 'artificially' keep the namespace scope on the
                // stack until the next `next()` call occurs.
                // Otherwise the caller has no chance to use `resolve` in the context of the
                // namespace declarations that are 'in scope' for the empty element alone.
                // Ex: <img rdf:nodeID="abc" xmlns:rdf="urn:the-rdf-uri" />
                self.ns_buffer.push_new_namespaces(&e, namespace_buffer);
                // notify next `read_namespaced_event()` invocation that it needs to pop this
                // namespace scope
                self.ns_buffer.pending_pop = true;
                Ok((
                    self.ns_buffer
                        .find_namespace_value(e.name(), &**namespace_buffer),
                    Event::Empty(e),
                ))
            }
            Ok(Event::End(e)) => {
                // notify next `read_namespaced_event()` invocation that it needs to pop this
                // namespace scope
                self.ns_buffer.pending_pop = true;
                Ok((
                    self.ns_buffer
                        .find_namespace_value(e.name(), &**namespace_buffer),
                    Event::End(e),
                ))
            }
            Ok(e) => Ok((None, e)),
            Err(e) => Err(e),
        }
    }

    /// Returns the `Reader`s encoding.
    ///
    /// The used encoding may change after parsing the XML declaration.
    ///
    /// This encoding will be used by [`decode`].
    ///
    /// [`decode`]: #method.decode
    #[cfg(feature = "encoding")]
    pub fn encoding(&self) -> &'static Encoding {
        self.encoding
    }

    /// Decodes a slice using the encoding specified in the XML declaration.
    ///
    /// Decode `bytes` with BOM sniffing and with malformed sequences replaced with the
    /// `U+FFFD REPLACEMENT CHARACTER`.
    ///
    /// If no encoding is specified, defaults to UTF-8.
    #[inline]
    #[cfg(feature = "encoding")]
    pub fn decode<'b, 'c>(&'b self, bytes: &'c [u8]) -> Cow<'c, str> {
        self.encoding.decode(bytes).0
    }

    /// Decodes a UTF8 slice without BOM (Byte order mark) regardless of XML declaration.
    ///
    /// Decode `bytes` without BOM and with malformed sequences replaced with the
    /// `U+FFFD REPLACEMENT CHARACTER`.
    ///
    /// # Note
    ///
    /// If you instead want to use XML declared encoding, use the `encoding` feature
    #[inline]
    #[cfg(not(feature = "encoding"))]
    pub fn decode_without_bom<'c>(&self, bytes: &'c [u8]) -> Result<&'c str> {
        if bytes.starts_with(b"\xEF\xBB\xBF") {
            from_utf8(&bytes[3..]).map_err(Error::Utf8)
        } else {
            from_utf8(bytes).map_err(Error::Utf8)
        }
    }

    /// Decodes a slice using without BOM (Byte order mark) the encoding specified in the XML declaration.
    ///
    /// Decode `bytes` without BOM and with malformed sequences replaced with the
    /// `U+FFFD REPLACEMENT CHARACTER`.
    ///
    /// If no encoding is specified, defaults to UTF-8.
    #[inline]
    #[cfg(feature = "encoding")]
    pub fn decode_without_bom<'b, 'c>(&'b mut self, mut bytes: &'c [u8]) -> Cow<'c, str> {
        if self.is_encoding_set {
            return self.encoding.decode_with_bom_removal(bytes).0;
        }
        if bytes.starts_with(b"\xEF\xBB\xBF") {
            self.is_encoding_set = true;
            bytes = &bytes[3..];
        } else if bytes.starts_with(b"\xFF\xFE") {
            self.is_encoding_set = true;
            self.encoding = UTF_16LE;
            bytes = &bytes[2..];
        } else if bytes.starts_with(b"\xFE\xFF") {
            self.is_encoding_set = true;
            self.encoding = UTF_16BE;
            bytes = &bytes[3..];
        };
        self.encoding.decode_without_bom_handling(bytes).0
    }

    /// Decodes a UTF8 slice regardless of XML declaration.
    ///
    /// Decode `bytes` with BOM sniffing and with malformed sequences replaced with the
    /// `U+FFFD REPLACEMENT CHARACTER`.
    ///
    /// # Note
    ///
    /// If you instead want to use XML declared encoding, use the `encoding` feature
    #[inline]
    #[cfg(not(feature = "encoding"))]
    pub fn decode<'c>(&self, bytes: &'c [u8]) -> Result<&'c str> {
        from_utf8(bytes).map_err(Error::Utf8)
    }

    /// Get utf8 decoder
    #[cfg(feature = "encoding")]
    pub fn decoder(&self) -> Decoder {
        Decoder {
            encoding: self.encoding,
        }
    }

    /// Get utf8 decoder
    #[cfg(not(feature = "encoding"))]
    pub fn decoder(&self) -> Decoder {
        Decoder
    }

    /// Reads until end element is found
    ///
    /// Manages nested cases where parent and child elements have the same name
    pub fn read_to_end<K: AsRef<[u8]>>(&mut self, end: K, buf: &mut Vec<u8>) -> Result<()> {
        let mut depth = 0;
        let end = end.as_ref();
        loop {
            match self.read_event(buf) {
                Ok(Event::End(ref e)) if e.name() == end => {
                    if depth == 0 {
                        return Ok(());
                    }
                    depth -= 1;
                }
                Ok(Event::Start(ref e)) if e.name() == end => depth += 1,
                Err(e) => return Err(e),
                Ok(Event::Eof) => {
                    return Err(Error::UnexpectedEof(format!("</{:?}>", from_utf8(end))));
                }
                _ => (),
            }
            buf.clear();
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
    /// use fast_xml::Reader;
    /// use fast_xml::events::Event;
    ///
    /// let mut xml = Reader::from_reader(b"
    ///     <a>&lt;b&gt;</a>
    ///     <a></a>
    /// " as &[u8]);
    /// xml.trim_text(true);
    ///
    /// let expected = ["<b>", ""];
    /// for &content in expected.iter() {
    ///     match xml.read_event(&mut Vec::new()) {
    ///         Ok(Event::Start(ref e)) => {
    ///             assert_eq!(&xml.read_text(e.name(), &mut Vec::new()).unwrap(), content);
    ///         },
    ///         e => panic!("Expecting Start event, found {:?}", e),
    ///     }
    /// }
    /// ```
    ///
    /// [`Text`]: events/enum.Event.html#variant.Text
    /// [`End`]: events/enum.Event.html#variant.End
    pub fn read_text<K: AsRef<[u8]>>(&mut self, end: K, buf: &mut Vec<u8>) -> Result<String> {
        let s = match self.read_event(buf) {
            Ok(Event::Text(e)) => e.unescape_and_decode(self),
            Ok(Event::End(ref e)) if e.name() == end.as_ref() => return Ok("".to_string()),
            Err(e) => return Err(e),
            Ok(Event::Eof) => return Err(Error::UnexpectedEof("Text".to_string())),
            _ => return Err(Error::TextNotFound),
        };
        self.read_to_end(end, buf)?;
        s
    }

    /// Consumes `Reader` returning the underlying reader
    ///
    /// Can be used to compute line and column of a parsing error position
    ///
    /// # Examples
    ///
    /// ```
    /// # use pretty_assertions::assert_eq;
    /// use std::{str, io::Cursor};
    /// use fast_xml::Reader;
    /// use fast_xml::events::Event;
    ///
    /// let xml = r#"<tag1 att1 = "test">
    ///                 <tag2><!--Test comment-->Test</tag2>
    ///                 <tag3>Test 2</tag3>
    ///             </tag1>"#;
    /// let mut reader = Reader::from_reader(Cursor::new(xml.as_bytes()));
    /// let mut buf = Vec::new();
    ///
    /// fn into_line_and_column(reader: Reader<Cursor<&[u8]>>) -> (usize, usize) {
    ///     let end_pos = reader.buffer_position();
    ///     let mut cursor = reader.into_underlying_reader();
    ///     let s = String::from_utf8(cursor.into_inner()[0..end_pos].to_owned())
    ///         .expect("can't make a string");
    ///     let mut line = 1;
    ///     let mut column = 0;
    ///     for c in s.chars() {
    ///         if c == '\n' {
    ///             line += 1;
    ///             column = 0;
    ///         } else {
    ///             column += 1;
    ///         }
    ///     }
    ///     (line, column)
    /// }
    ///
    /// loop {
    ///     match reader.read_event(&mut buf) {
    ///         Ok(Event::Start(ref e)) => match e.name() {
    ///             b"tag1" | b"tag2" => (),
    ///             tag => {
    ///                 assert_eq!(b"tag3", tag);
    ///                 assert_eq!((3, 22), into_line_and_column(reader));
    ///                 break;
    ///             }
    ///         },
    ///         Ok(Event::Eof) => unreachable!(),
    ///         _ => (),
    ///     }
    ///     buf.clear();
    /// }
    /// ```
    pub fn into_underlying_reader(self) -> R {
        self.reader
    }
}

impl Reader<BufReader<File>> {
    /// Creates an XML reader from a file path.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Reader<BufReader<File>>> {
        let file = File::open(path).map_err(Error::Io)?;
        let reader = BufReader::new(file);
        Ok(Reader::from_reader(reader))
    }
}

impl<'a> Reader<&'a [u8]> {
    /// Creates an XML reader from a string slice.
    pub fn from_str(s: &'a str) -> Reader<&'a [u8]> {
        Reader::from_reader(s.as_bytes())
    }

    /// Creates an XML reader from a slice of bytes.
    pub fn from_bytes(s: &'a [u8]) -> Reader<&'a [u8]> {
        Reader::from_reader(s)
    }

    /// Read an event that borrows from the input rather than a buffer.
    #[inline]
    pub fn read_event_unbuffered(&mut self) -> Result<Event<'a>> {
        self.read_event_buffered(())
    }

    /// Reads until end element is found
    ///
    /// Manages nested cases where parent and child elements have the same name
    pub fn read_to_end_unbuffered<K: AsRef<[u8]>>(&mut self, end: K) -> Result<()> {
        let mut depth = 0;
        let end = end.as_ref();
        loop {
            match self.read_event_unbuffered() {
                Ok(Event::End(ref e)) if e.name() == end => {
                    if depth == 0 {
                        return Ok(());
                    }
                    depth -= 1;
                }
                Ok(Event::Start(ref e)) if e.name() == end => depth += 1,
                Err(e) => return Err(e),
                Ok(Event::Eof) => {
                    return Err(Error::UnexpectedEof(format!("</{:?}>", from_utf8(end))));
                }
                _ => (),
            }
        }
    }
}

trait BufferedInput<'r, 'i, B>
where
    Self: 'i,
{
    /// Read input until `byte` is found or end of input is reached.
    ///
    /// Returns a slice of data read up to `byte`, which does not include into result.
    /// If input (`Self`) is exhausted, returns `None`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut position = 0;
    /// let mut input = b"abc*def".as_ref();
    /// //                    ^= 4
    ///
    /// assert_eq!(
    ///     input.read_bytes_until(b'*', (), &mut position).unwrap(),
    ///     Some(b"abc".as_ref())
    /// );
    /// assert_eq!(position, 4); // position after the symbol matched
    /// ```
    ///
    /// # Parameters
    /// - `byte`: Byte for search
    /// - `buf`: Buffer that could be filled from an input (`Self`) and
    ///   from which [events] could borrow their data
    /// - `position`: Will be increased by amount of bytes consumed
    ///
    /// [events]: crate::events::Event
    fn read_bytes_until(
        &mut self,
        byte: u8,
        buf: B,
        position: &mut usize,
    ) -> Result<Option<&'r [u8]>>;

    /// Read input until comment, CDATA or processing instruction is finished.
    ///
    /// This method expect that `<` already was read.
    ///
    /// Returns a slice of data read up to end of comment, CDATA or processing
    /// instruction (`>`), which does not include into result.
    ///
    /// If input (`Self`) is exhausted and nothing was read, returns `None`.
    ///
    /// # Parameters
    /// - `buf`: Buffer that could be filled from an input (`Self`) and
    ///   from which [events] could borrow their data
    /// - `position`: Will be increased by amount of bytes consumed
    ///
    /// [events]: crate::events::Event
    fn read_bang_element(
        &mut self,
        buf: B,
        position: &mut usize,
    ) -> Result<Option<(BangType, &'r [u8])>>;

    /// Read input until XML element is closed by approaching a `>` symbol.
    /// Returns `Some(buffer)` that contains a data between `<` and `>` or
    /// `None` if end-of-input was reached and nothing was read.
    ///
    /// Derived from `read_until`, but modified to handle XML attributes
    /// using a minimal state machine.
    ///
    /// Attribute values are [defined] as follows:
    /// ```plain
    /// AttValue := '"' (([^<&"]) | Reference)* '"'
    ///           | "'" (([^<&']) | Reference)* "'"
    /// ```
    /// (`Reference` is something like `&quot;`, but we don't care about
    /// escaped characters at this level)
    ///
    /// # Parameters
    /// - `buf`: Buffer that could be filled from an input (`Self`) and
    ///   from which [events] could borrow their data
    /// - `position`: Will be increased by amount of bytes consumed
    ///
    /// [defined]: https://www.w3.org/TR/xml11/#NT-AttValue
    /// [events]: crate::events::Event
    fn read_element(&mut self, buf: B, position: &mut usize) -> Result<Option<&'r [u8]>>;

    fn skip_whitespace(&mut self, position: &mut usize) -> Result<()>;

    fn skip_one(&mut self, byte: u8, position: &mut usize) -> Result<bool>;

    fn peek_one(&mut self) -> Result<Option<u8>>;

    fn input_borrowed(event: Event<'r>) -> Event<'i>;
}

/// Implementation of BufferedInput for any BufRead reader using a user-given
/// Vec<u8> as buffer that will be borrowed by events.
impl<'b, 'i, R: BufRead + 'i> BufferedInput<'b, 'i, &'b mut Vec<u8>> for R {
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
        #[derive(Clone, Copy)]
        enum State {
            /// The initial state (inside element, but outside of attribute value)
            Elem,
            /// Inside a single-quoted attribute value
            SingleQ,
            /// Inside a double-quoted attribute value
            DoubleQ,
        }
        impl State {
            fn find<'b>(&mut self, end_byte: u8, bytes: &'b [u8]) -> Option<(&'b [u8], usize)> {
                for i in memchr::memchr3_iter(end_byte, b'\'', b'"', bytes) {
                    *self = match (*self, bytes[i]) {
                        (State::Elem, b) if b == end_byte => {
                            // only allowed to match `end_byte` while we are in state `Elem`
                            return Some((&bytes[..i], i + 1));
                        }
                        (State::Elem, b'\'') => State::SingleQ,
                        (State::Elem, b'\"') => State::DoubleQ,

                        // the only end_byte that gets us out if the same character
                        (State::SingleQ, b'\'') | (State::DoubleQ, b'\"') => State::Elem,

                        // all other bytes: no state change
                        _ => *self,
                    };
                }
                None
            }
        }
        let mut state = State::Elem;
        let mut read = 0;
        let mut done = false;

        let start = buf.len();
        while !done {
            let used = {
                let available = match self.fill_buf() {
                    Ok(n) if n.is_empty() => {
                        if read == 0 {
                            return Ok(None);
                        } else {
                            return Ok(Some(&buf[start..]));
                        }
                    }
                    Ok(n) => n,
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => {
                        *position += read;
                        return Err(Error::Io(e));
                    }
                };

                if let Some((consumed, used)) = state.find(b'>', available) {
                    done = true;
                    buf.extend_from_slice(consumed);
                    used
                } else {
                    buf.extend_from_slice(available);
                    available.len()
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

    fn input_borrowed(event: Event<'b>) -> Event<'i> {
        event.into_owned()
    }
}

/// Implementation of BufferedInput for any BufRead reader using a user-given
/// Vec<u8> as buffer that will be borrowed by events.
impl<'a> BufferedInput<'a, 'a, ()> for &'a [u8] {
    fn read_bytes_until(
        &mut self,
        byte: u8,
        _buf: (),
        position: &mut usize,
    ) -> Result<Option<&'a [u8]>> {
        if self.is_empty() {
            return Ok(None);
        }

        Ok(Some(if let Some(i) = memchr::memchr(byte, self) {
            *position += i + 1;
            let bytes = &self[..i];
            *self = &self[i + 1..];
            bytes
        } else {
            *position += self.len();
            let bytes = &self[..];
            *self = &[];
            bytes
        }))
    }

    fn read_bang_element(
        &mut self,
        _buf: (),
        position: &mut usize,
    ) -> Result<Option<(BangType, &'a [u8])>> {
        // Peeked one bang ('!') before being called, so it's guaranteed to
        // start with it.
        debug_assert_eq!(self[0], b'!');

        let bang_type = BangType::new(self[1..].first().copied())?;

        if let Some((bytes, i)) = bang_type.parse(self, 0) {
            *position += i;
            *self = &self[i..];
            return Ok(Some((bang_type, bytes)));
        }

        // Note: Do not update position, so the error points to
        // somewhere sane rather than at the EOF
        Err(bang_type.to_err())
    }

    fn read_element(&mut self, _buf: (), position: &mut usize) -> Result<Option<&'a [u8]>> {
        if self.is_empty() {
            return Ok(None);
        }

        #[derive(Clone, Copy)]
        enum State {
            /// The initial state (inside element, but outside of attribute value)
            Elem,
            /// Inside a single-quoted attribute value
            SingleQ,
            /// Inside a double-quoted attribute value
            DoubleQ,
        }
        let mut state = State::Elem;

        let end_byte = b'>';

        for i in memchr::memchr3_iter(end_byte, b'\'', b'"', self) {
            state = match (state, self[i]) {
                (State::Elem, b) if b == end_byte => {
                    // only allowed to match `end_byte` while we are in state `Elem`
                    *position += i + 1;
                    let bytes = &self[..i];
                    // Skip the '>' too.
                    *self = &self[i + 1..];
                    return Ok(Some(bytes));
                }
                (State::Elem, b'\'') => State::SingleQ,
                (State::Elem, b'\"') => State::DoubleQ,

                // the only end_byte that gets us out if the same character
                (State::SingleQ, b'\'') | (State::DoubleQ, b'\"') => State::Elem,

                // all other bytes: no state change
                _ => state,
            };
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
        *self = &self[whitespaces..];
        Ok(())
    }

    fn skip_one(&mut self, byte: u8, position: &mut usize) -> Result<bool> {
        if self.first() == Some(&byte) {
            *self = &self[1..];
            *position += 1;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn peek_one(&mut self) -> Result<Option<u8>> {
        Ok(self.first().copied())
    }

    fn input_borrowed(event: Event<'a>) -> Event<'a> {
        return event;
    }
}

/// Possible elements started with `<!`
#[derive(Debug, PartialEq)]
enum BangType {
    /// <![CDATA[...]]>
    CData,
    /// <!--...-->
    Comment,
    /// <!DOCTYPE...>
    DocType,
}
impl BangType {
    #[inline(always)]
    fn new(byte: Option<u8>) -> Result<Self> {
        Ok(match byte {
            Some(b'[') => Self::CData,
            Some(b'-') => Self::Comment,
            Some(b'D') | Some(b'd') => Self::DocType,
            Some(b) => return Err(Error::UnexpectedBang(b)),
            None => return Err(Error::UnexpectedEof("Bang".to_string())),
        })
    }

    /// If element is finished, returns its content up to `>` symbol and
    /// an index of this symbol, otherwise returns `None`
    #[inline(always)]
    fn parse<'b>(&self, chunk: &'b [u8], offset: usize) -> Option<(&'b [u8], usize)> {
        for i in memchr::memchr_iter(b'>', chunk) {
            match self {
                // Need to read at least 6 symbols (`!---->`) for properly finished comment
                // <!----> - XML comment
                //  012345 - i
                Self::Comment => {
                    if offset + i > 4 && chunk[..i].ends_with(b"--") {
                        // We cannot strip last `--` from the buffer because we need it in case of
                        // check_comments enabled option. XML standard requires that comment
                        // will not end with `--->` sequence because this is a special case of
                        // `--` in the comment (https://www.w3.org/TR/xml11/#sec-comments)
                        return Some((&chunk[..i], i + 1)); // +1 for `>`
                    }
                }
                Self::CData => {
                    if chunk[..i].ends_with(b"]]") {
                        return Some((&chunk[..i - 2], i + 1)); // +1 for `>`
                    }
                }
                Self::DocType => {
                    let content = &chunk[..i];
                    let balance = memchr::memchr2_iter(b'<', b'>', content)
                        .map(|p| if content[p] == b'<' { 1i32 } else { -1 })
                        .sum::<i32>();
                    if balance == 0 {
                        return Some((content, i + 1)); // +1 for `>`
                    }
                }
            }
        }
        None
    }
    #[inline]
    fn to_err(self) -> Error {
        let bang_str = match self {
            Self::CData => "CData",
            Self::Comment => "Comment",
            Self::DocType => "DOCTYPE",
        };
        Error::UnexpectedEof(bang_str.to_string())
    }
}

/// A function to check whether the byte is a whitespace (blank, new line, carriage return or tab)
#[inline]
pub(crate) fn is_whitespace(b: u8) -> bool {
    match b {
        b' ' | b'\r' | b'\n' | b'\t' => true,
        _ => false,
    }
}

/// A namespace declaration. Can either bind a namespace to a prefix or define the current default
/// namespace.
#[derive(Debug, Clone)]
struct Namespace {
    /// Index of the namespace in the buffer
    start: usize,
    /// Length of the prefix
    /// * if bigger than start, then binds this namespace to the corresponding slice.
    /// * else defines the current default namespace.
    prefix_len: usize,
    /// The namespace name (the URI) of this namespace declaration.
    ///
    /// The XML standard specifies that an empty namespace value 'removes' a namespace declaration
    /// for the extent of its scope. For prefix declarations that's not very interesting, but it is
    /// vital for default namespace declarations. With `xmlns=""` you can revert back to the default
    /// behaviour of leaving unqualified element names unqualified.
    value_len: usize,
    /// Level of nesting at which this namespace was declared. The declaring element is included,
    /// i.e., a declaration on the document root has `level = 1`.
    /// This is used to pop the namespace when the element gets closed.
    level: i32,
}

impl Namespace {
    /// Gets the value slice out of namespace buffer
    ///
    /// Returns `None` if `value_len == 0`
    #[inline]
    fn opt_value<'a, 'b>(&'a self, ns_buffer: &'b [u8]) -> Option<&'b [u8]> {
        if self.value_len == 0 {
            None
        } else {
            let start = self.start + self.prefix_len;
            Some(&ns_buffer[start..start + self.value_len])
        }
    }

    /// Check if the namespace matches the potentially qualified name
    #[inline]
    fn is_match(&self, ns_buffer: &[u8], qname: &[u8]) -> bool {
        if self.prefix_len == 0 {
            !qname.contains(&b':')
        } else {
            qname.get(self.prefix_len).map_or(false, |n| *n == b':')
                && qname.starts_with(&ns_buffer[self.start..self.start + self.prefix_len])
        }
    }
}

/// A namespace management buffer.
///
/// Holds all internal logic to push/pop namespaces with their levels.
#[derive(Debug, Default, Clone)]
struct NamespaceBufferIndex {
    /// a buffer of namespace ranges
    slices: Vec<Namespace>,
    /// The number of open tags at the moment. We need to keep track of this to know which namespace
    /// declarations to remove when we encounter an `End` event.
    nesting_level: i32,
    /// For `Empty` events keep the 'scope' of the element on the stack artificially. That way, the
    /// consumer has a chance to use `resolve` in the context of the empty element. We perform the
    /// pop as the first operation in the next `next()` call.
    pending_pop: bool,
}

impl NamespaceBufferIndex {
    #[inline]
    fn find_namespace_value<'a, 'b, 'c>(
        &'a self,
        element_name: &'b [u8],
        buffer: &'c [u8],
    ) -> Option<&'c [u8]> {
        self.slices
            .iter()
            .rfind(|n| n.is_match(buffer, element_name))
            .and_then(|n| n.opt_value(buffer))
    }

    fn pop_empty_namespaces(&mut self, buffer: &mut Vec<u8>) {
        if !self.pending_pop {
            return;
        }
        self.pending_pop = false;
        self.nesting_level -= 1;
        let current_level = self.nesting_level;
        // from the back (most deeply nested scope), look for the first scope that is still valid
        match self.slices.iter().rposition(|n| n.level <= current_level) {
            // none of the namespaces are valid, remove all of them
            None => {
                buffer.clear();
                self.slices.clear();
            }
            // drop all namespaces past the last valid namespace
            Some(last_valid_pos) => {
                if let Some(len) = self.slices.get(last_valid_pos + 1).map(|n| n.start) {
                    buffer.truncate(len);
                    self.slices.truncate(last_valid_pos + 1);
                }
            }
        }
    }

    fn push_new_namespaces(&mut self, e: &BytesStart, buffer: &mut Vec<u8>) {
        self.nesting_level += 1;
        let level = self.nesting_level;
        // adds new namespaces for attributes starting with 'xmlns:' and for the 'xmlns'
        // (default namespace) attribute.
        for a in e.attributes().with_checks(false) {
            if let Ok(Attribute { key: k, value: v }) = a {
                if k.starts_with(b"xmlns") {
                    match k.get(5) {
                        None => {
                            let start = buffer.len();
                            buffer.extend_from_slice(&*v);
                            self.slices.push(Namespace {
                                start,
                                prefix_len: 0,
                                value_len: v.len(),
                                level,
                            });
                        }
                        Some(&b':') => {
                            let start = buffer.len();
                            buffer.extend_from_slice(&k[6..]);
                            buffer.extend_from_slice(&*v);
                            self.slices.push(Namespace {
                                start,
                                prefix_len: k.len() - 6,
                                value_len: v.len(),
                                level,
                            });
                        }
                        _ => break,
                    }
                }
            } else {
                break;
            }
        }
    }

    /// Resolves a potentially qualified **attribute name** into (namespace name, local name).
    ///
    /// *Qualified* attribute names have the form `prefix:local-name` where the`prefix` is defined
    /// on any containing XML element via `xmlns:prefix="the:namespace:uri"`. The namespace prefix
    /// can be defined on the same element as the attribute in question.
    ///
    /// *Unqualified* attribute names do *not* inherit the current *default namespace*.
    #[inline]
    fn resolve_namespace<'a, 'b, 'c>(
        &'a self,
        qname: &'b [u8],
        buffer: &'c [u8],
        use_default: bool,
    ) -> (Option<&'c [u8]>, &'b [u8]) {
        self.slices
            .iter()
            .rfind(|n| n.is_match(buffer, qname))
            .map_or((None, qname), |n| {
                let len = n.prefix_len;
                if len > 0 {
                    (n.opt_value(buffer), &qname[len + 1..])
                } else if use_default {
                    (n.opt_value(buffer), qname)
                } else {
                    (None, qname)
                }
            })
    }
}

/// Utf8 Decoder
#[cfg(not(feature = "encoding"))]
#[derive(Clone, Copy, Debug)]
pub struct Decoder;

/// Utf8 Decoder
#[cfg(feature = "encoding")]
#[derive(Clone, Copy, Debug)]
pub struct Decoder {
    encoding: &'static Encoding,
}

impl Decoder {
    #[cfg(not(feature = "encoding"))]
    pub fn decode<'c>(&self, bytes: &'c [u8]) -> Result<&'c str> {
        from_utf8(bytes).map_err(Error::Utf8)
    }

    #[cfg(not(feature = "encoding"))]
    pub fn decode_owned<'c>(&self, bytes: Vec<u8>) -> Result<String> {
        String::from_utf8(bytes).map_err(|e| Error::Utf8(e.utf8_error()))
    }

    #[cfg(feature = "encoding")]
    pub fn decode<'c>(&self, bytes: &'c [u8]) -> Cow<'c, str> {
        self.encoding.decode(bytes).0
    }
}

#[cfg(test)]
mod test {
    macro_rules! check {
        ($buf:expr) => {
            mod read_bytes_until {
                use crate::reader::BufferedInput;
                // Use Bytes for printing bytes as strings for ASCII range
                use crate::utils::Bytes;
                use pretty_assertions::assert_eq;

                /// Checks that search in the empty buffer returns `None`
                #[test]
                fn empty() {
                    let buf = $buf;
                    let mut position = 0;
                    let mut input = b"".as_ref();
                    //                ^= 0

                    assert_eq!(
                        input
                            .read_bytes_until(b'*', buf, &mut position)
                            .unwrap()
                            .map(Bytes),
                        None
                    );
                    assert_eq!(position, 0);
                }

                /// Checks that search in the buffer non-existent value returns entire buffer
                /// as a result and set `position` to `len()`
                #[test]
                fn non_existent() {
                    let buf = $buf;
                    let mut position = 0;
                    let mut input = b"abcdef".as_ref();
                    //                      ^= 6

                    assert_eq!(
                        input
                            .read_bytes_until(b'*', buf, &mut position)
                            .unwrap()
                            .map(Bytes),
                        Some(Bytes(b"abcdef"))
                    );
                    assert_eq!(position, 6);
                }

                /// Checks that search in the buffer an element that is located in the front of
                /// buffer returns empty slice as a result and set `position` to one symbol
                /// after match (`1`)
                #[test]
                fn at_the_start() {
                    let buf = $buf;
                    let mut position = 0;
                    let mut input = b"*abcdef".as_ref();
                    //                 ^= 1

                    assert_eq!(
                        input
                            .read_bytes_until(b'*', buf, &mut position)
                            .unwrap()
                            .map(Bytes),
                        Some(Bytes(b""))
                    );
                    assert_eq!(position, 1); // position after the symbol matched
                }

                /// Checks that search in the buffer an element that is located in the middle of
                /// buffer returns slice before that symbol as a result and set `position` to one
                /// symbol after match
                #[test]
                fn inside() {
                    let buf = $buf;
                    let mut position = 0;
                    let mut input = b"abc*def".as_ref();
                    //                    ^= 4

                    assert_eq!(
                        input
                            .read_bytes_until(b'*', buf, &mut position)
                            .unwrap()
                            .map(Bytes),
                        Some(Bytes(b"abc"))
                    );
                    assert_eq!(position, 4); // position after the symbol matched
                }

                /// Checks that search in the buffer an element that is located in the end of
                /// buffer returns slice before that symbol as a result and set `position` to one
                /// symbol after match (`len()`)
                #[test]
                fn in_the_end() {
                    let buf = $buf;
                    let mut position = 0;
                    let mut input = b"abcdef*".as_ref();
                    //                       ^= 7

                    assert_eq!(
                        input
                            .read_bytes_until(b'*', buf, &mut position)
                            .unwrap()
                            .map(Bytes),
                        Some(Bytes(b"abcdef"))
                    );
                    assert_eq!(position, 7); // position after the symbol matched
                }
            }

            mod read_bang_element {
                /// Checks that reading CDATA content works correctly
                mod cdata {
                    use crate::errors::Error;
                    use crate::reader::{BangType, BufferedInput};
                    use crate::utils::Bytes;
                    use pretty_assertions::assert_eq;

                    /// Checks that if input begins like CDATA element, but CDATA start sequence
                    /// is not finished, parsing ends with an error
                    #[test]
                    #[ignore = "start CDATA sequence fully checked outside of `read_bang_element`"]
                    fn not_properly_start() {
                        let buf = $buf;
                        let mut position = 0;
                        let mut input = b"![]]>other content".as_ref();
                        //                ^= 0

                        match input.read_bang_element(buf, &mut position) {
                            Err(Error::UnexpectedEof(s)) if s == "CData" => {}
                            x => assert!(
                                false,
                                r#"Expected `UnexpectedEof("CData")`, but result is: {:?}"#,
                                x
                            ),
                        }
                        assert_eq!(position, 0);
                    }

                    /// Checks that if CDATA startup sequence was matched, but an end sequence
                    /// is not found, parsing ends with an error
                    #[test]
                    fn not_closed() {
                        let buf = $buf;
                        let mut position = 0;
                        let mut input = b"![CDATA[other content".as_ref();
                        //                ^= 0

                        match input.read_bang_element(buf, &mut position) {
                            Err(Error::UnexpectedEof(s)) if s == "CData" => {}
                            x => assert!(
                                false,
                                r#"Expected `UnexpectedEof("CData")`, but result is: {:?}"#,
                                x
                            ),
                        }
                        assert_eq!(position, 0);
                    }

                    /// Checks that CDATA element without content inside parsed successfully
                    #[test]
                    fn empty() {
                        let buf = $buf;
                        let mut position = 0;
                        let mut input = b"![CDATA[]]>other content".as_ref();
                        //                           ^= 11

                        assert_eq!(
                            input
                                .read_bang_element(buf, &mut position)
                                .unwrap()
                                .map(|(ty, data)| (ty, Bytes(data))),
                            Some((BangType::CData, Bytes(b"![CDATA[")))
                        );
                        assert_eq!(position, 11);
                    }

                    /// Checks that CDATA element with content parsed successfully.
                    /// Additionally checks that sequences inside CDATA that may look like
                    /// a CDATA end sequence do not interrupt CDATA parsing
                    #[test]
                    fn with_content() {
                        let buf = $buf;
                        let mut position = 0;
                        let mut input = b"![CDATA[cdata]] ]>content]]>other content]]>".as_ref();
                        //                                            ^= 28

                        assert_eq!(
                            input
                                .read_bang_element(buf, &mut position)
                                .unwrap()
                                .map(|(ty, data)| (ty, Bytes(data))),
                            Some((BangType::CData, Bytes(b"![CDATA[cdata]] ]>content")))
                        );
                        assert_eq!(position, 28);
                    }
                }

                /// Checks that reading XML comments works correctly. According to the [specification],
                /// comment data can contain any sequence except `--`:
                ///
                /// ```peg
                /// comment = '<--' (!'--' char)* '-->';
                /// char = [#x1-#x2C]
                ///      / [#x2E-#xD7FF]
                ///      / [#xE000-#xFFFD]
                ///      / [#x10000-#x10FFFF]
                /// ```
                ///
                /// The presence of this limitation, however, is simply a poorly designed specification
                /// (maybe for purpose of building of LL(1) XML parser) and quick-xml does not check for
                /// presence of these sequences by default. This tests allow such content.
                ///
                /// [specification]: https://www.w3.org/TR/xml11/#dt-comment
                mod comment {
                    use crate::errors::Error;
                    use crate::reader::{BangType, BufferedInput};
                    use crate::utils::Bytes;
                    use pretty_assertions::assert_eq;

                    #[test]
                    #[ignore = "start comment sequence fully checked outside of `read_bang_element`"]
                    fn not_properly_start() {
                        let buf = $buf;
                        let mut position = 0;
                        let mut input = b"!- -->other content".as_ref();
                        //                ^= 0

                        match input.read_bang_element(buf, &mut position) {
                            Err(Error::UnexpectedEof(s)) if s == "Comment" => {}
                            x => assert!(
                                false,
                                r#"Expected `UnexpectedEof("Comment")`, but result is: {:?}"#,
                                x
                            ),
                        }
                        assert_eq!(position, 0);
                    }

                    #[test]
                    fn not_properly_end() {
                        let buf = $buf;
                        let mut position = 0;
                        let mut input = b"!->other content".as_ref();
                        //                ^= 0

                        match input.read_bang_element(buf, &mut position) {
                            Err(Error::UnexpectedEof(s)) if s == "Comment" => {}
                            x => assert!(
                                false,
                                r#"Expected `UnexpectedEof("Comment")`, but result is: {:?}"#,
                                x
                            ),
                        }
                        assert_eq!(position, 0);
                    }

                    #[test]
                    fn not_closed1() {
                        let buf = $buf;
                        let mut position = 0;
                        let mut input = b"!--other content".as_ref();
                        //                ^= 0

                        match input.read_bang_element(buf, &mut position) {
                            Err(Error::UnexpectedEof(s)) if s == "Comment" => {}
                            x => assert!(
                                false,
                                r#"Expected `UnexpectedEof("Comment")`, but result is: {:?}"#,
                                x
                            ),
                        }
                        assert_eq!(position, 0);
                    }

                    #[test]
                    fn not_closed2() {
                        let buf = $buf;
                        let mut position = 0;
                        let mut input = b"!-->other content".as_ref();
                        //                ^= 0

                        match input.read_bang_element(buf, &mut position) {
                            Err(Error::UnexpectedEof(s)) if s == "Comment" => {}
                            x => assert!(
                                false,
                                r#"Expected `UnexpectedEof("Comment")`, but result is: {:?}"#,
                                x
                            ),
                        }
                        assert_eq!(position, 0);
                    }

                    #[test]
                    fn not_closed3() {
                        let buf = $buf;
                        let mut position = 0;
                        let mut input = b"!--->other content".as_ref();
                        //                ^= 0

                        match input.read_bang_element(buf, &mut position) {
                            Err(Error::UnexpectedEof(s)) if s == "Comment" => {}
                            x => assert!(
                                false,
                                r#"Expected `UnexpectedEof("Comment")`, but result is: {:?}"#,
                                x
                            ),
                        }
                        assert_eq!(position, 0);
                    }

                    #[test]
                    fn empty() {
                        let buf = $buf;
                        let mut position = 0;
                        let mut input = b"!---->other content".as_ref();
                        //                      ^= 6

                        assert_eq!(
                            input
                                .read_bang_element(buf, &mut position)
                                .unwrap()
                                .map(|(ty, data)| (ty, Bytes(data))),
                            Some((BangType::Comment, Bytes(b"!----")))
                        );
                        assert_eq!(position, 6);
                    }

                    #[test]
                    fn with_content() {
                        let buf = $buf;
                        let mut position = 0;
                        let mut input = b"!--->comment<--->other content".as_ref();
                        //                                 ^= 17

                        assert_eq!(
                            input
                                .read_bang_element(buf, &mut position)
                                .unwrap()
                                .map(|(ty, data)| (ty, Bytes(data))),
                            Some((BangType::Comment, Bytes(b"!--->comment<---")))
                        );
                        assert_eq!(position, 17);
                    }
                }

                /// Checks that reading DOCTYPE definition works correctly
                mod doctype {
                    mod uppercase {
                        use crate::errors::Error;
                        use crate::reader::{BangType, BufferedInput};
                        use crate::utils::Bytes;
                        use pretty_assertions::assert_eq;

                        #[test]
                        fn not_properly_start() {
                            let buf = $buf;
                            let mut position = 0;
                            let mut input = b"!D other content".as_ref();
                            //                ^= 0

                            match input.read_bang_element(buf, &mut position) {
                                Err(Error::UnexpectedEof(s)) if s == "DOCTYPE" => {}
                                x => assert!(
                                    false,
                                    r#"Expected `UnexpectedEof("DOCTYPE")`, but result is: {:?}"#,
                                    x
                                ),
                            }
                            assert_eq!(position, 0);
                        }

                        #[test]
                        fn without_space() {
                            let buf = $buf;
                            let mut position = 0;
                            let mut input = b"!DOCTYPEother content".as_ref();
                            //                ^= 0

                            match input.read_bang_element(buf, &mut position) {
                                Err(Error::UnexpectedEof(s)) if s == "DOCTYPE" => {}
                                x => assert!(
                                    false,
                                    r#"Expected `UnexpectedEof("DOCTYPE")`, but result is: {:?}"#,
                                    x
                                ),
                            }
                            assert_eq!(position, 0);
                        }

                        #[test]
                        fn empty() {
                            let buf = $buf;
                            let mut position = 0;
                            let mut input = b"!DOCTYPE>other content".as_ref();
                            //                         ^= 9

                            assert_eq!(
                                input
                                    .read_bang_element(buf, &mut position)
                                    .unwrap()
                                    .map(|(ty, data)| (ty, Bytes(data))),
                                Some((BangType::DocType, Bytes(b"!DOCTYPE")))
                            );
                            assert_eq!(position, 9);
                        }

                        #[test]
                        fn not_closed() {
                            let buf = $buf;
                            let mut position = 0;
                            let mut input = b"!DOCTYPE other content".as_ref();
                            //                ^= 0

                            match input.read_bang_element(buf, &mut position) {
                                Err(Error::UnexpectedEof(s)) if s == "DOCTYPE" => {}
                                x => assert!(
                                    false,
                                    r#"Expected `UnexpectedEof("DOCTYPE")`, but result is: {:?}"#,
                                    x
                                ),
                            }
                            assert_eq!(position, 0);
                        }
                    }

                    mod lowercase {
                        use crate::errors::Error;
                        use crate::reader::{BangType, BufferedInput};
                        use crate::utils::Bytes;
                        use pretty_assertions::assert_eq;

                        #[test]
                        fn not_properly_start() {
                            let buf = $buf;
                            let mut position = 0;
                            let mut input = b"!d other content".as_ref();
                            //                ^= 0

                            match input.read_bang_element(buf, &mut position) {
                                Err(Error::UnexpectedEof(s)) if s == "DOCTYPE" => {}
                                x => assert!(
                                    false,
                                    r#"Expected `UnexpectedEof("DOCTYPE")`, but result is: {:?}"#,
                                    x
                                ),
                            }
                            assert_eq!(position, 0);
                        }

                        #[test]
                        fn without_space() {
                            let buf = $buf;
                            let mut position = 0;
                            let mut input = b"!doctypeother content".as_ref();
                            //                ^= 0

                            match input.read_bang_element(buf, &mut position) {
                                Err(Error::UnexpectedEof(s)) if s == "DOCTYPE" => {}
                                x => assert!(
                                    false,
                                    r#"Expected `UnexpectedEof("DOCTYPE")`, but result is: {:?}"#,
                                    x
                                ),
                            }
                            assert_eq!(position, 0);
                        }

                        #[test]
                        fn empty() {
                            let buf = $buf;
                            let mut position = 0;
                            let mut input = b"!doctype>other content".as_ref();
                            //                         ^= 9

                            assert_eq!(
                                input
                                    .read_bang_element(buf, &mut position)
                                    .unwrap()
                                    .map(|(ty, data)| (ty, Bytes(data))),
                                Some((BangType::DocType, Bytes(b"!doctype")))
                            );
                            assert_eq!(position, 9);
                        }

                        #[test]
                        fn not_closed() {
                            let buf = $buf;
                            let mut position = 0;
                            let mut input = b"!doctype other content".as_ref();
                            //                ^= 0

                            match input.read_bang_element(buf, &mut position) {
                                Err(Error::UnexpectedEof(s)) if s == "DOCTYPE" => {}
                                x => assert!(
                                    false,
                                    r#"Expected `UnexpectedEof("DOCTYPE")`, but result is: {:?}"#,
                                    x
                                ),
                            }
                            assert_eq!(position, 0);
                        }
                    }
                }
            }

            mod read_element {
                use crate::reader::BufferedInput;
                use crate::utils::Bytes;
                use pretty_assertions::assert_eq;

                /// Checks that nothing was read from empty buffer
                #[test]
                fn empty() {
                    let buf = $buf;
                    let mut position = 0;
                    let mut input = b"".as_ref();
                    //                ^= 0

                    assert_eq!(input.read_element(buf, &mut position).unwrap().map(Bytes), None);
                    assert_eq!(position, 0);
                }

                mod open {
                    use crate::reader::BufferedInput;
                    use crate::utils::Bytes;
                    use pretty_assertions::assert_eq;

                    #[test]
                    fn empty_tag() {
                        let buf = $buf;
                        let mut position = 0;
                        let mut input = b">".as_ref();
                        //                 ^= 1

                        assert_eq!(
                            input.read_element(buf, &mut position).unwrap().map(Bytes),
                            Some(Bytes(b""))
                        );
                        assert_eq!(position, 1);
                    }

                    #[test]
                    fn normal() {
                        let buf = $buf;
                        let mut position = 0;
                        let mut input = b"tag>".as_ref();
                        //                    ^= 4

                        assert_eq!(
                            input.read_element(buf, &mut position).unwrap().map(Bytes),
                            Some(Bytes(b"tag"))
                        );
                        assert_eq!(position, 4);
                    }

                    #[test]
                    fn empty_ns_empty_tag() {
                        let buf = $buf;
                        let mut position = 0;
                        let mut input = b":>".as_ref();
                        //                  ^= 2

                        assert_eq!(
                            input.read_element(buf, &mut position).unwrap().map(Bytes),
                            Some(Bytes(b":"))
                        );
                        assert_eq!(position, 2);
                    }

                    #[test]
                    fn empty_ns() {
                        let buf = $buf;
                        let mut position = 0;
                        let mut input = b":tag>".as_ref();
                        //                     ^= 5

                        assert_eq!(
                            input.read_element(buf, &mut position).unwrap().map(Bytes),
                            Some(Bytes(b":tag"))
                        );
                        assert_eq!(position, 5);
                    }

                    #[test]
                    fn with_attributes() {
                        let buf = $buf;
                        let mut position = 0;
                        let mut input = br#"tag  attr-1=">"  attr2  =  '>'  3attr>"#.as_ref();
                        //                                                        ^= 38

                        assert_eq!(
                            input.read_element(buf, &mut position).unwrap().map(Bytes),
                            Some(Bytes(br#"tag  attr-1=">"  attr2  =  '>'  3attr"#))
                        );
                        assert_eq!(position, 38);
                    }
                }

                mod self_closed {
                    use crate::reader::BufferedInput;
                    use crate::utils::Bytes;
                    use pretty_assertions::assert_eq;

                    #[test]
                    fn empty_tag() {
                        let buf = $buf;
                        let mut position = 0;
                        let mut input = b"/>".as_ref();
                        //                  ^= 2

                        assert_eq!(
                            input.read_element(buf, &mut position).unwrap().map(Bytes),
                            Some(Bytes(b"/"))
                        );
                        assert_eq!(position, 2);
                    }

                    #[test]
                    fn normal() {
                        let buf = $buf;
                        let mut position = 0;
                        let mut input = b"tag/>".as_ref();
                        //                     ^= 5

                        assert_eq!(
                            input.read_element(buf, &mut position).unwrap().map(Bytes),
                            Some(Bytes(b"tag/"))
                        );
                        assert_eq!(position, 5);
                    }

                    #[test]
                    fn empty_ns_empty_tag() {
                        let buf = $buf;
                        let mut position = 0;
                        let mut input = b":/>".as_ref();
                        //                   ^= 3

                        assert_eq!(
                            input.read_element(buf, &mut position).unwrap().map(Bytes),
                            Some(Bytes(b":/"))
                        );
                        assert_eq!(position, 3);
                    }

                    #[test]
                    fn empty_ns() {
                        let buf = $buf;
                        let mut position = 0;
                        let mut input = b":tag/>".as_ref();
                        //                      ^= 6

                        assert_eq!(
                            input.read_element(buf, &mut position).unwrap().map(Bytes),
                            Some(Bytes(b":tag/"))
                        );
                        assert_eq!(position, 6);
                    }

                    #[test]
                    fn with_attributes() {
                        let buf = $buf;
                        let mut position = 0;
                        let mut input = br#"tag  attr-1="/>"  attr2  =  '/>'  3attr/>"#.as_ref();
                        //                                                           ^= 41

                        assert_eq!(
                            input.read_element(buf, &mut position).unwrap().map(Bytes),
                            Some(Bytes(br#"tag  attr-1="/>"  attr2  =  '/>'  3attr/"#))
                        );
                        assert_eq!(position, 41);
                    }
                }
            }

            mod issue_344 {
                use crate::errors::Error;

                #[test]
                fn cdata() {
                    let doc = "![]]>";
                    let mut reader = crate::Reader::from_str(doc);

                    match reader.read_until_close($buf) {
                        Err(Error::UnexpectedEof(s)) if s == "CData" => {}
                        x => assert!(
                            false,
                            r#"Expected `UnexpectedEof("CData")`, but result is: {:?}"#,
                            x
                        ),
                    }
                }

                #[test]
                fn comment() {
                    let doc = "!- -->";
                    let mut reader = crate::Reader::from_str(doc);

                    match reader.read_until_close($buf) {
                        Err(Error::UnexpectedEof(s)) if s == "Comment" => {}
                        x => assert!(
                            false,
                            r#"Expected `UnexpectedEof("Comment")`, but result is: {:?}"#,
                            x
                        ),
                    }
                }

                #[test]
                fn doctype_uppercase() {
                    let doc = "!D>";
                    let mut reader = crate::Reader::from_str(doc);

                    match reader.read_until_close($buf) {
                        Err(Error::UnexpectedEof(s)) if s == "DOCTYPE" => {}
                        x => assert!(
                            false,
                            r#"Expected `UnexpectedEof("DOCTYPE")`, but result is: {:?}"#,
                            x
                        ),
                    }
                }

                #[test]
                fn doctype_lowercase() {
                    let doc = "!d>";
                    let mut reader = crate::Reader::from_str(doc);

                    match reader.read_until_close($buf) {
                        Err(Error::UnexpectedEof(s)) if s == "DOCTYPE" => {}
                        x => assert!(
                            false,
                            r#"Expected `UnexpectedEof("DOCTYPE")`, but result is: {:?}"#,
                            x
                        ),
                    }
                }
            }
        };
    }

    /// Tests for reader that generates events that borrow from the provided buffer
    mod buffered {
        check!(&mut Vec::new());
    }

    /// Tests for reader that generates events that borrow from the input
    mod borrowed {
        check!(());
    }
}

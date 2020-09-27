//! A module to handle the `AsyncReader`

use async_recursion::async_recursion;
#[cfg(feature = "encoding")]
use encoding_rs::Encoding;
use std::future::Future;
use std::io;
use std::marker::Unpin;
use std::path::Path;
use std::pin::Pin;
use std::str::from_utf8;
use std::task::{Context, Poll};
use tokio::fs::File;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, BufReader};

use crate::errors::{Error, Result};
use crate::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};

use super::{is_whitespace, Decode, Decoder, NamespaceBufferIndex, TagState};

impl<B: AsyncBufRead> Decode for AsyncReader<B> {
    #[cfg(feature = "encoding")]
    fn read_encoding(&self) -> &'static Encoding {
        self.encoding
    }

    #[cfg(feature = "encoding")]
    fn read_is_encoding_set(&self) -> bool {
        self.is_encoding_set
    }

    #[cfg(feature = "encoding")]
    fn write_encoding(&mut self, val: &'static Encoding) {
        self.encoding = val;
    }

    #[cfg(feature = "encoding")]
    fn write_is_encoding_set(&mut self, val: bool) {
        self.is_encoding_set = val;
    }
}

/// A low level encoding-agnostic XML event reader.
///
/// Consumes a `BufRead` and streams XML `Event`s.
///
/// # Examples
///
/// ```
/// use quick_xml::AsyncReader;
/// use quick_xml::events::Event;
///
/// #[tokio::main]
/// async fn main() {
///     let xml = r#"<tag1 att1 = "test">
///                     <tag2><!--Test comment-->Test</tag2>
///                     <tag2>Test 2</tag2>
///                 </tag1>"#;
///     let mut reader = AsyncReader::from_str(xml);
///     reader.trim_text(true);
///     let mut count = 0;
///     let mut txt = Vec::new();
///     let mut buf = Vec::new();
///     loop {
///         match reader.read_event(&mut buf).await {
///             Ok(Event::Start(ref e)) => {
///                 match e.name() {
///                     b"tag1" => println!("attributes values: {:?}",
///                                         e.attributes().map(|a| a.unwrap().value)
///                                         .collect::<Vec<_>>()),
///                     b"tag2" => count += 1,
///                     _ => (),
///                 }
///             },
///             Ok(Event::Text(e)) => txt.push(e.unescape_and_decode(&reader).unwrap()),
///             Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
///             Ok(Event::Eof) => break,
///             _ => (),
///         }
///         buf.clear();
///     }
/// }
/// ```
pub struct AsyncReader<B: AsyncBufRead> {
    /// reader
    reader: B,
    /// current buffer position, useful for debuging errors
    buf_position: usize,
    /// current state Open/Close
    tag_state: TagState,
    /// expand empty element into an opening and closing element
    expand_empty_elements: bool,
    /// trims Text events, skip the element if text is empty
    trim_text: bool,
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

impl<B: AsyncBufRead + Unpin + Send> AsyncReader<B> {
    /// Creates a `Reader` that reads from a reader implementing `BufRead`.
    pub fn from_reader(reader: B) -> AsyncReader<B> {
        AsyncReader {
            reader,
            opened_buffer: Vec::new(),
            opened_starts: Vec::new(),
            tag_state: TagState::Closed,
            expand_empty_elements: false,
            trim_text: false,
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
    pub fn expand_empty_elements(&mut self, val: bool) -> &mut AsyncReader<B> {
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
    pub fn trim_text(&mut self, val: bool) -> &mut AsyncReader<B> {
        self.trim_text = val;
        self
    }

    /// Changes wether trailing whitespaces after the markup name are trimmed in closing tags
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
    pub fn trim_markup_names_in_closing_tags(&mut self, val: bool) -> &mut AsyncReader<B> {
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
    pub fn check_end_names(&mut self, val: bool) -> &mut AsyncReader<B> {
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
    pub fn check_comments(&mut self, val: bool) -> &mut AsyncReader<B> {
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
    #[async_recursion]
    async fn read_until_open<'a, 'b>(&'a mut self, buf: &'b mut Vec<u8>) -> Result<Event<'b>> {
        self.tag_state = TagState::Opened;
        let buf_start = buf.len();

        match read_until(&mut self.reader, b'<', buf, &mut self.buf_position).await {
            Ok(0) => Ok(Event::Eof),
            Ok(_) => {
                let (start, len) = if self.trim_text {
                    match buf.iter().skip(buf_start).position(|&b| !is_whitespace(b)) {
                        Some(start) => (
                            buf_start + start,
                            buf.iter()
                                .rposition(|&b| !is_whitespace(b))
                                .map_or_else(|| buf.len(), |p| p + 1),
                        ),
                        None => return self.read_event(buf).await,
                    }
                } else {
                    (buf_start, buf.len())
                };
                Ok(Event::Text(BytesText::from_escaped(&buf[start..len])))
            }
            Err(e) => Err(e),
        }
    }

    /// private function to read until '>' is found
    async fn read_until_close<'a, 'b>(&'a mut self, buf: &'b mut Vec<u8>) -> Result<Event<'b>> {
        self.tag_state = TagState::Closed;

        // need to read 1 character to decide whether pay special attention to attribute values
        let buf_start = buf.len();

        let start = match read_one_dont_consume(&mut self.reader).await {
            Ok(n) if n.is_none() => return Ok(Event::Eof),
            Ok(n) => n.unwrap(),
            Err(e) => return Err(Error::Io(e)),
        };

        if start != b'/' && start != b'!' && start != b'?' {
            match read_elem_until(&mut self.reader, b'>', buf, &mut self.buf_position).await {
                Ok(0) => Ok(Event::Eof),
                Ok(_) => {
                    // we already *know* that we are in this case
                    self.read_start(&buf[buf_start..])
                }
                Err(e) => Err(e),
            }
        } else {
            match read_until(&mut self.reader, b'>', buf, &mut self.buf_position).await {
                Ok(0) => Ok(Event::Eof),
                Ok(_) => match start {
                    b'/' => self.read_end(&buf[buf_start..]),
                    b'!' => self.read_bang(buf_start, buf).await,
                    b'?' => self.read_question_mark(&buf[buf_start..]),
                    _ => unreachable!(
                        "We checked that `start` must be one of [/!?], was {:?} \
                                 instead.",
                        start
                    ),
                },
                Err(e) => Err(e),
            }
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
    ///
    /// Note: depending on the start of the Event, we may need to read more
    /// data, thus we need a mutable buffer
    async fn read_bang<'a, 'b>(
        &'a mut self,
        buf_start: usize,
        buf: &'b mut Vec<u8>,
    ) -> Result<Event<'b>> {
        if buf[buf_start..].starts_with(b"!--") {
            while buf.len() < buf_start + 5 || !buf.ends_with(b"--") {
                buf.push(b'>');
                match read_until(&mut self.reader, b'>', buf, &mut self.buf_position).await {
                    Ok(0) => {
                        // In sync sometimes the last char is included and sometimes it isn't
                        self.buf_position -= 1;
                        self.buf_position -= buf.len() - buf_start;
                        return Err(Error::UnexpectedEof("Comment".to_string()));
                    }
                    Ok(_) => (),
                    Err(e) => return Err(e),
                }
            }
            let len = buf.len();
            if self.check_comments {
                // search if '--' not in comments
                if let Some(p) = memchr::memchr_iter(b'-', &buf[buf_start + 3..len - 2])
                    .position(|p| buf[buf_start + 3 + p + 1] == b'-')
                {
                    self.buf_position -= buf.len() - buf_start + p;
                    return Err(Error::UnexpectedToken("--".to_string()));
                }
            }
            Ok(Event::Comment(BytesText::from_escaped(
                &buf[buf_start + 3..len - 2],
            )))
        } else if buf.len() >= buf_start + 8 {
            match &buf[buf_start + 1..buf_start + 8] {
                b"[CDATA[" => {
                    while buf.len() < 10 || !buf.ends_with(b"]]") {
                        buf.push(b'>');
                        match read_until(&mut self.reader, b'>', buf, &mut self.buf_position).await
                        {
                            Ok(0) => {
                                self.buf_position -= buf.len() - buf_start;
                                return Err(Error::UnexpectedEof("CData".to_string()));
                            }
                            Ok(_) => (),
                            Err(e) => return Err(e),
                        }
                    }
                    Ok(Event::CData(BytesText::from_plain(
                        &buf[buf_start + 8..buf.len() - 2],
                    )))
                }
                x if x.eq_ignore_ascii_case(b"DOCTYPE") => {
                    let mut count = buf.iter().skip(buf_start).filter(|&&b| b == b'<').count();
                    while count > 0 {
                        buf.push(b'>');
                        match read_until(&mut self.reader, b'>', buf, &mut self.buf_position).await
                        {
                            Ok(0) => {
                                self.buf_position -= buf.len() - buf_start;
                                return Err(Error::UnexpectedEof("DOCTYPE".to_string()));
                            }
                            Ok(n) => {
                                let start = buf.len() - n;
                                count += buf.iter().skip(start).filter(|&&b| b == b'<').count();
                                count -= 1;
                            }
                            Err(e) => return Err(e),
                        }
                    }
                    Ok(Event::DocType(BytesText::from_escaped(
                        &buf[buf_start + 8..buf.len()],
                    )))
                }
                _ => return Err(Error::UnexpectedBang),
            }
        } else {
            self.buf_position -= buf.len() - buf_start;
            return Err(Error::UnexpectedBang);
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
    /// use quick_xml::AsyncReader;
    /// use quick_xml::events::Event;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let xml = r#"<tag1 att1 = "test">
    ///                     <tag2><!--Test comment-->Test</tag2>
    ///                     <tag2>Test 2</tag2>
    ///                 </tag1>"#;
    ///     let mut reader = AsyncReader::from_str(xml);
    ///     reader.trim_text(true);
    ///     let mut count = 0;
    ///     let mut buf = Vec::new();
    ///     let mut txt = Vec::new();
    ///     loop {
    ///         match reader.read_event(&mut buf).await {
    ///             Ok(Event::Start(ref e)) => count += 1,
    ///             Ok(Event::Text(e)) => txt.push(e.unescape_and_decode(&reader).expect("Error!")),
    ///             Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
    ///             Ok(Event::Eof) => break,
    ///             _ => (),
    ///         }
    ///         buf.clear();
    ///     }
    ///     println!("Found {} start events", count);
    ///     println!("Text events: {:?}", txt);
    /// }
    /// ```
    #[async_recursion]
    pub async fn read_event<'a, 'b>(&'a mut self, buf: &'b mut Vec<u8>) -> Result<Event<'b>> {
        let event = match self.tag_state {
            TagState::Opened => self.read_until_close(buf).await,
            TagState::Closed => self.read_until_open(buf).await,
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
    /// use quick_xml::AsyncReader;
    /// use quick_xml::events::Event;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let xml = r#"<x:tag1 xmlns:x="www.xxxx" xmlns:y="www.yyyy" att1 = "test">
    ///                     <y:tag2><!--Test comment-->Test</y:tag2>
    ///                     <y:tag2>Test 2</y:tag2>
    ///                 </x:tag1>"#;
    ///     let mut reader = AsyncReader::from_str(xml);
    ///     reader.trim_text(true);
    ///     let mut count = 0;
    ///     let mut buf = Vec::new();
    ///     let mut ns_buf = Vec::new();
    ///     let mut txt = Vec::new();
    ///     loop {
    ///         match reader.read_namespaced_event(&mut buf, &mut ns_buf).await {
    ///             Ok((ref ns, Event::Start(ref e))) => {
    ///                 count += 1;
    ///                 match (*ns, e.local_name()) {
    ///                     (Some(b"www.xxxx"), b"tag1") => (),
    ///                     (Some(b"www.yyyy"), b"tag2") => (),
    ///                     (ns, n) => panic!("Namespace and local name mismatch"),
    ///                 }
    ///                 println!("Resolved namespace: {:?}", ns.and_then(|ns| from_utf8(ns).ok()));
    ///             }
    ///             Ok((_, Event::Text(e))) => {
    ///                 txt.push(e.unescape_and_decode(&reader).expect("Error!"))
    ///             },
    ///             Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
    ///             Ok((_, Event::Eof)) => break,
    ///             _ => (),
    ///         }
    ///         buf.clear();
    ///     }
    ///     println!("Found {} start events", count);
    ///     println!("Text events: {:?}", txt);
    /// }
    /// ```
    pub async fn read_namespaced_event<'a, 'b, 'c>(
        &'a mut self,
        buf: &'b mut Vec<u8>,
        namespace_buffer: &'c mut Vec<u8>,
    ) -> Result<(Option<&'c [u8]>, Event<'b>)> {
        self.ns_buffer.pop_empty_namespaces(namespace_buffer);
        match self.read_event(buf).await {
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

    /// Returns the `AsyncReader`s encoding.
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
    pub async fn read_to_end<K: AsRef<[u8]>>(&mut self, end: K, buf: &mut Vec<u8>) -> Result<()> {
        let mut depth = 0;
        let end = end.as_ref();
        loop {
            match self.read_event(buf).await {
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
    /// use quick_xml::AsyncReader;
    /// use quick_xml::events::Event;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut xml = AsyncReader::from_reader(b"
    ///         <a>&lt;b&gt;</a>
    ///         <a></a>
    ///     " as &[u8]);
    ///     xml.trim_text(true);
    ///
    ///     let expected = ["<b>", ""];
    ///     for &content in expected.iter() {
    ///         match xml.read_event(&mut Vec::new()).await {
    ///             Ok(Event::Start(ref e)) => {
    ///                 assert_eq!(&xml.read_text(e.name(), &mut Vec::new()).await.unwrap(), content);
    ///             },
    ///             e => panic!("Expecting Start event, found {:?}", e),
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// [`Text`]: events/enum.Event.html#variant.Text
    /// [`End`]: events/enum.Event.html#variant.End
    pub async fn read_text<K: AsRef<[u8]>>(&mut self, end: K, buf: &mut Vec<u8>) -> Result<String> {
        let s = match self.read_event(buf).await {
            Ok(Event::Text(e)) => e.unescape_and_decode(self),
            Ok(Event::End(ref e)) if e.name() == end.as_ref() => return Ok("".to_string()),
            Err(e) => return Err(e),
            Ok(Event::Eof) => return Err(Error::UnexpectedEof("Text".to_string())),
            _ => return Err(Error::TextNotFound),
        };
        self.read_to_end(end, buf).await?;
        s
    }

    /// Consumes `AsyncReader` returning the underlying reader
    ///
    /// Can be used to compute line and column of a parsing error position
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use std::{str, io::Cursor};
    /// use quick_xml::AsyncReader;
    /// use quick_xml::events::Event;
    ///
    /// fn into_line_and_column(reader: AsyncReader<Cursor<&[u8]>>) -> (usize, usize) {
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
    /// #[tokio::main]
    /// async fn main() {
    ///     let xml = r#"<tag1 att1 = "test">
    ///                     <tag2><!--Test comment-->Test</tag2>
    ///                     <tag3>Test 2</tag3>
    ///                 </tag1>"#;
    ///     let mut reader = AsyncReader::from_reader(Cursor::new(xml.as_bytes()));
    ///     let mut buf = Vec::new();
    ///
    ///     loop {
    ///         match reader.read_event(&mut buf).await {
    ///             Ok(Event::Start(ref e)) => match e.name() {
    ///                 b"tag1" | b"tag2" => (),
    ///                 tag => {
    ///                     assert_eq!(b"tag3", tag);
    ///                     assert_eq!((3, 22), into_line_and_column(reader));
    ///                     break;
    ///                 }
    ///             },
    ///             Ok(Event::Eof) => unreachable!(),
    ///             _ => (),
    ///         }
    ///         buf.clear();
    ///     }
    /// }
    /// ```
    pub fn into_underlying_reader(self) -> B {
        self.reader
    }
}

impl AsyncReader<BufReader<File>> {
    /// Creates an XML reader from a file path.
    pub async fn from_file<P: AsRef<Path>>(path: P) -> Result<AsyncReader<BufReader<File>>> {
        let file = File::open(path).await.map_err(Error::Io)?;
        let reader = BufReader::new(file);
        Ok(AsyncReader::from_reader(reader))
    }
}

impl<'a> AsyncReader<&'a [u8]> {
    /// Creates an XML reader from a string slice.
    pub fn from_str(s: &'a str) -> AsyncReader<&'a [u8]> {
        AsyncReader::from_reader(s.as_bytes())
    }
}

/// Container for a future that reads one byte from a reader
/// but does not consume the byte, so it can be read again.
#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct ReadOneDontConsume<'a, R: ?Sized> {
    reader: &'a mut R,
}

fn read_one_dont_consume<'a, R>(reader: &'a mut R) -> ReadOneDontConsume<'a, R>
where
    R: AsyncBufRead + ?Sized + Unpin,
{
    ReadOneDontConsume { reader }
}

fn read_one_dont_consume_internal<R: AsyncBufRead + ?Sized>(
    mut reader: Pin<&mut R>,
    cx: &mut Context<'_>,
) -> Poll<io::Result<Option<u8>>> {
    match reader.as_mut().poll_fill_buf(cx) {
        Poll::Ready(t) => Poll::Ready(t.map(|s| if s.is_empty() { None } else { Some(s[0]) })),
        Poll::Pending => Poll::Pending,
    }
}

impl<R: AsyncBufRead + ?Sized + Unpin> Future for ReadOneDontConsume<'_, R> {
    type Output = io::Result<Option<u8>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Self { reader } = &mut *self;
        read_one_dont_consume_internal(Pin::new(reader), cx)
    }
}

/// read until `byte` is found or end of file
/// return the position of byte
#[inline]
async fn read_until<R: AsyncBufRead + ?Sized + Unpin>(
    r: &mut R,
    byte: u8,
    buf: &mut Vec<u8>,
    buf_position: &mut usize,
) -> Result<usize> {
    let result = r.read_until(byte, buf).await;

    if let Ok(size) = result {
        if buf.len() > 0 && buf[buf.len() - 1] == byte {
            buf.remove(buf.len() - 1);
        }
        *buf_position += size;
    }

    result.map_err(Error::Io)

    // let mut read = 0;
    // let mut done = false;
    // while !done {
    //     let used = {
    //         let available = match r.fill_buf() {
    //             Ok(n) if n.is_empty() => break,
    //             Ok(n) => n,
    //             Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
    //             Err(e) => {
    //                 *position += read;
    //                 return Err(Error::Io(e));
    //             }
    //         };

    //         match memchr::memchr(byte, available) {
    //             Some(i) => {
    //                 buf.extend_from_slice(&available[..i]);
    //                 done = true;
    //                 i + 1
    //             }
    //             None => {
    //                 buf.extend_from_slice(available);
    //                 available.len()
    //             }
    //         }
    //     };
    //     r.consume(used);
    //     read += used;
    // }
    // *position += read;
    // Ok(read)
}

/// Derived from `read_until`, but modified to handle XML attributes using a minimal state machine.
/// [W3C Extensible Markup Language (XML) 1.1 (2006)](https://www.w3.org/TR/xml11)
///
/// Attribute values are defined as follows:
/// ```plain
/// AttValue := '"' (([^<&"]) | Reference)* '"'
///           | "'" (([^<&']) | Reference)* "'"
/// ```
/// (`Reference` is something like `&quot;`, but we don't care about escaped characters at this
/// level)
#[inline]
async fn read_elem_until<R: AsyncBufRead + Unpin>(
    r: &mut R,
    end_byte: u8,
    buf: &mut Vec<u8>,
    position: &mut usize,
) -> Result<usize> {
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
    let mut read = 0;
    let mut done = false;
    while !done {
        let used = {
            let available = match r.read_until(end_byte, buf).await {
                Ok(n) if n == 0 => {
                    buf.remove(buf.len() - 1);
                    return Ok(read);
                }
                Ok(n) => {
                    let len = buf.len();
                    &buf[len - n..len]
                }
                Err(ref e) if e.kind() == tokio::io::ErrorKind::Interrupted => continue,
                Err(e) => {
                    *position += read;
                    return Err(Error::Io(e));
                }
            };

            let mut memiter = memchr::memchr3_iter(end_byte, b'\'', b'"', &available);
            let used: usize;
            loop {
                match memiter.next() {
                    Some(i) => {
                        state = match (state, available[i]) {
                            (State::Elem, b) if b == end_byte => {
                                // only allowed to match `end_byte` while we are in state `Elem`
                                done = true;
                                used = i + 1;
                                break;
                            }
                            (State::Elem, b'\'') => State::SingleQ,
                            (State::Elem, b'\"') => State::DoubleQ,

                            // the only end_byte that gets us out if the same character
                            (State::SingleQ, b'\'') | (State::DoubleQ, b'\"') => State::Elem,

                            // all other bytes: no state change
                            _ => state,
                        };
                    }
                    None => {
                        used = available.len();
                        break;
                    }
                }
            }

            used
        };
        read += used;
    }

    buf.remove(buf.len() - 1);

    *position += read;
    Ok(read)
}

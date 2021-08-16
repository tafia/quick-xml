//! A module to handle `Reader`

use std::borrow::Cow;
use std::io::{self, BufRead, BufReader};
use std::{fs::File, path::Path, str::from_utf8};

#[cfg(feature = "encoding")]
use encoding_rs::{Encoding, UTF_16BE, UTF_16LE, UTF_8};

use crate::errors::{Error, Result};
use crate::events::{BytesCData, BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use crate::name::{LocalName, NamespaceResolver, QName, ResolveResult};

use memchr;

/// Possible reader states. The state transition diagram (`true` and `false` shows
/// value of [`Reader::expand_empty_elements()`] option):
///
/// ```mermaid
/// flowchart LR
///   subgraph _
///     direction LR
///
///     Init   -- "(no event)"\nStartText                              --> Opened
///     Opened -- Decl, DocType, PI\nComment, CData\nStart, Empty, End --> Closed
///     Closed -- "#lt;false#gt;\n(no event)"\nText                    --> Opened
///   end
///   Closed -- "#lt;true#gt;"\nStart --> Empty
///   Empty  -- End                   --> Closed
///   _ -. Eof .-> Exit
/// ```
#[derive(Clone)]
enum TagState {
    /// Initial state in which reader stay after creation. Transition from that
    /// state could produce a `StartText`, `Decl`, `Comment` or `Start` event.
    /// The next state is always `Opened`. The reader will never return to this
    /// state. The event emitted during transition to `Opened` is a `StartEvent`
    /// if the first symbol not `<`, otherwise no event are emitted.
    Init,
    /// State after seeing the `<` symbol. Depending on the next symbol all other
    /// events (except `StartText`) could be generated.
    ///
    /// After generating ane event the reader moves to the `Closed` state.
    Opened,
    /// State in which reader searches the `<` symbol of a markup. All bytes before
    /// that symbol will be returned in the [`Event::Text`] event. After that
    /// the reader moves to the `Opened` state.
    Closed,
    /// This state is used only if option `expand_empty_elements` is set to `true`.
    /// Reader enters to this state when it is in a `Closed` state and emits an
    /// [`Event::Start`] event. The next event emitted will be an [`Event::End`],
    /// after which reader returned to the `Closed` state.
    Empty,
    /// Reader enters this state when `Eof` event generated or an error occurred.
    /// This is the last state, the reader stay in it forever.
    Exit,
}

/// A low level encoding-agnostic XML event reader.
///
/// Consumes a `BufRead` and streams XML `Event`s.
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
/// let mut txt = Vec::new();
/// let mut buf = Vec::new();
/// loop {
///     match reader.read_event(&mut buf) {
///         Ok(Event::Start(ref e)) => {
///             match e.name().as_ref() {
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
    /// All currently Started elements which didn't have a matching
    /// End element yet.
    ///
    /// For an XML
    ///
    /// ```xml
    /// <root><one/><inner attr="value">|<tag></inner></root>
    /// ```
    /// when cursor at the `|` position buffer contains:
    ///
    /// ```text
    /// rootinner
    /// ^   ^
    /// ```
    ///
    /// The `^` symbols shows which positions stored in the [`Self::opened_starts`]
    /// (0 and 4 in that case).
    opened_buffer: Vec<u8>,
    /// Opened name start indexes into [`Self::opened_buffer`]. See documentation
    /// for that field for details
    opened_starts: Vec<usize>,

    /// A buffer to manage namespaces
    ns_resolver: NamespaceResolver,
    /// For `Empty` events keep the 'scope' of the namespace on the stack artificially. That way, the
    /// consumer has a chance to use `resolve` in the context of the empty element. We perform the
    /// pop as the first operation in the next `next()` call.
    pending_pop: bool,

    #[cfg(feature = "encoding")]
    /// the encoding specified in the xml, defaults to utf8
    encoding: &'static Encoding,
    #[cfg(feature = "encoding")]
    /// check if quick-rs could find out the encoding
    is_encoding_set: bool,
}

/// Builder methods
impl<R: BufRead> Reader<R> {
    /// Creates a `Reader` that reads from a reader implementing `BufRead`.
    pub fn from_reader(reader: R) -> Self {
        Self {
            reader,
            opened_buffer: Vec::new(),
            opened_starts: Vec::new(),
            tag_state: TagState::Init,
            expand_empty_elements: false,
            trim_text_start: false,
            trim_text_end: false,
            trim_markup_names_in_closing_tags: true,
            check_end_names: true,
            buf_position: 0,
            check_comments: false,

            ns_resolver: NamespaceResolver::default(),
            pending_pop: false,

            #[cfg(feature = "encoding")]
            encoding: ::encoding_rs::UTF_8,
            #[cfg(feature = "encoding")]
            is_encoding_set: false,
        }
    }

    /// Changes whether empty elements should be split into an `Open` and a `Close` event.
    ///
    /// When set to `true`, all [`Empty`] events produced by a self-closing tag like `<tag/>` are
    /// expanded into a [`Start`] event followed by an [`End`] event. When set to `false` (the
    /// default), those tags are represented by an [`Empty`] event instead.
    ///
    /// Note, that setting this to `true` will lead to additional allocates that
    /// needed to store tag name for an [`End`] event. There is no additional
    /// allocation, however, if [`Self::check_end_names()`] is also set.
    ///
    /// (`false` by default)
    ///
    /// [`Empty`]: events/enum.Event.html#variant.Empty
    /// [`Start`]: events/enum.Event.html#variant.Start
    /// [`End`]: events/enum.Event.html#variant.End
    pub fn expand_empty_elements(&mut self, val: bool) -> &mut Self {
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
    pub fn trim_text(&mut self, val: bool) -> &mut Self {
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
    pub fn trim_text_end(&mut self, val: bool) -> &mut Self {
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
    pub fn trim_markup_names_in_closing_tags(&mut self, val: bool) -> &mut Self {
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
    /// Note, that setting this to `true` will lead to additional allocates that
    /// needed to store tag name for an [`End`] event. There is no additional
    /// allocation, however, if [`Self::expand_empty_elements()`] is also set.
    ///
    /// (`true` by default)
    ///
    /// [`End`]: events/enum.Event.html#variant.End
    pub fn check_end_names(&mut self, val: bool) -> &mut Self {
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
    pub fn check_comments(&mut self, val: bool) -> &mut Self {
        self.check_comments = val;
        self
    }
}

/// Getters
impl<R: BufRead> Reader<R> {
    /// Consumes `Reader` returning the underlying reader
    ///
    /// Can be used to compute line and column of a parsing error position
    ///
    /// # Examples
    ///
    /// ```
    /// # use pretty_assertions::assert_eq;
    /// use std::{str, io::Cursor};
    /// use quick_xml::Reader;
    /// use quick_xml::events::Event;
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
    ///     let mut cursor = reader.into_inner();
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
    ///         Ok(Event::Start(ref e)) => match e.name().as_ref() {
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
    pub fn into_inner(self) -> R {
        self.reader
    }

    /// Gets a reference to the underlying reader.
    pub fn get_ref(&self) -> &R {
        &self.reader
    }

    /// Gets a mutable reference to the underlying reader.
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.reader
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

    /// Resolves a potentially qualified **event name** into (namespace name, local name).
    ///
    /// *Qualified* attribute names have the form `prefix:local-name` where the`prefix` is defined
    /// on any containing XML element via `xmlns:prefix="the:namespace:uri"`. The namespace prefix
    /// can be defined on the same element as the attribute in question.
    ///
    /// *Unqualified* event inherits the current *default namespace*.
    ///
    /// # Lifetimes
    ///
    /// - `'n`: lifetime of an element name
    /// - `'ns`: lifetime of a namespaces buffer, where all found namespaces are stored
    #[inline]
    pub fn event_namespace<'n, 'ns>(
        &self,
        name: QName<'n>,
        namespace_buffer: &'ns [u8],
    ) -> (ResolveResult<'ns>, LocalName<'n>) {
        self.ns_resolver.resolve(name, namespace_buffer, true)
    }

    /// Resolves a potentially qualified **attribute name** into (namespace name, local name).
    ///
    /// *Qualified* attribute names have the form `prefix:local-name` where the`prefix` is defined
    /// on any containing XML element via `xmlns:prefix="the:namespace:uri"`. The namespace prefix
    /// can be defined on the same element as the attribute in question.
    ///
    /// *Unqualified* attribute names do *not* inherit the current *default namespace*.
    ///
    /// # Lifetimes
    ///
    /// - `'n`: lifetime of an attribute
    /// - `'ns`: lifetime of a namespaces buffer, where all found namespaces are stored
    #[inline]
    pub fn attribute_namespace<'n, 'ns>(
        &self,
        name: QName<'n>,
        namespace_buffer: &'ns [u8],
    ) -> (ResolveResult<'ns>, LocalName<'n>) {
        self.ns_resolver.resolve(name, namespace_buffer, false)
    }

    /// Get the decoder, used to decode bytes, read by this reader, to the strings.
    ///
    /// If `encoding` feature is enabled, the used encoding may change after
    /// parsing the XML declaration, otherwise encoding is fixed to UTF-8.
    ///
    /// If `encoding` feature is enabled and no encoding is specified in declaration,
    /// defaults to UTF-8.
    pub fn decoder(&self) -> Decoder {
        Decoder {
            #[cfg(feature = "encoding")]
            encoding: self.encoding,
        }
    }
}

/// Read methods
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
    pub fn read_event<'b>(&mut self, buf: &'b mut Vec<u8>) -> Result<Event<'b>> {
        self.read_event_buffered(buf)
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
    pub fn read_namespaced_event<'b, 'ns>(
        &mut self,
        buf: &'b mut Vec<u8>,
        namespace_buffer: &'ns mut Vec<u8>,
    ) -> Result<(ResolveResult<'ns>, Event<'b>)> {
        if self.pending_pop {
            self.ns_resolver.pop(namespace_buffer);
        }
        self.pending_pop = false;
        match self.read_event(buf) {
            Ok(Event::Eof) => Ok((ResolveResult::Unbound, Event::Eof)),
            Ok(Event::Start(e)) => {
                self.ns_resolver.push(&e, namespace_buffer);
                Ok((
                    self.ns_resolver.find(e.name(), namespace_buffer),
                    Event::Start(e),
                ))
            }
            Ok(Event::Empty(e)) => {
                // For empty elements we need to 'artificially' keep the namespace scope on the
                // stack until the next `next()` call occurs.
                // Otherwise the caller has no chance to use `resolve` in the context of the
                // namespace declarations that are 'in scope' for the empty element alone.
                // Ex: <img rdf:nodeID="abc" xmlns:rdf="urn:the-rdf-uri" />
                self.ns_resolver.push(&e, namespace_buffer);
                // notify next `read_namespaced_event()` invocation that it needs to pop this
                // namespace scope
                self.pending_pop = true;
                Ok((
                    self.ns_resolver.find(e.name(), namespace_buffer),
                    Event::Empty(e),
                ))
            }
            Ok(Event::End(e)) => {
                // notify next `read_namespaced_event()` invocation that it needs to pop this
                // namespace scope
                self.pending_pop = true;
                Ok((
                    self.ns_resolver.find(e.name(), namespace_buffer),
                    Event::End(e),
                ))
            }
            Ok(e) => Ok((ResolveResult::Unbound, e)),
            Err(e) => Err(e),
        }
    }

    /// Reads until end element is found
    ///
    /// Manages nested cases where parent and child elements have the same name
    pub fn read_to_end<K: AsRef<[u8]>>(&mut self, end: K, buf: &mut Vec<u8>) -> Result<()> {
        let mut depth = 0;
        let end = end.as_ref();
        loop {
            match self.read_event(buf) {
                Ok(Event::End(ref e)) if e.name().as_ref() == end => {
                    if depth == 0 {
                        return Ok(());
                    }
                    depth -= 1;
                }
                Ok(Event::Start(ref e)) if e.name().as_ref() == end => depth += 1,
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
            Ok(Event::End(ref e)) if e.name().as_ref() == end.as_ref() => return Ok("".to_string()),
            Err(e) => return Err(e),
            Ok(Event::Eof) => return Err(Error::UnexpectedEof("Text".to_string())),
            _ => return Err(Error::TextNotFound),
        };
        self.read_to_end(end, buf)?;
        s
    }
}

/// Private methods
impl<R: BufRead> Reader<R> {
    /// Read text into the given buffer, and return an event that borrows from
    /// either that buffer or from the input itself, based on the type of the
    /// reader.
    fn read_event_buffered<'i, B>(&mut self, buf: B) -> Result<Event<'i>>
    where
        R: XmlSource<'i, B>,
    {
        let event = match self.tag_state {
            TagState::Init => self.read_until_open(buf, true),
            TagState::Closed => self.read_until_open(buf, false),
            TagState::Opened => self.read_until_close(buf),
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
    fn read_until_open<'i, B>(&mut self, buf: B, first: bool) -> Result<Event<'i>>
    where
        R: XmlSource<'i, B>,
    {
        self.tag_state = TagState::Opened;

        if self.trim_text_start {
            self.reader.skip_whitespace(&mut self.buf_position)?;
        }

        // If we already at the `<` symbol, do not try to return an empty Text event
        if self.reader.skip_one(b'<', &mut self.buf_position)? {
            return self.read_event_buffered(buf);
        }

        match self
            .reader
            .read_bytes_until(b'<', buf, &mut self.buf_position)
        {
            Ok(Some(bytes)) => {
                #[cfg(feature = "encoding")]
                if first {
                    if let Some(encoding) = detect_encoding(bytes) {
                        self.encoding = encoding;
                        self.is_encoding_set = true;
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
    fn read_until_close<'i, B>(&mut self, buf: B) -> Result<Event<'i>>
    where
        R: XmlSource<'i, B>,
    {
        self.tag_state = TagState::Closed;

        match self.reader.peek_one() {
            // `<!` - comment, CDATA or DOCTYPE declaration
            Ok(Some(b'!')) => match self.reader.read_bang_element(buf, &mut self.buf_position) {
                Ok(None) => Ok(Event::Eof),
                Ok(Some((bang_type, bytes))) => self.read_bang(bang_type, bytes),
                Err(e) => Err(e),
            },
            // `</` - closing tag
            Ok(Some(b'/')) => match self
                .reader
                .read_bytes_until(b'>', buf, &mut self.buf_position)
            {
                Ok(None) => Ok(Event::Eof),
                Ok(Some(bytes)) => self.read_end(bytes),
                Err(e) => Err(e),
            },
            // `<?` - processing instruction
            Ok(Some(b'?')) => match self
                .reader
                .read_bytes_until(b'>', buf, &mut self.buf_position)
            {
                Ok(None) => Ok(Event::Eof),
                Ok(Some(bytes)) => self.read_question_mark(bytes),
                Err(e) => Err(e),
            },
            // `<...` - opening or self-closed tag
            Ok(Some(_)) => match self.reader.read_element(buf, &mut self.buf_position) {
                Ok(None) => Ok(Event::Eof),
                Ok(Some(bytes)) => self.read_start(bytes),
                Err(e) => Err(e),
            },
            Ok(None) => Ok(Event::Eof),
            Err(e) => Err(e),
        }
    }

    /// reads `BytesElement` starting with a `!`,
    /// return `Comment`, `CData` or `DocType` event
    fn read_bang<'b>(&mut self, bang_type: BangType, buf: &'b [u8]) -> Result<Event<'b>> {
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
                Ok(Event::CData(BytesCData::new(&buf[8..])))
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

    /// reads `BytesElement` starting with a `/`,
    /// if `self.check_end_names`, checks that element matches last opened element
    /// return `End` event
    fn read_end<'b>(&mut self, buf: &'b [u8]) -> Result<Event<'b>> {
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
                    let expected = &self.opened_buffer[start..];
                    if name != expected {
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

    /// reads `BytesElement` starting with a `?`,
    /// return `Decl` or `PI` event
    fn read_question_mark<'b>(&mut self, buf: &'b [u8]) -> Result<Event<'b>> {
        let len = buf.len();
        if len > 2 && buf[len - 1] == b'?' {
            if len > 5 && &buf[1..4] == b"xml" && is_whitespace(buf[4]) {
                let event = BytesDecl::from_start(BytesStart::borrowed(&buf[1..len - 1], 3));

                // Try getting encoding from the declaration event
                #[cfg(feature = "encoding")]
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
    fn read_start<'b>(&mut self, buf: &'b [u8]) -> Result<Event<'b>> {
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
}

impl Reader<BufReader<File>> {
    /// Creates an XML reader from a file path.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path).map_err(Error::Io)?;
        let reader = BufReader::new(file);
        Ok(Self::from_reader(reader))
    }
}

impl<'a> Reader<&'a [u8]> {
    /// Creates an XML reader from a string slice.
    pub fn from_str(s: &'a str) -> Self {
        Self::from_reader(s.as_bytes())
    }

    /// Creates an XML reader from a slice of bytes.
    pub fn from_bytes(s: &'a [u8]) -> Self {
        Self::from_reader(s)
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
                Ok(Event::End(ref e)) if e.name().as_ref() == end => {
                    if depth == 0 {
                        return Ok(());
                    }
                    depth -= 1;
                }
                Ok(Event::Start(ref e)) if e.name().as_ref() == end => depth += 1,
                Err(e) => return Err(e),
                Ok(Event::Eof) => {
                    return Err(Error::UnexpectedEof(format!("</{:?}>", from_utf8(end))));
                }
                _ => (),
            }
        }
    }
}

/// Represents an input for a reader that can return borrowed data.
///
/// There are two implementors of this trait: generic one that read data from
/// `Self`, copies some part of it into a provided buffer of type `B` and then
/// returns data that borrow from that buffer.
///
/// The other implementor is for `&[u8]` and instead of copying data returns
/// borrowed data from `Self` instead. This implementation allows zero-copy
/// deserialization.
///
/// # Parameters
/// - `'r`: lifetime of a buffer from which events will borrow
/// - `B`: a type of a buffer that can be used to store data read from `Self` and
///   from which events can borrow
trait XmlSource<'r, B> {
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
}

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

/// Implementation of `XmlSource` for `&[u8]` reader using a `Self` as buffer
/// that will be borrowed by events. This implementation provides a zero-copy deserialization
impl<'a> XmlSource<'a, ()> for &'a [u8] {
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

        let mut state = ReadElementState::Elem;

        if let Some((bytes, i)) = state.change(self) {
            *position += i;
            *self = &self[i..];
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

/// State machine for the [`XmlSource::read_element`]
#[derive(Clone, Copy)]
enum ReadElementState {
    /// The initial state (inside element, but outside of attribute value)
    Elem,
    /// Inside a single-quoted attribute value
    SingleQ,
    /// Inside a double-quoted attribute value
    DoubleQ,
}
impl ReadElementState {
    /// Changes state by analyzing part of input.
    /// Returns a tuple with part of chunk up to element closing symbol `>`
    /// and a position after that symbol or `None` if such symbol was not found
    #[inline(always)]
    fn change<'b>(&mut self, chunk: &'b [u8]) -> Option<(&'b [u8], usize)> {
        for i in memchr::memchr3_iter(b'>', b'\'', b'"', chunk) {
            *self = match (*self, chunk[i]) {
                // only allowed to match `>` while we are in state `Elem`
                (Self::Elem, b'>') => return Some((&chunk[..i], i + 1)),
                (Self::Elem, b'\'') => Self::SingleQ,
                (Self::Elem, b'\"') => Self::DoubleQ,

                // the only end_byte that gets us out if the same character
                (Self::SingleQ, b'\'') | (Self::DoubleQ, b'"') => Self::Elem,

                // all other bytes: no state change
                _ => *self,
            };
        }
        None
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

////////////////////////////////////////////////////////////////////////////////////////////////////

/// Decoder of byte slices to the strings. This is lightweight object that can be copied.
///
/// If feature `encoding` is enabled, this encoding taken from the `"encoding"`
/// XML declaration or assumes UTF-8, if XML has no <?xml ?> declaration, encoding
/// key is not defined or contains unknown encoding.
///
/// The library supports any UTF-8 compatible encodings that crate `encoding_rs`
/// is supported. [*UTF-16 is not supported at the present*][utf16].
///
/// If feature `encoding` is disabled, the decoder is always UTF-8 decoder:
/// any XML declarations are ignored.
///
/// [utf16]: https://github.com/tafia/quick-xml/issues/158
#[derive(Clone, Copy, Debug)]
pub struct Decoder {
    #[cfg(feature = "encoding")]
    encoding: &'static Encoding,
}

#[cfg(not(feature = "encoding"))]
impl Decoder {
    /// Decodes a UTF8 slice regardless of XML declaration and ignoring BOM if
    /// it is present in the `bytes`.
    ///
    /// Returns an error in case of malformed sequences in the `bytes`.
    ///
    /// If you instead want to use XML declared encoding, use the `encoding` feature
    #[inline]
    pub fn decode<'b>(&self, bytes: &'b [u8]) -> Result<Cow<'b, str>> {
        Ok(Cow::Borrowed(from_utf8(bytes)?))
    }

    /// Decodes a slice regardless of XML declaration with BOM removal if
    /// it is present in the `bytes`.
    ///
    /// Returns an error in case of malformed sequences in the `bytes`.
    ///
    /// If you instead want to use XML declared encoding, use the `encoding` feature
    pub fn decode_with_bom_removal<'b>(&self, bytes: &'b [u8]) -> Result<Cow<'b, str>> {
        let bytes = if bytes.starts_with(b"\xEF\xBB\xBF") {
            &bytes[3..]
        } else {
            bytes
        };
        self.decode(bytes)
    }
}

#[cfg(feature = "encoding")]
impl Decoder {
    /// Returns the `Reader`s encoding.
    ///
    /// This encoding will be used by [`decode`].
    ///
    /// [`decode`]: Self::decode
    pub fn encoding(&self) -> &'static Encoding {
        self.encoding
    }

    /// Decodes specified bytes using encoding, declared in the XML, if it was
    /// declared there, or UTF-8 otherwise, and ignoring BOM if it is present
    /// in the `bytes`.
    ///
    /// Returns an error in case of malformed sequences in the `bytes`.
    pub fn decode<'b>(&self, bytes: &'b [u8]) -> Result<Cow<'b, str>> {
        match self
            .encoding
            .decode_without_bom_handling_and_without_replacement(bytes)
        {
            None => Err(Error::NonDecodable(None)),
            Some(s) => Ok(s),
        }
    }

    /// Decodes a slice with BOM removal if it is present in the `bytes` using
    /// the reader encoding.
    ///
    /// If this method called after reading XML declaration with the `"encoding"`
    /// key, then this encoding is used, otherwise UTF-8 is used.
    ///
    /// If XML declaration is absent in the XML, UTF-8 is used.
    ///
    /// Returns an error in case of malformed sequences in the `bytes`.
    pub fn decode_with_bom_removal<'b>(&self, bytes: &'b [u8]) -> Result<Cow<'b, str>> {
        self.decode(self.remove_bom(bytes))
    }
    /// Copied from [`Encoding::decode_with_bom_removal`]
    #[inline]
    fn remove_bom<'b>(&self, bytes: &'b [u8]) -> &'b [u8] {
        use encoding_rs::*;

        if self.encoding == UTF_8 && bytes.starts_with(b"\xEF\xBB\xBF") {
            return &bytes[3..];
        }
        if self.encoding == UTF_16LE && bytes.starts_with(b"\xFF\xFE") {
            return &bytes[2..];
        }
        if self.encoding == UTF_16BE && bytes.starts_with(b"\xFE\xFF") {
            return &bytes[2..];
        }

        bytes
    }
}

/// This implementation is required for tests of other parts of the library
#[cfg(test)]
#[cfg(feature = "serialize")]
impl Decoder {
    pub(crate) fn utf8() -> Self {
        Decoder {
            #[cfg(feature = "encoding")]
            encoding: encoding_rs::UTF_8,
        }
    }

    #[cfg(feature = "encoding")]
    pub(crate) fn utf16() -> Self {
        Decoder {
            encoding: encoding_rs::UTF_16LE,
        }
    }
}

/// Automatic encoding detection of XML files based using the [recommended algorithm]
/// (https://www.w3.org/TR/xml11/#sec-guessing)
///
/// The algorithm suggests examine up to the first 4 bytes to determine encoding
/// according to the following table:
///
/// | Bytes       |Detected encoding
/// |-------------|------------------------------------------
/// |`00 00 FE FF`|UCS-4, big-endian machine (1234 order)
/// |`FF FE 00 00`|UCS-4, little-endian machine (4321 order)
/// |`00 00 FF FE`|UCS-4, unusual octet order (2143)
/// |`FE FF 00 00`|UCS-4, unusual octet order (3412)
/// |`FE FF ## ##`|UTF-16, big-endian
/// |`FF FE ## ##`|UTF-16, little-endian
/// |`EF BB BF`   |UTF-8
/// |-------------|------------------------------------------
/// |`00 00 00 3C`|UCS-4 or similar (use declared encoding to find the exact one), in big-endian (1234)
/// |`3C 00 00 00`|UCS-4 or similar (use declared encoding to find the exact one), in little-endian (4321)
/// |`00 00 3C 00`|UCS-4 or similar (use declared encoding to find the exact one), in unusual byte orders (2143)
/// |`00 3C 00 00`|UCS-4 or similar (use declared encoding to find the exact one), in unusual byte orders (3412)
/// |`00 3C 00 3F`|UTF-16 BE or ISO-10646-UCS-2 BE or similar 16-bit BE (use declared encoding to find the exact one)
/// |`3C 00 3F 00`|UTF-16 LE or ISO-10646-UCS-2 LE or similar 16-bit LE (use declared encoding to find the exact one)
/// |`3C 3F 78 6D`|UTF-8, ISO 646, ASCII, some part of ISO 8859, Shift-JIS, EUC, or any other 7-bit, 8-bit, or mixed-width encoding which ensures that the characters of ASCII have their normal positions, width, and values; the actual encoding declaration must be read to detect which of these applies, but since all of these encodings use the same bit patterns for the relevant ASCII characters, the encoding declaration itself may be read reliably
/// |`4C 6F A7 94`|EBCDIC (in some flavor; the full encoding declaration must be read to tell which code page is in use)
/// |_Other_      |UTF-8 without an encoding declaration, or else the data stream is mislabeled (lacking a required encoding declaration), corrupt, fragmentary, or enclosed in a wrapper of some kind
///
/// Because [`encoding_rs`] crate supported only subset of those encodings, only
/// supported subset are detected, which is UTF-8, UTF-16 BE and UTF-16 LE.
///
/// If encoding is detected, `Some` is returned, otherwise `None` is returned.
#[cfg(feature = "encoding")]
fn detect_encoding(bytes: &[u8]) -> Option<&'static Encoding> {
    match bytes {
        // with BOM
        _ if bytes.starts_with(&[0xFE, 0xFF]) => Some(UTF_16BE),
        _ if bytes.starts_with(&[0xFF, 0xFE]) => Some(UTF_16LE),
        _ if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) => Some(UTF_8),

        // without BOM
        _ if bytes.starts_with(&[0x00, b'<', 0x00, b'?']) => Some(UTF_16BE), // Some BE encoding, for example, UTF-16 or ISO-10646-UCS-2
        _ if bytes.starts_with(&[b'<', 0x00, b'?', 0x00]) => Some(UTF_16LE), // Some LE encoding, for example, UTF-16 or ISO-10646-UCS-2
        _ if bytes.starts_with(&[b'<', b'?', b'x', b'm']) => Some(UTF_8), // Some ASCII compatible

        _ => None,
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod test {
    macro_rules! check {
        ($buf:expr) => {
            mod read_bytes_until {
                use crate::reader::XmlSource;
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
                    use crate::reader::{BangType, XmlSource};
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
                    use crate::reader::{BangType, XmlSource};
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
                        use crate::reader::{BangType, XmlSource};
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
                        use crate::reader::{BangType, XmlSource};
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
                use crate::reader::XmlSource;
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
                    use crate::reader::XmlSource;
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
                    use crate::reader::XmlSource;
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

            /// Ensures, that no empty `Text` events are generated
            mod read_event_buffered {
                use crate::events::{BytesCData, BytesDecl, BytesEnd, BytesStart, BytesText, Event};
                use crate::reader::Reader;
                use pretty_assertions::assert_eq;

                #[test]
                fn start_text() {
                    let mut reader = Reader::from_str("bom");

                    assert_eq!(
                        reader.read_event_buffered($buf).unwrap(),
                        Event::StartText(BytesText::from_escaped(b"bom".as_ref()).into())
                    );
                }

                #[test]
                fn declaration() {
                    let mut reader = Reader::from_str("<?xml ?>");

                    assert_eq!(
                        reader.read_event_buffered($buf).unwrap(),
                        Event::Decl(BytesDecl::from_start(BytesStart::borrowed(b"xml ", 3)))
                    );
                }

                #[test]
                fn doctype() {
                    let mut reader = Reader::from_str("<!DOCTYPE x>");

                    assert_eq!(
                        reader.read_event_buffered($buf).unwrap(),
                        Event::DocType(BytesText::from_escaped(b"x".as_ref()))
                    );
                }

                #[test]
                fn processing_instruction() {
                    let mut reader = Reader::from_str("<?xml-stylesheet?>");

                    assert_eq!(
                        reader.read_event_buffered($buf).unwrap(),
                        Event::PI(BytesText::from_escaped(b"xml-stylesheet".as_ref()))
                    );
                }

                #[test]
                fn start() {
                    let mut reader = Reader::from_str("<tag>");

                    assert_eq!(
                        reader.read_event_buffered($buf).unwrap(),
                        Event::Start(BytesStart::borrowed_name(b"tag"))
                    );
                }

                #[test]
                fn end() {
                    let mut reader = Reader::from_str("</tag>");
                    // Because we expect invalid XML, do not check that
                    // the end name paired with the start name
                    reader.check_end_names(false);

                    assert_eq!(
                        reader.read_event_buffered($buf).unwrap(),
                        Event::End(BytesEnd::borrowed(b"tag"))
                    );
                }

                #[test]
                fn empty() {
                    let mut reader = Reader::from_str("<tag/>");

                    assert_eq!(
                        reader.read_event_buffered($buf).unwrap(),
                        Event::Empty(BytesStart::borrowed_name(b"tag"))
                    );
                }

                /// Text event cannot be generated without preceding event of another type
                #[test]
                fn text() {
                    let mut reader = Reader::from_str("<tag/>text");

                    assert_eq!(
                        reader.read_event_buffered($buf).unwrap(),
                        Event::Empty(BytesStart::borrowed_name(b"tag"))
                    );

                    assert_eq!(
                        reader.read_event_buffered($buf).unwrap(),
                        Event::Text(BytesText::from_escaped(b"text".as_ref()))
                    );
                }

                #[test]
                fn cdata() {
                    let mut reader = Reader::from_str("<![CDATA[]]>");

                    assert_eq!(
                        reader.read_event_buffered($buf).unwrap(),
                        Event::CData(BytesCData::from_str(""))
                    );
                }

                #[test]
                fn comment() {
                    let mut reader = Reader::from_str("<!---->");

                    assert_eq!(
                        reader.read_event_buffered($buf).unwrap(),
                        Event::Comment(BytesText::from_escaped(b"".as_ref()))
                    );
                }

                #[test]
                fn eof() {
                    let mut reader = Reader::from_str("");

                    assert_eq!(
                        reader.read_event_buffered($buf).unwrap(),
                        Event::Eof
                    );
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

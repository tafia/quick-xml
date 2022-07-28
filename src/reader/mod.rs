//! A module to handle `Reader`

use std::str::from_utf8;

#[cfg(feature = "encoding")]
use encoding_rs::{Encoding, UTF_8};

#[cfg(feature = "encoding")]
use crate::encoding::detect_encoding;
use crate::encoding::Decoder;
use crate::errors::{Error, Result};
use crate::events::{BytesCData, BytesDecl, BytesEnd, BytesStart, BytesText, Event};

use memchr;

macro_rules! configure_methods {
    ($($holder:ident)?) => {
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
        /// [`Empty`]: Event::Empty
        /// [`Start`]: Event::Start
        /// [`End`]: Event::End
        pub fn expand_empty_elements(&mut self, val: bool) -> &mut Self {
            self $(.$holder)? .expand_empty_elements = val;
            self
        }

        /// Changes whether whitespace before and after character data should be removed.
        ///
        /// When set to `true`, all [`Text`] events are trimmed. If they are empty, no event will be
        /// pushed.
        ///
        /// (`false` by default)
        ///
        /// [`Text`]: Event::Text
        pub fn trim_text(&mut self, val: bool) -> &mut Self {
            self $(.$holder)? .trim_text_start = val;
            self $(.$holder)? .trim_text_end = val;
            self
        }

        /// Changes whether whitespace after character data should be removed.
        ///
        /// When set to `true`, trailing whitespace is trimmed in [`Text`] events.
        ///
        /// (`false` by default)
        ///
        /// [`Text`]: Event::Text
        pub fn trim_text_end(&mut self, val: bool) -> &mut Self {
            self $(.$holder)? .trim_text_end = val;
            self
        }

        /// Changes whether trailing whitespaces after the markup name are trimmed in closing tags
        /// `</a >`.
        ///
        /// If true the emitted [`End`] event is stripped of trailing whitespace after the markup name.
        ///
        /// Note that if set to `false` and `check_end_names` is true the comparison of markup names is
        /// going to fail erroneously if a closing tag contains trailing whitespaces.
        ///
        /// (`true` by default)
        ///
        /// [`End`]: Event::End
        pub fn trim_markup_names_in_closing_tags(&mut self, val: bool) -> &mut Self {
            self $(.$holder)? .trim_markup_names_in_closing_tags = val;
            self
        }

        /// Changes whether mismatched closing tag names should be detected.
        ///
        /// Note, that start and end tags [should match literally][spec], they cannot
        /// have different prefixes even if both prefixes resolve to the same namespace.
        /// The XML
        ///
        /// ```xml
        /// <outer xmlns="namespace" xmlns:p="namespace">
        /// </p:outer>
        /// ```
        ///
        /// is not valid, even though semantically the start tag is the same as the
        /// end tag. The reason is that namespaces are an extension of the original
        /// XML specification (without namespaces) and it should be backward-compatible.
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
        /// [spec]: https://www.w3.org/TR/xml11/#dt-etag
        /// [`End`]: Event::End
        pub fn check_end_names(&mut self, val: bool) -> &mut Self {
            self $(.$holder)? .check_end_names = val;
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
        /// [`Comment`]: Event::Comment
        pub fn check_comments(&mut self, val: bool) -> &mut Self {
            self $(.$holder)? .check_comments = val;
            self
        }
    };
}

mod buffered_reader;
mod ns_reader;
mod slice_reader;

pub use ns_reader::NsReader;

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

/// A reference to an encoding together with information about how it was retrieved.
///
/// The state transition diagram:
///
/// ```mermaid
/// flowchart LR
///   Implicit    -- from_str       --> Explicit
///   Implicit    -- BOM            --> BomDetected
///   Implicit    -- "encoding=..." --> XmlDetected
///   BomDetected -- "encoding=..." --> XmlDetected
/// ```
#[cfg(feature = "encoding")]
#[derive(Clone, Copy)]
enum EncodingRef {
    /// Encoding was implicitly assumed to have a specified value. It can be refined
    /// using BOM or by the XML declaration event (`<?xml encoding=... ?>`)
    Implicit(&'static Encoding),
    /// Encoding was explicitly set to the desired value. It cannot be changed
    /// nor by BOM, nor by parsing XML declaration (`<?xml encoding=... ?>`)
    Explicit(&'static Encoding),
    /// Encoding was detected from a byte order mark (BOM) or by the first bytes
    /// of the content. It can be refined by the XML declaration event (`<?xml encoding=... ?>`)
    BomDetected(&'static Encoding),
    /// Encoding was detected using XML declaration event (`<?xml encoding=... ?>`).
    /// It can no longer change
    XmlDetected(&'static Encoding),
}
#[cfg(feature = "encoding")]
impl EncodingRef {
    #[inline]
    fn encoding(&self) -> &'static Encoding {
        match self {
            Self::Implicit(e) => e,
            Self::Explicit(e) => e,
            Self::BomDetected(e) => e,
            Self::XmlDetected(e) => e,
        }
    }
    #[inline]
    fn can_be_refined(&self) -> bool {
        match self {
            Self::Implicit(_) | Self::BomDetected(_) => true,
            Self::Explicit(_) | Self::XmlDetected(_) => false,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// A low level encoding-agnostic XML event reader.
///
/// Consumes bytes and streams XML [`Event`]s.
///
/// This reader does not manage namespace declarations and not able to resolve
/// prefixes. If you want these features, use the [`NsReader`].
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
///              </tag1>"#;
/// let mut reader = Reader::from_str(xml);
/// reader.trim_text(true);
///
/// let mut count = 0;
/// let mut txt = Vec::new();
/// let mut buf = Vec::new();
///
/// // The `Reader` does not implement `Iterator` because it outputs borrowed data (`Cow`s)
/// loop {
///     // NOTE: this is the generic case when we don't know about the input BufRead.
///     // when the input is a &str or a &[u8], we don't actually need to use another
///     // buffer, we could directly call `reader.read_event()`
///     match reader.read_event_into(&mut buf) {
///         Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
///         // exits the loop when reaching end of file
///         Ok(Event::Eof) => break,
///
///         Ok(Event::Start(e)) => {
///             match e.name().as_ref() {
///                 b"tag1" => println!("attributes values: {:?}",
///                                     e.attributes().map(|a| a.unwrap().value)
///                                     .collect::<Vec<_>>()),
///                 b"tag2" => count += 1,
///                 _ => (),
///             }
///         }
///         Ok(Event::Text(e)) => txt.push(e.unescape().unwrap().into_owned()),
///
///         // There are several other `Event`s we do not consider here
///         _ => (),
///     }
///     // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
///     buf.clear();
/// }
/// ```
///
/// [`NsReader`]: crate::reader::NsReader
#[derive(Clone)]
pub struct Reader<R> {
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

    #[cfg(feature = "encoding")]
    /// Reference to the encoding used to read an XML
    encoding: EncodingRef,
}

/// Builder methods
impl<R> Reader<R> {
    /// Creates a `Reader` that reads from a given reader.
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

            #[cfg(feature = "encoding")]
            encoding: EncodingRef::Implicit(UTF_8),
        }
    }

    configure_methods!();
}

/// Getters
impl<R> Reader<R> {
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
    ///     match reader.read_event_into(&mut buf) {
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
            encoding: self.encoding.encoding(),
        }
    }
}

/// Parse methods, independent from a way of reading data
impl<R> Reader<R> {
    /// Trims whitespaces from `bytes`, if required, and returns a [`StartText`]
    /// or a [`Text`] event. When [`StartText`] is returned, the method can change
    /// the encoding of the reader, detecting it from the beginning of the stream.
    ///
    /// # Parameters
    /// - `bytes`: data from the start of stream to the first `<` or from `>` to `<`
    /// - `first`: if `true`, then this is the first call of that function,
    ///   i. e. data from the start of stream and [`StartText`] will be returned,
    ///   otherwise [`Text`] will be returned
    ///
    /// [`StartText`]: Event::StartText
    /// [`Text`]: Event::Text
    fn read_text<'b>(&mut self, bytes: &'b [u8], first: bool) -> Result<Event<'b>> {
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
            Event::StartText(BytesText::wrap(content, self.decoder()).into())
        } else {
            Event::Text(BytesText::wrap(content, self.decoder()))
        })
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
                Ok(Event::Comment(BytesText::wrap(
                    &buf[3..len - 2],
                    self.decoder(),
                )))
            }
            BangType::CData if uncased_starts_with(buf, b"![CDATA[") => {
                Ok(Event::CData(BytesCData::wrap(&buf[8..], self.decoder())))
            }
            BangType::DocType if uncased_starts_with(buf, b"!DOCTYPE") => {
                let start = buf[8..]
                    .iter()
                    .position(|b| !is_whitespace(*b))
                    .unwrap_or_else(|| len - 8);
                debug_assert!(start < len - 8, "DocType must have a name");
                Ok(Event::DocType(BytesText::wrap(
                    &buf[8 + start..],
                    self.decoder(),
                )))
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
                        Ok(Event::End(BytesEnd::wrap(name.into())))
                    }
                }
                None => mismatch_err(b"", &buf[1..], &mut self.buf_position),
            }
        } else {
            Ok(Event::End(BytesEnd::wrap(name.into())))
        }
    }

    /// reads `BytesElement` starting with a `?`,
    /// return `Decl` or `PI` event
    fn read_question_mark<'b>(&mut self, buf: &'b [u8]) -> Result<Event<'b>> {
        let len = buf.len();
        if len > 2 && buf[len - 1] == b'?' {
            if len > 5 && &buf[1..4] == b"xml" && is_whitespace(buf[4]) {
                let event = BytesDecl::from_start(BytesStart::wrap(&buf[1..len - 1], 3));

                // Try getting encoding from the declaration event
                #[cfg(feature = "encoding")]
                if self.encoding.can_be_refined() {
                    if let Some(encoding) = event.encoder() {
                        self.encoding = EncodingRef::XmlDetected(encoding);
                    }
                }

                Ok(Event::Decl(event))
            } else {
                Ok(Event::PI(BytesText::wrap(&buf[1..len - 1], self.decoder())))
            }
        } else {
            self.buf_position -= len;
            Err(Error::UnexpectedEof("XmlDecl".to_string()))
        }
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
                Ok(Event::Start(BytesStart::wrap(&buf[..len - 1], end)))
            } else {
                Ok(Event::Empty(BytesStart::wrap(&buf[..len - 1], end)))
            }
        } else {
            if self.check_end_names {
                self.opened_starts.push(self.opened_buffer.len());
                self.opened_buffer.extend(&buf[..name_end]);
            }
            Ok(Event::Start(BytesStart::wrap(buf, name_end)))
        }
    }

    #[inline]
    fn close_expanded_empty(&mut self) -> Result<Event<'static>> {
        self.tag_state = TagState::Closed;
        let name = self
            .opened_buffer
            .split_off(self.opened_starts.pop().unwrap());
        Ok(Event::End(BytesEnd::wrap(name.into())))
    }
}

/// Private sync reading methods
impl<R> Reader<R> {
    /// Read text into the given buffer, and return an event that borrows from
    /// either that buffer or from the input itself, based on the type of the
    /// reader.
    fn read_event_impl<'i, B>(&mut self, buf: B) -> Result<Event<'i>>
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
            return self.read_event_impl(buf);
        }

        match self
            .reader
            .read_bytes_until(b'<', buf, &mut self.buf_position)
        {
            Ok(Some(bytes)) => self.read_text(bytes, first),
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
}

////////////////////////////////////////////////////////////////////////////////////////////////////

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

#[cfg(test)]
mod test {
    /// Checks the internal implementation of the various reader methods
    macro_rules! check {
        ($buf:expr) => {
            mod read_bytes_until {
                use super::*;
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
                use super::*;

                /// Checks that reading CDATA content works correctly
                mod cdata {
                    use super::*;
                    use crate::errors::Error;
                    use crate::reader::BangType;
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
                    use super::*;
                    use crate::errors::Error;
                    use crate::reader::BangType;
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
                    use super::*;

                    mod uppercase {
                        use super::*;
                        use crate::errors::Error;
                        use crate::reader::BangType;
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
                        use super::*;
                        use crate::errors::Error;
                        use crate::reader::BangType;
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
                use super::*;
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
                    use super::*;
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
                    use super::*;
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
            mod read_event_impl {
                use crate::events::{BytesCData, BytesDecl, BytesEnd, BytesStart, BytesText, Event};
                use crate::reader::Reader;
                use pretty_assertions::assert_eq;

                #[test]
                fn start_text() {
                    let mut reader = Reader::from_str("bom");

                    assert_eq!(
                        reader.read_event_impl($buf).unwrap(),
                        Event::StartText(BytesText::from_escaped("bom").into())
                    );
                }

                #[test]
                fn declaration() {
                    let mut reader = Reader::from_str("<?xml ?>");

                    assert_eq!(
                        reader.read_event_impl($buf).unwrap(),
                        Event::Decl(BytesDecl::from_start(BytesStart::from_content("xml ", 3)))
                    );
                }

                #[test]
                fn doctype() {
                    let mut reader = Reader::from_str("<!DOCTYPE x>");

                    assert_eq!(
                        reader.read_event_impl($buf).unwrap(),
                        Event::DocType(BytesText::from_escaped("x"))
                    );
                }

                #[test]
                fn processing_instruction() {
                    let mut reader = Reader::from_str("<?xml-stylesheet?>");

                    assert_eq!(
                        reader.read_event_impl($buf).unwrap(),
                        Event::PI(BytesText::from_escaped("xml-stylesheet"))
                    );
                }

                #[test]
                fn start() {
                    let mut reader = Reader::from_str("<tag>");

                    assert_eq!(
                        reader.read_event_impl($buf).unwrap(),
                        Event::Start(BytesStart::new("tag"))
                    );
                }

                #[test]
                fn end() {
                    let mut reader = Reader::from_str("</tag>");
                    // Because we expect invalid XML, do not check that
                    // the end name paired with the start name
                    reader.check_end_names(false);

                    assert_eq!(
                        reader.read_event_impl($buf).unwrap(),
                        Event::End(BytesEnd::new("tag"))
                    );
                }

                #[test]
                fn empty() {
                    let mut reader = Reader::from_str("<tag/>");

                    assert_eq!(
                        reader.read_event_impl($buf).unwrap(),
                        Event::Empty(BytesStart::new("tag"))
                    );
                }

                /// Text event cannot be generated without preceding event of another type
                #[test]
                fn text() {
                    let mut reader = Reader::from_str("<tag/>text");

                    assert_eq!(
                        reader.read_event_impl($buf).unwrap(),
                        Event::Empty(BytesStart::new("tag"))
                    );

                    assert_eq!(
                        reader.read_event_impl($buf).unwrap(),
                        Event::Text(BytesText::from_escaped("text"))
                    );
                }

                #[test]
                fn cdata() {
                    let mut reader = Reader::from_str("<![CDATA[]]>");

                    assert_eq!(
                        reader.read_event_impl($buf).unwrap(),
                        Event::CData(BytesCData::new(""))
                    );
                }

                #[test]
                fn comment() {
                    let mut reader = Reader::from_str("<!---->");

                    assert_eq!(
                        reader.read_event_impl($buf).unwrap(),
                        Event::Comment(BytesText::from_escaped(""))
                    );
                }

                #[test]
                fn eof() {
                    let mut reader = Reader::from_str("");

                    assert_eq!(
                        reader.read_event_impl($buf).unwrap(),
                        Event::Eof
                    );
                }
            }
        };
    }

    // Export a macro for the child modules:
    // - buffered_reader
    // - slice_reader
    pub(super) use check;
}

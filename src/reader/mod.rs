//! Contains high-level interface for a pull-based XML parser.

#[cfg(feature = "encoding")]
use encoding_rs::Encoding;
use std::ops::Range;

use crate::encoding::Decoder;
use crate::reader::state::ReaderState;

/// A struct that holds a parser configuration.
///
/// Current parser configuration can be retrieved by calling [`Reader::config()`]
/// and changed by changing properties of the object returned by a call to
/// [`Reader::config_mut()`].
///
/// [`Reader::config()`]: crate::reader::Reader::config
/// [`Reader::config_mut()`]: crate::reader::Reader::config_mut
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde-types", derive(serde::Deserialize, serde::Serialize))]
#[non_exhaustive]
pub struct Config {
    /// Whether comments should be validated. If enabled, in case of invalid comment
    /// [`Error::IllFormed(DoubleHyphenInComment)`] is returned from read methods.
    ///
    /// When set to `true`, every [`Comment`] event will be checked for not
    /// containing `--`, which [is not allowed] in XML comments. Most of the time
    /// we don't want comments at all so we don't really care about comment
    /// correctness, thus the default value is `false` to improve performance.
    ///
    /// Default: `false`
    ///
    /// [`Error::IllFormed(DoubleHyphenInComment)`]: crate::errors::IllFormedError::DoubleHyphenInComment
    /// [`Comment`]: crate::events::Event::Comment
    /// [is not allowed]: https://www.w3.org/TR/xml11/#sec-comments
    pub check_comments: bool,

    /// Whether mismatched closing tag names should be detected. If enabled, in
    /// case of mismatch the [`Error::IllFormed(MismatchedEndTag)`] is returned from
    /// read methods.
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
    /// When set to `false`, it won't check if a closing tag matches the corresponding
    /// opening tag. For example, `<mytag></different_tag>` will be permitted.
    ///
    /// If the XML is known to be sane (already processed, etc.) this saves extra time.
    ///
    /// Note that the emitted [`End`] event will not be modified if this is disabled,
    /// ie. it will contain the data of the mismatched end tag.
    ///
    /// Note, that setting this to `true` will lead to additional allocates that
    /// needed to store tag name for an [`End`] event. However if [`expand_empty_elements`]
    /// is also set, only one additional allocation will be performed that support
    /// both these options.
    ///
    /// Default: `true`
    ///
    /// [`Error::IllFormed(MismatchedEndTag)`]: crate::errors::IllFormedError::MismatchedEndTag
    /// [spec]: https://www.w3.org/TR/xml11/#dt-etag
    /// [`End`]: crate::events::Event::End
    /// [`expand_empty_elements`]: Self::expand_empty_elements
    pub check_end_names: bool,

    /// Whether empty elements should be split into an `Open` and a `Close` event.
    ///
    /// When set to `true`, all [`Empty`] events produced by a self-closing tag
    /// like `<tag/>` are expanded into a [`Start`] event followed by an [`End`]
    /// event. When set to `false` (the default), those tags are represented by
    /// an [`Empty`] event instead.
    ///
    /// Note, that setting this to `true` will lead to additional allocates that
    /// needed to store tag name for an [`End`] event. However if [`check_end_names`]
    /// is also set, only one additional allocation will be performed that support
    /// both these options.
    ///
    /// Default: `false`
    ///
    /// [`Empty`]: crate::events::Event::Empty
    /// [`Start`]: crate::events::Event::Start
    /// [`End`]: crate::events::Event::End
    /// [`check_end_names`]: Self::check_end_names
    pub expand_empty_elements: bool,

    /// Whether trailing whitespace after the markup name are trimmed in closing
    /// tags `</a >`.
    ///
    /// If `true` the emitted [`End`] event is stripped of trailing whitespace
    /// after the markup name.
    ///
    /// Note that if set to `false` and [`check_end_names`] is `true` the comparison
    /// of markup names is going to fail erroneously if a closing tag contains
    /// trailing whitespace.
    ///
    /// Default: `true`
    ///
    /// [`End`]: crate::events::Event::End
    /// [`check_end_names`]: Self::check_end_names
    pub trim_markup_names_in_closing_tags: bool,

    /// Whether whitespace before character data should be removed.
    ///
    /// When set to `true`, leading whitespace is trimmed in [`Text`] events.
    /// If after that the event is empty it will not be pushed.
    ///
    /// Default: `false`
    ///
    /// <div style="background:rgba(80, 240, 100, 0.20);padding:0.75em;">
    ///
    /// WARNING: With this option every text events will be trimmed which is
    /// incorrect behavior when text events delimited by comments, processing
    /// instructions or CDATA sections. To correctly trim data manually apply
    /// [`BytesText::inplace_trim_start`] and [`BytesText::inplace_trim_end`]
    /// only to necessary events.
    /// </div>
    ///
    /// [`Text`]: crate::events::Event::Text
    /// [`BytesText::inplace_trim_start`]: crate::events::BytesText::inplace_trim_start
    /// [`BytesText::inplace_trim_end`]: crate::events::BytesText::inplace_trim_end
    pub trim_text_start: bool,

    /// Whether whitespace after character data should be removed.
    ///
    /// When set to `true`, trailing whitespace is trimmed in [`Text`] events.
    /// If after that the event is empty it will not be pushed.
    ///
    /// Default: `false`
    ///
    /// <div style="background:rgba(80, 240, 100, 0.20);padding:0.75em;">
    ///
    /// WARNING: With this option every text events will be trimmed which is
    /// incorrect behavior when text events delimited by comments, processing
    /// instructions or CDATA sections. To correctly trim data manually apply
    /// [`BytesText::inplace_trim_start`] and [`BytesText::inplace_trim_end`]
    /// only to necessary events.
    /// </div>
    ///
    /// [`Text`]: crate::events::Event::Text
    /// [`BytesText::inplace_trim_start`]: crate::events::BytesText::inplace_trim_start
    /// [`BytesText::inplace_trim_end`]: crate::events::BytesText::inplace_trim_end
    pub trim_text_end: bool,
}

impl Config {
    /// Set both [`trim_text_start`] and [`trim_text_end`] to the same value.
    ///
    /// <div style="background:rgba(80, 240, 100, 0.20);padding:0.75em;">
    ///
    /// WARNING: With this option every text events will be trimmed which is
    /// incorrect behavior when text events delimited by comments, processing
    /// instructions or CDATA sections. To correctly trim data manually apply
    /// [`BytesText::inplace_trim_start`] and [`BytesText::inplace_trim_end`]
    /// only to necessary events.
    /// </div>
    ///
    /// [`trim_text_start`]: Self::trim_text_start
    /// [`trim_text_end`]: Self::trim_text_end
    /// [`BytesText::inplace_trim_start`]: crate::events::BytesText::inplace_trim_start
    /// [`BytesText::inplace_trim_end`]: crate::events::BytesText::inplace_trim_end
    #[inline]
    pub fn trim_text(&mut self, trim: bool) {
        self.trim_text_start = trim;
        self.trim_text_end = trim;
    }

    /// Turn on or off all checks for well-formedness. Currently it is that settings:
    /// - [`check_comments`](Self::check_comments)
    /// - [`check_end_names`](Self::check_end_names)
    #[inline]
    pub fn enable_all_checks(&mut self, enable: bool) {
        self.check_comments = enable;
        self.check_end_names = enable;
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            check_comments: false,
            check_end_names: true,
            expand_empty_elements: false,
            trim_markup_names_in_closing_tags: true,
            trim_text_start: false,
            trim_text_end: false,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

macro_rules! read_event_impl {
    (
        $self:ident, $buf:ident
        $(, $await:ident)?
    ) => {{
        dbg!("===============================================================");
        if let Some(end) = $self.state.pending_end() {
            return Ok(end);
        }
        // Content in buffer before call is not a part of next event
        let start = $buf.len();
        let offset = $self.state.offset;
        loop {
            dbg!("--------------------------------");
            break match dbg!($self.reader.fill_buf() $(.$await)?) {
                Ok(bytes) if bytes.is_empty() => {
                    let content = &$buf[start..];
                    if content.is_empty() {
                        Ok(Event::Eof)
                    } else
                    if let Err(error) = dbg!($self.state.parser.finish()) {
                        $self.state.last_error_offset = offset;
                        Err(Error::Syntax(error))
                    } else {
                        // Content already trimmed, because we do not put whitespaces
                        // to the buffer at all if they should be trimmed
                        Ok(Event::Text(BytesText::wrap(content, $self.decoder())))
                    }
                }
                Ok(bytes) => match dbg!($self.state.parse_into(bytes, $buf))? {
                    ParseOutcome::Consume(offset, result) => {
                        $self.reader.consume(offset);
                        $self.state.make_event(result, &$buf[start..])
                    }
                    ParseOutcome::ConsumeAndEmitText(offset) => {
                        $self.reader.consume(offset);
                        Ok(Event::Text(BytesText::wrap(&$buf[start..], $self.decoder())))
                    }
                    ParseOutcome::ConsumeAndContinue(offset) => {
                        $self.reader.consume(offset);
                        continue;
                    }
                },
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => {
                    $self.state.last_error_offset = $self.state.offset;
                    Err(Error::Io(e.into()))
                }
            };
        }
    }};
}

/// Generalization of `read_to_end` method for buffered and borrowed readers
macro_rules! read_to_end {
    (
        $self:expr, $end:expr, $buf:expr,
        $read_event:ident,
        // Code block that performs clearing of internal buffer after read of each event
        $clear:block
        $(, $await:ident)?
    ) => {{
        let start = $self.buffer_position();
        let mut depth = 0;
        loop {
            $clear
            let end = $self.buffer_position();
            match $self.$read_event($buf) $(.$await)? {
                Err(e) => return Err(e),

                Ok(Event::Start(e)) if e.name() == $end => depth += 1,
                Ok(Event::End(e)) if e.name() == $end => {
                    if depth == 0 {
                        break start..end;
                    }
                    depth -= 1;
                }
                Ok(Event::Eof) => return Err(Error::missed_end($end, $self.decoder())),
                _ => (),
            }
        }
    }};
}

#[cfg(feature = "async-tokio")]
mod async_tokio;
mod buffered_reader;
mod ns_reader;
mod slice_reader;
mod state;

pub use ns_reader::NsReader;

/// Range of input in bytes, that corresponds to some piece of XML
pub type Span = Range<usize>;

////////////////////////////////////////////////////////////////////////////////////////////////////

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
#[derive(Clone, Copy, Debug)]
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
/// use quick_xml::events::Event;
/// use quick_xml::reader::Reader;
///
/// let xml = r#"<tag1 att1 = "test">
///                 <tag2><!--Test comment-->Test</tag2>
///                 <tag2>Test 2</tag2>
///              </tag1>"#;
/// let mut reader = Reader::from_str(xml);
/// reader.config_mut().trim_text(true);
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
/// [`Event`]: crate::events::Event
/// [`NsReader`]: crate::reader::NsReader
#[derive(Clone)]
pub struct Reader<R> {
    /// Source of data for parse
    reader: R,
    /// Configuration and current parse state
    state: ReaderState,
}

/// Builder methods
impl<R> Reader<R> {
    /// Creates a `Reader` that reads from a given reader.
    pub fn from_reader(reader: R) -> Self {
        Self {
            reader,
            state: ReaderState::default(),
        }
    }

    /// Returns reference to the parser configuration
    pub fn config(&self) -> &Config {
        &self.state.config
    }

    /// Returns mutable reference to the parser configuration
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.state.config
    }
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
    /// use quick_xml::events::Event;
    /// use quick_xml::reader::Reader;
    ///
    /// let xml = r#"<tag1 att1 = "test">
    ///                 <tag2><!--Test comment-->Test</tag2>
    ///                 <tag3>Test 2</tag3>
    ///              </tag1>"#;
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
    pub fn buffer_position(&self) -> usize {
        self.state.offset
    }

    /// Gets the last error byte position in the input data. If there is no errors
    /// yet, returns `0`.
    ///
    /// Unlike `buffer_position` it will point to the place where it is rational
    /// to report error to the end user. For example, all [`SyntaxError`]s are
    /// reported when the parser sees EOF inside of some kind of markup. The
    /// `buffer_position()` will point to the last byte of input which is not
    /// very useful. `error_position()` will point to the start of corresponding
    /// markup element (i. e. to the `<` character).
    ///
    /// This position is always `<= buffer_position()`.
    pub fn error_position(&self) -> usize {
        self.state.last_error_offset
    }

    /// Get the decoder, used to decode bytes, read by this reader, to the strings.
    ///
    /// If [`encoding`] feature is enabled, the used encoding may change after
    /// parsing the XML declaration, otherwise encoding is fixed to UTF-8.
    ///
    /// If [`encoding`] feature is enabled and no encoding is specified in declaration,
    /// defaults to UTF-8.
    ///
    /// [`encoding`]: ../index.html#encoding
    #[inline]
    pub fn decoder(&self) -> Decoder {
        self.state.decoder()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// A function to check whether the byte is a whitespace (blank, new line, carriage return or tab)
#[inline]
pub(crate) const fn is_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\r' | b'\n' | b'\t')
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod test {
    /// Checks the internal implementation of the various reader methods
    macro_rules! check {
        (
            #[$test:meta]
            $read_event:ident,
            // constructor of the buffer to which read data will stored
            $buf:expr
            $(, $async:ident, $await:ident)?
        ) => {
            /// Ensures, that no empty `Text` events are generated
            mod $read_event {
                use crate::events::{BytesCData, BytesDecl, BytesEnd, BytesStart, BytesText, Event};
                use crate::reader::Reader;
                use pretty_assertions::assert_eq;

                /// When `encoding` feature is enabled, encoding should be detected
                /// from BOM (UTF-8) and BOM should be stripped.
                ///
                /// When `encoding` feature is disabled, UTF-8 is assumed and BOM
                /// character should be stripped for consistency
                #[$test]
                $($async)? fn bom_from_reader() {
                    let mut reader = Reader::from_reader("\u{feff}\u{feff}".as_bytes());

                    assert_eq!(
                        reader.$read_event($buf) $(.$await)? .unwrap(),
                        Event::Text(BytesText::from_escaped("\u{feff}"))
                    );

                    assert_eq!(
                        reader.$read_event($buf) $(.$await)? .unwrap(),
                        Event::Eof
                    );
                }

                /// When parsing from &str, encoding is fixed (UTF-8), so
                /// - when `encoding` feature is disabled, the behavior the
                ///   same as in `bom_from_reader` text
                /// - when `encoding` feature is enabled, the behavior should
                ///   stay consistent, so the first BOM character is stripped
                #[$test]
                $($async)? fn bom_from_str() {
                    let mut reader = Reader::from_str("\u{feff}\u{feff}");

                    assert_eq!(
                        reader.$read_event($buf) $(.$await)? .unwrap(),
                        Event::Text(BytesText::from_escaped("\u{feff}"))
                    );

                    assert_eq!(
                        reader.$read_event($buf) $(.$await)? .unwrap(),
                        Event::Eof
                    );
                }

                #[$test]
                $($async)? fn declaration() {
                    let mut reader = Reader::from_str("<?xml ?>");

                    assert_eq!(
                        reader.$read_event($buf) $(.$await)? .unwrap(),
                        Event::Decl(BytesDecl::from_start(BytesStart::from_content("xml ", 3)))
                    );
                }

                #[$test]
                $($async)? fn doctype() {
                    let mut reader = Reader::from_str("<!DOCTYPE x>");

                    assert_eq!(
                        reader.$read_event($buf) $(.$await)? .unwrap(),
                        Event::DocType(BytesText::from_escaped("x"))
                    );
                }

                #[$test]
                $($async)? fn processing_instruction() {
                    let mut reader = Reader::from_str("<?xml-stylesheet?>");

                    assert_eq!(
                        reader.$read_event($buf) $(.$await)? .unwrap(),
                        Event::PI(BytesText::from_escaped("xml-stylesheet"))
                    );
                }

                /// Lone closing tags are not allowed, so testing it together with start tag
                #[$test]
                $($async)? fn start_and_end() {
                    let mut reader = Reader::from_str("<tag></tag>");

                    assert_eq!(
                        reader.$read_event($buf) $(.$await)? .unwrap(),
                        Event::Start(BytesStart::new("tag"))
                    );

                    assert_eq!(
                        reader.$read_event($buf) $(.$await)? .unwrap(),
                        Event::End(BytesEnd::new("tag"))
                    );
                }

                #[$test]
                $($async)? fn empty() {
                    let mut reader = Reader::from_str("<tag/>");

                    assert_eq!(
                        reader.$read_event($buf) $(.$await)? .unwrap(),
                        Event::Empty(BytesStart::new("tag"))
                    );
                }

                #[$test]
                $($async)? fn text() {
                    let mut reader = Reader::from_str("text");

                    assert_eq!(
                        reader.$read_event($buf) $(.$await)? .unwrap(),
                        Event::Text(BytesText::from_escaped("text"))
                    );
                }

                #[$test]
                $($async)? fn cdata() {
                    let mut reader = Reader::from_str("<![CDATA[]]>");

                    assert_eq!(
                        reader.$read_event($buf) $(.$await)? .unwrap(),
                        Event::CData(BytesCData::new(""))
                    );
                }

                #[$test]
                $($async)? fn comment() {
                    let mut reader = Reader::from_str("<!---->");

                    assert_eq!(
                        reader.$read_event($buf) $(.$await)? .unwrap(),
                        Event::Comment(BytesText::from_escaped(""))
                    );
                }

                #[$test]
                $($async)? fn eof() {
                    let mut reader = Reader::from_str("");

                    assert_eq!(
                        reader.$read_event($buf) $(.$await)? .unwrap(),
                        Event::Eof
                    );
                }
            }
        };
    }

    /// Tests for https://github.com/tafia/quick-xml/issues/469
    macro_rules! small_buffers {
        (
            #[$test:meta]
            $read_event:ident: $BufReader:ty
            $(, $async:ident, $await:ident)?
        ) => {
            mod small_buffers {
                use crate::events::{BytesCData, BytesDecl, BytesStart, BytesText, Event};
                use crate::reader::Reader;
                use pretty_assertions::assert_eq;

                #[$test]
                $($async)? fn decl() {
                    let xml = "<?xml ?>";
                    //         ^^^^^^^ data that fit into buffer
                    let size = xml.match_indices("?>").next().unwrap().0 + 1;
                    let br = <$BufReader>::with_capacity(size, xml.as_bytes());
                    let mut reader = Reader::from_reader(br);
                    let mut buf = Vec::new();

                    assert_eq!(
                        reader.$read_event(&mut buf) $(.$await)? .unwrap(),
                        Event::Decl(BytesDecl::from_start(BytesStart::from_content("xml ", 3)))
                    );
                    assert_eq!(
                        reader.$read_event(&mut buf) $(.$await)? .unwrap(),
                        Event::Eof
                    );
                }

                #[$test]
                $($async)? fn pi() {
                    let xml = "<?pi?>";
                    //         ^^^^^ data that fit into buffer
                    let size = xml.match_indices("?>").next().unwrap().0 + 1;
                    let br = <$BufReader>::with_capacity(size, xml.as_bytes());
                    let mut reader = Reader::from_reader(br);
                    let mut buf = Vec::new();

                    assert_eq!(
                        reader.$read_event(&mut buf) $(.$await)? .unwrap(),
                        Event::PI(BytesText::new("pi"))
                    );
                    assert_eq!(
                        reader.$read_event(&mut buf) $(.$await)? .unwrap(),
                        Event::Eof
                    );
                }

                #[$test]
                $($async)? fn empty() {
                    let xml = "<empty/>";
                    //         ^^^^^^^ data that fit into buffer
                    let size = xml.match_indices("/>").next().unwrap().0 + 1;
                    let br = <$BufReader>::with_capacity(size, xml.as_bytes());
                    let mut reader = Reader::from_reader(br);
                    let mut buf = Vec::new();

                    assert_eq!(
                        reader.$read_event(&mut buf) $(.$await)? .unwrap(),
                        Event::Empty(BytesStart::new("empty"))
                    );
                    assert_eq!(
                        reader.$read_event(&mut buf) $(.$await)? .unwrap(),
                        Event::Eof
                    );
                }

                #[$test]
                $($async)? fn cdata1() {
                    let xml = "<![CDATA[cdata]]>";
                    //         ^^^^^^^^^^^^^^^ data that fit into buffer
                    let size = xml.match_indices("]]>").next().unwrap().0 + 1;
                    let br = <$BufReader>::with_capacity(size, xml.as_bytes());
                    let mut reader = Reader::from_reader(br);
                    let mut buf = Vec::new();

                    assert_eq!(
                        reader.$read_event(&mut buf) $(.$await)? .unwrap(),
                        Event::CData(BytesCData::new("cdata"))
                    );
                    assert_eq!(
                        reader.$read_event(&mut buf) $(.$await)? .unwrap(),
                        Event::Eof
                    );
                }

                #[$test]
                $($async)? fn cdata2() {
                    let xml = "<![CDATA[cdata]]>";
                    //         ^^^^^^^^^^^^^^^^ data that fit into buffer
                    let size = xml.match_indices("]]>").next().unwrap().0 + 2;
                    let br = <$BufReader>::with_capacity(size, xml.as_bytes());
                    let mut reader = Reader::from_reader(br);
                    let mut buf = Vec::new();

                    assert_eq!(
                        reader.$read_event(&mut buf) $(.$await)? .unwrap(),
                        Event::CData(BytesCData::new("cdata"))
                    );
                    assert_eq!(
                        reader.$read_event(&mut buf) $(.$await)? .unwrap(),
                        Event::Eof
                    );
                }

                #[$test]
                $($async)? fn comment1() {
                    let xml = "<!--comment-->";
                    //         ^^^^^^^^^^^^ data that fit into buffer
                    let size = xml.match_indices("-->").next().unwrap().0 + 1;
                    let br = <$BufReader>::with_capacity(size, xml.as_bytes());
                    let mut reader = Reader::from_reader(br);
                    let mut buf = Vec::new();

                    assert_eq!(
                        reader.$read_event(&mut buf) $(.$await)? .unwrap(),
                        Event::Comment(BytesText::new("comment"))
                    );
                    assert_eq!(
                        reader.$read_event(&mut buf) $(.$await)? .unwrap(),
                        Event::Eof
                    );
                }

                #[$test]
                $($async)? fn comment2() {
                    let xml = "<!--comment-->";
                    //         ^^^^^^^^^^^^^ data that fit into buffer
                    let size = xml.match_indices("-->").next().unwrap().0 + 2;
                    let br = <$BufReader>::with_capacity(size, xml.as_bytes());
                    let mut reader = Reader::from_reader(br);
                    let mut buf = Vec::new();

                    assert_eq!(
                        reader.$read_event(&mut buf) $(.$await)? .unwrap(),
                        Event::Comment(BytesText::new("comment"))
                    );
                    assert_eq!(
                        reader.$read_event(&mut buf) $(.$await)? .unwrap(),
                        Event::Eof
                    );
                }
            }
        };
    }

    // Export macros for the child modules:
    // - buffered_reader
    // - slice_reader
    pub(super) use check;
    pub(super) use small_buffers;
}

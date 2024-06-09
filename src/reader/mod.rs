//! Contains high-level interface for a pull-based XML parser.

#[cfg(feature = "encoding")]
use encoding_rs::Encoding;
use std::ops::Range;

use crate::encoding::Decoder;
use crate::errors::{Error, Result, SyntaxError};
use crate::events::Event;
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
        $self:ident, $buf:ident,
        $reader:expr,
        $read_until_open:ident,
        $read_until_close:ident
        $(, $await:ident)?
    ) => {{
        let event = loop {
            match $self.state.state {
                ParseState::Init => { // Go to OpenedTag state
                    // If encoding set explicitly, we not need to detect it. For example,
                    // explicit UTF-8 set automatically if Reader was created using `from_str`.
                    // But we still need to remove BOM for consistency with no encoding
                    // feature enabled path
                    #[cfg(feature = "encoding")]
                    if let Some(encoding) = $reader.detect_encoding() $(.$await)? ? {
                        if $self.state.encoding.can_be_refined() {
                            $self.state.encoding = crate::reader::EncodingRef::BomDetected(encoding);
                        }
                    }

                    // Removes UTF-8 BOM if it is present
                    #[cfg(not(feature = "encoding"))]
                    $reader.remove_utf8_bom() $(.$await)? ?;

                    // Go to OpenedTag state
                    match $self.$read_until_open($buf) $(.$await)? {
                        Ok(Ok(ev)) => break Ok(ev),
                        Ok(Err(b)) => $buf = b,
                        Err(err)   => break Err(err),
                    }
                },
                ParseState::ClosedTag => { // Go to OpenedTag state
                    match $self.$read_until_open($buf) $(.$await)? {
                        Ok(Ok(ev)) => break Ok(ev),
                        Ok(Err(b)) => $buf = b,
                        Err(err)   => break Err(err),
                    }
                },
                // Go to ClosedTag state in next two arms
                ParseState::OpenedTag => break $self.$read_until_close($buf) $(.$await)?,
                ParseState::Empty => break $self.state.close_expanded_empty(),
                ParseState::Exit => break Ok(Event::Eof),
            };
        };
        match event {
            // #513: In case of ill-formed errors we already consume the wrong data
            // and change the state. We can continue parsing if we wish
            Err(Error::IllFormed(_)) => {}
            Err(_) | Ok(Event::Eof) => $self.state.state = ParseState::Exit,
            _ => {}
        }
        event
    }};
}

/// Read bytes up to `<` and skip it. If current byte (after skipping all space
/// characters if [`Config::trim_text_start`] is `true`) is already `<`, then
/// returns the next event, otherwise stay at position just after the `<` symbol.
///
/// Moves parser to the `OpenedTag` state.
///
/// This code is executed in two cases:
/// - after start of parsing just after skipping BOM if it is present
/// - after parsing `</tag>` or `<tag>`
macro_rules! read_until_open {
    (
        $self:ident, $buf:ident,
        $reader:expr,
        $read_event:ident
        $(, $await:ident)?
    ) => {{
        if $self.state.config.trim_text_start {
            $reader.skip_whitespace(&mut $self.state.offset) $(.$await)? ?;
        }

        // If we already at the `<` symbol, do not try to return an empty Text event
        if $reader.skip_one(b'<') $(.$await)? ? {
            $self.state.offset += 1;
            $self.state.state = ParseState::OpenedTag;
            // Pass $buf to the next next iteration of parsing loop
            return Ok(Err($buf));
        }

        match $reader
            .read_bytes_until(b'<', $buf, &mut $self.state.offset)
            $(.$await)?
        {
            Ok((bytes, found)) => {
                if found {
                    $self.state.state = ParseState::OpenedTag;
                }
                // Return Text event with `bytes` content or Eof if bytes is empty
                $self.state.emit_text(bytes).map(Ok)
            }
            Err(e) => Err(e),
        }
    }};
}

/// Read bytes up to the `>` and skip it. This method is expected to be called
/// after seeing the `<` symbol and skipping it. Inspects the next (current)
/// symbol and returns an appropriate [`Event`]:
///
/// |Symbol |Event
/// |-------|-------------------------------------
/// |`!`    |[`Comment`], [`CData`] or [`DocType`]
/// |`/`    |[`End`]
/// |`?`    |[`PI`]
/// |_other_|[`Start`] or [`Empty`]
///
/// Moves parser to the `ClosedTag` state.
///
/// [`Comment`]: Event::Comment
/// [`CData`]: Event::CData
/// [`DocType`]: Event::DocType
/// [`End`]: Event::End
/// [`PI`]: Event::PI
/// [`Start`]: Event::Start
/// [`Empty`]: Event::Empty
macro_rules! read_until_close {
    (
        $self:ident, $buf:ident,
        $reader:expr
        $(, $await:ident)?
    ) => {{
        $self.state.state = ParseState::ClosedTag;

        let start = $self.state.offset;
        match $reader.peek_one() $(.$await)? {
            // `<!` - comment, CDATA or DOCTYPE declaration
            Ok(Some(b'!')) => match $reader
                .read_bang_element($buf, &mut $self.state.offset)
                $(.$await)?
            {
                Ok((bang_type, bytes)) => $self.state.emit_bang(bang_type, bytes),
                Err(e) => {
                    // <!....EOF
                    //  ^^^^^ - `buf` does not contains `<`, but we want to report error at `<`,
                    //          so we move offset to it (-1 for `<`)
                    $self.state.last_error_offset = start - 1;
                    Err(e)
                }
            },
            // `</` - closing tag
            Ok(Some(b'/')) => match $reader
                .read_bytes_until(b'>', $buf, &mut $self.state.offset)
                $(.$await)?
            {
                Ok((bytes, true)) => $self.state.emit_end(bytes),
                Ok((_, false)) => {
                    // We want to report error at `<`, but offset was increased,
                    // so return it back (-1 for `<`)
                    $self.state.last_error_offset = start - 1;
                    Err(Error::Syntax(SyntaxError::UnclosedTag))
                }
                Err(e) => Err(e),
            },
            // `<?` - processing instruction
            Ok(Some(b'?')) => match $reader
                .read_with(PiParser::default(), $buf, &mut $self.state.offset)
                $(.$await)?
            {
                Ok(bytes) => $self.state.emit_question_mark(bytes),
                Err(e) => {
                    // We want to report error at `<`, but offset was increased,
                    // so return it back (-1 for `<`)
                    $self.state.last_error_offset = start - 1;
                    Err(e)
                }
            },
            // `<...` - opening or self-closed tag
            Ok(Some(_)) => match $reader
                .read_with(ElementParser::default(), $buf, &mut $self.state.offset)
                $(.$await)?
            {
                Ok(bytes) => $self.state.emit_start(bytes),
                Err(e) => Err(e),
            },
            // `<` - syntax error, tag not closed
            Ok(None) => {
                // We want to report error at `<`, but offset was increased,
                // so return it back (-1 for `<`)
                $self.state.last_error_offset = start - 1;
                Err(Error::Syntax(SyntaxError::UnclosedTag))
            }
            Err(e) => Err(e),
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
mod element;
mod ns_reader;
mod pi;
mod slice_reader;
mod state;

pub use element::ElementParser;
pub use ns_reader::NsReader;
pub use pi::PiParser;

/// Range of input in bytes, that corresponds to some piece of XML
pub type Span = Range<usize>;

////////////////////////////////////////////////////////////////////////////////////////////////////

/// Possible reader states. The state transition diagram (`true` and `false` shows
/// value of [`Config::expand_empty_elements`] option):
///
/// ```mermaid
/// flowchart LR
///   subgraph _
///     direction LR
///
///     Init      -- "(no event)"\n                                       --> OpenedTag
///     OpenedTag -- Decl, DocType, PI\nComment, CData\nStart, Empty, End --> ClosedTag
///     ClosedTag -- "#lt;false#gt;\n(no event)"\nText                    --> OpenedTag
///   end
///   ClosedTag -- "#lt;true#gt;"\nStart --> Empty
///   Empty     -- End                   --> ClosedTag
///   _ -. Eof .-> Exit
/// ```
#[derive(Clone, Debug)]
enum ParseState {
    /// Initial state in which reader stay after creation. Transition from that
    /// state could produce a `Text`, `Decl`, `Comment` or `Start` event. The next
    /// state is always `OpenedTag`. The reader will never return to this state. The
    /// event emitted during transition to `OpenedTag` is a `StartEvent` if the
    /// first symbol not `<`, otherwise no event are emitted.
    Init,
    /// State after seeing the `<` symbol. Depending on the next symbol all other
    /// events could be generated.
    ///
    /// After generating one event the reader moves to the `ClosedTag` state.
    OpenedTag,
    /// State in which reader searches the `<` symbol of a markup. All bytes before
    /// that symbol will be returned in the [`Event::Text`] event. After that
    /// the reader moves to the `OpenedTag` state.
    ClosedTag,
    /// This state is used only if option [`expand_empty_elements`] is set to `true`.
    /// Reader enters to this state when it is in a `ClosedTag` state and emits an
    /// [`Event::Start`] event. The next event emitted will be an [`Event::End`],
    /// after which reader returned to the `ClosedTag` state.
    ///
    /// [`expand_empty_elements`]: Config::expand_empty_elements
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
        // when internal state is OpenedTag, we have actually read until '<',
        // which we don't want to show
        if let ParseState::OpenedTag = self.state.state {
            self.state.offset - 1
        } else {
            self.state.offset
        }
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

/// Private sync reading methods
impl<R> Reader<R> {
    /// Read text into the given buffer, and return an event that borrows from
    /// either that buffer or from the input itself, based on the type of the
    /// reader.
    fn read_event_impl<'i, B>(&mut self, mut buf: B) -> Result<Event<'i>>
    where
        R: XmlSource<'i, B>,
    {
        read_event_impl!(self, buf, self.reader, read_until_open, read_until_close)
    }

    /// Read until '<' is found, moves reader to an `OpenedTag` state and returns a `Text` event.
    ///
    /// Returns inner `Ok` if the loop should be broken and an event returned.
    /// Returns inner `Err` with the same `buf` because Rust borrowck stumbles upon this case in particular.
    fn read_until_open<'i, B>(&mut self, buf: B) -> Result<std::result::Result<Event<'i>, B>>
    where
        R: XmlSource<'i, B>,
    {
        read_until_open!(self, buf, self.reader, read_event_impl)
    }

    /// Private function to read until `>` is found. This function expects that
    /// it was called just after encounter a `<` symbol.
    fn read_until_close<'i, B>(&mut self, buf: B) -> Result<Event<'i>>
    where
        R: XmlSource<'i, B>,
    {
        read_until_close!(self, buf, self.reader)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// Used to decouple reading of data from data source and parsing XML structure from it.
/// This is a state preserved between getting chunks of bytes from the reader.
///
/// This trait is implemented for every parser that processes piece of XML grammar.
pub trait Parser {
    /// Process new data and try to determine end of the parsed thing.
    ///
    /// Returns position of the end of thing in `bytes` in case of successful search
    /// and `None` otherwise.
    ///
    /// # Parameters
    /// - `bytes`: a slice to find the end of a thing.
    ///   Should contain text in ASCII-compatible encoding
    fn feed(&mut self, bytes: &[u8]) -> Option<usize>;

    /// Returns parse error produced by this parser in case of reaching end of
    /// input without finding the end of a parsed thing.
    fn eof_error() -> SyntaxError;
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
    /// Removes UTF-8 BOM if it is present
    #[cfg(not(feature = "encoding"))]
    fn remove_utf8_bom(&mut self) -> Result<()>;

    /// Determines encoding from the start of input and removes BOM if it is present
    #[cfg(feature = "encoding")]
    fn detect_encoding(&mut self) -> Result<Option<&'static Encoding>>;

    /// Read input until `byte` is found or end of input is reached.
    ///
    /// Returns a slice of data read up to `byte` (exclusive),
    /// and a flag noting whether `byte` was found in the input or not.
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
    ///     (b"abc".as_ref(), true)
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
    ) -> Result<(&'r [u8], bool)>;

    /// Read input until processing instruction is finished.
    ///
    /// This method expect that start sequence of a parser already was read.
    ///
    /// Returns a slice of data read up to the end of the thing being parsed.
    /// The end of thing and the returned content is determined by the used parser.
    ///
    /// If input (`Self`) is exhausted and no bytes was read, or if the specified
    /// parser could not find the ending sequence of the thing, returns `SyntaxError`.
    ///
    /// # Parameters
    /// - `buf`: Buffer that could be filled from an input (`Self`) and
    ///   from which [events] could borrow their data
    /// - `position`: Will be increased by amount of bytes consumed
    ///
    /// A `P` type parameter is used to preserve state between calls to the underlying
    /// reader which provides bytes fed into the parser.
    /// [events]: crate::events::Event
    fn read_with<P>(&mut self, parser: P, buf: B, position: &mut usize) -> Result<&'r [u8]>
    where
        P: Parser;

    /// Read input until comment or CDATA is finished.
    ///
    /// This method expect that `<` already was read.
    ///
    /// Returns a slice of data read up to end of comment or CDATA (`>`),
    /// which does not include into result.
    ///
    /// If input (`Self`) is exhausted and nothing was read, returns `None`.
    ///
    /// # Parameters
    /// - `buf`: Buffer that could be filled from an input (`Self`) and
    ///   from which [events] could borrow their data
    /// - `position`: Will be increased by amount of bytes consumed
    ///
    /// [events]: crate::events::Event
    fn read_bang_element(&mut self, buf: B, position: &mut usize) -> Result<(BangType, &'r [u8])>;

    /// Consume and discard all the whitespace until the next non-whitespace
    /// character or EOF.
    ///
    /// # Parameters
    /// - `position`: Will be increased by amount of bytes consumed
    fn skip_whitespace(&mut self, position: &mut usize) -> Result<()>;

    /// Consume and discard one character if it matches the given byte. Return
    /// `true` if it matched.
    ///
    /// # Parameters
    /// - `byte`: Character to skip
    fn skip_one(&mut self, byte: u8) -> Result<bool>;

    /// Return one character without consuming it, so that future `read_*` calls
    /// will still include it. On EOF, return `None`.
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
            _ => return Err(Error::Syntax(SyntaxError::InvalidBangMarkup)),
        })
    }

    /// If element is finished, returns its content up to `>` symbol and
    /// an index of this symbol, otherwise returns `None`
    ///
    /// # Parameters
    /// - `buf`: buffer with data consumed on previous iterations
    /// - `chunk`: data read on current iteration and not yet consumed from reader
    #[inline(always)]
    fn parse<'b>(&self, buf: &[u8], chunk: &'b [u8]) -> Option<(&'b [u8], usize)> {
        for i in memchr::memchr_iter(b'>', chunk) {
            match self {
                // Need to read at least 6 symbols (`!---->`) for properly finished comment
                // <!----> - XML comment
                //  012345 - i
                Self::Comment if buf.len() + i > 4 => {
                    if chunk[..i].ends_with(b"--") {
                        // We cannot strip last `--` from the buffer because we need it in case of
                        // check_comments enabled option. XML standard requires that comment
                        // will not end with `--->` sequence because this is a special case of
                        // `--` in the comment (https://www.w3.org/TR/xml11/#sec-comments)
                        return Some((&chunk[..i], i + 1)); // +1 for `>`
                    }
                    // End sequence `-|->` was splitted at |
                    //        buf --/   \-- chunk
                    if i == 1 && buf.ends_with(b"-") && chunk[0] == b'-' {
                        return Some((&chunk[..i], i + 1)); // +1 for `>`
                    }
                    // End sequence `--|>` was splitted at |
                    //         buf --/   \-- chunk
                    if i == 0 && buf.ends_with(b"--") {
                        return Some((&[], i + 1)); // +1 for `>`
                    }
                }
                Self::Comment => {}
                Self::CData => {
                    if chunk[..i].ends_with(b"]]") {
                        return Some((&chunk[..i], i + 1)); // +1 for `>`
                    }
                    // End sequence `]|]>` was splitted at |
                    //        buf --/   \-- chunk
                    if i == 1 && buf.ends_with(b"]") && chunk[0] == b']' {
                        return Some((&chunk[..i], i + 1)); // +1 for `>`
                    }
                    // End sequence `]]|>` was splitted at |
                    //         buf --/   \-- chunk
                    if i == 0 && buf.ends_with(b"]]") {
                        return Some((&[], i + 1)); // +1 for `>`
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
    fn to_err(&self) -> Error {
        match self {
            Self::CData => Error::Syntax(SyntaxError::UnclosedCData),
            Self::Comment => Error::Syntax(SyntaxError::UnclosedComment),
            Self::DocType => Error::Syntax(SyntaxError::UnclosedDoctype),
        }
    }
}

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
            $read_until_close:ident,
            // constructor of the XML source on which internal functions will be called
            $source:path,
            // constructor of the buffer to which read data will stored
            $buf:expr
            $(, $async:ident, $await:ident)?
        ) => {
            mod read_bytes_until {
                use super::*;
                // Use Bytes for printing bytes as strings for ASCII range
                use crate::utils::Bytes;
                use pretty_assertions::assert_eq;

                /// Checks that search in the empty buffer returns `None`
                #[$test]
                $($async)? fn empty() {
                    let buf = $buf;
                    let mut position = 0;
                    let mut input = b"".as_ref();
                    //                ^= 0

                    let (bytes, found) = $source(&mut input)
                        .read_bytes_until(b'*', buf, &mut position)
                        $(.$await)?
                        .unwrap();
                    assert_eq!(
                        (Bytes(bytes), found),
                        (Bytes(b""), false)
                    );
                    assert_eq!(position, 0);
                }

                /// Checks that search in the buffer non-existent value returns entire buffer
                /// as a result and set `position` to `len()`
                #[$test]
                $($async)? fn non_existent() {
                    let buf = $buf;
                    let mut position = 0;
                    let mut input = b"abcdef".as_ref();
                    //                      ^= 6

                    let (bytes, found) = $source(&mut input)
                        .read_bytes_until(b'*', buf, &mut position)
                        $(.$await)?
                        .unwrap();
                    assert_eq!(
                        (Bytes(bytes), found),
                        (Bytes(b"abcdef"), false)
                    );
                    assert_eq!(position, 6);
                }

                /// Checks that search in the buffer an element that is located in the front of
                /// buffer returns empty slice as a result and set `position` to one symbol
                /// after match (`1`)
                #[$test]
                $($async)? fn at_the_start() {
                    let buf = $buf;
                    let mut position = 0;
                    let mut input = b"*abcdef".as_ref();
                    //                 ^= 1

                    let (bytes, found) = $source(&mut input)
                        .read_bytes_until(b'*', buf, &mut position)
                        $(.$await)?
                        .unwrap();
                    assert_eq!(
                        (Bytes(bytes), found),
                        (Bytes(b""), true)
                    );
                    assert_eq!(position, 1); // position after the symbol matched
                }

                /// Checks that search in the buffer an element that is located in the middle of
                /// buffer returns slice before that symbol as a result and set `position` to one
                /// symbol after match
                #[$test]
                $($async)? fn inside() {
                    let buf = $buf;
                    let mut position = 0;
                    let mut input = b"abc*def".as_ref();
                    //                    ^= 4

                    let (bytes, found) = $source(&mut input)
                        .read_bytes_until(b'*', buf, &mut position)
                        $(.$await)?
                        .unwrap();
                    assert_eq!(
                        (Bytes(bytes), found),
                        (Bytes(b"abc"), true)
                    );
                    assert_eq!(position, 4); // position after the symbol matched
                }

                /// Checks that search in the buffer an element that is located in the end of
                /// buffer returns slice before that symbol as a result and set `position` to one
                /// symbol after match (`len()`)
                #[$test]
                $($async)? fn in_the_end() {
                    let buf = $buf;
                    let mut position = 0;
                    let mut input = b"abcdef*".as_ref();
                    //                       ^= 7

                    let (bytes, found) = $source(&mut input)
                        .read_bytes_until(b'*', buf, &mut position)
                        $(.$await)?
                        .unwrap();
                    assert_eq!(
                        (Bytes(bytes), found),
                        (Bytes(b"abcdef"), true)
                    );
                    assert_eq!(position, 7); // position after the symbol matched
                }
            }

            mod read_bang_element {
                use super::*;
                use crate::errors::{Error, SyntaxError};
                use crate::reader::BangType;
                use crate::utils::Bytes;

                /// Checks that reading CDATA content works correctly
                mod cdata {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    /// Checks that if input begins like CDATA element, but CDATA start sequence
                    /// is not finished, parsing ends with an error
                    #[$test]
                    #[ignore = "start CDATA sequence fully checked outside of `read_bang_element`"]
                    $($async)? fn not_properly_start() {
                        let buf = $buf;
                        let mut position = 1;
                        let mut input = b"![]]>other content".as_ref();
                        //                ^= 1

                        match $source(&mut input).read_bang_element(buf, &mut position) $(.$await)? {
                            Err(Error::Syntax(SyntaxError::UnclosedCData)) => {}
                            x => panic!(
                                "Expected `Err(Syntax(UnclosedCData))`, but got `{:?}`",
                                x
                            ),
                        }
                        assert_eq!(position, 1);
                    }

                    /// Checks that if CDATA startup sequence was matched, but an end sequence
                    /// is not found, parsing ends with an error
                    #[$test]
                    $($async)? fn not_closed() {
                        let buf = $buf;
                        let mut position = 1;
                        let mut input = b"![CDATA[other content".as_ref();
                        //                ^= 1                 ^= 22

                        match $source(&mut input).read_bang_element(buf, &mut position) $(.$await)? {
                            Err(Error::Syntax(SyntaxError::UnclosedCData)) => {}
                            x => panic!(
                                "Expected `Err(Syntax(UnclosedCData))`, but got `{:?}`",
                                x
                            ),
                        }
                        assert_eq!(position, 22);
                    }

                    /// Checks that CDATA element without content inside parsed successfully
                    #[$test]
                    $($async)? fn empty() {
                        let buf = $buf;
                        let mut position = 1;
                        let mut input = b"![CDATA[]]>other content".as_ref();
                        //                ^= 1       ^= 12

                        let (ty, bytes) = $source(&mut input)
                            .read_bang_element(buf, &mut position)
                            $(.$await)?
                            .unwrap();
                        assert_eq!(
                            (ty, Bytes(bytes)),
                            (BangType::CData, Bytes(b"![CDATA[]]"))
                        );
                        assert_eq!(position, 12);
                    }

                    /// Checks that CDATA element with content parsed successfully.
                    /// Additionally checks that sequences inside CDATA that may look like
                    /// a CDATA end sequence do not interrupt CDATA parsing
                    #[$test]
                    $($async)? fn with_content() {
                        let buf = $buf;
                        let mut position = 1;
                        let mut input = b"![CDATA[cdata]] ]>content]]>other content]]>".as_ref();
                        //                ^= 1                        ^= 29

                        let (ty, bytes) = $source(&mut input)
                            .read_bang_element(buf, &mut position)
                            $(.$await)?
                            .unwrap();
                        assert_eq!(
                            (ty, Bytes(bytes)),
                            (BangType::CData, Bytes(b"![CDATA[cdata]] ]>content]]"))
                        );
                        assert_eq!(position, 29);
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
                    use pretty_assertions::assert_eq;

                    #[$test]
                    #[ignore = "start comment sequence fully checked outside of `read_bang_element`"]
                    $($async)? fn not_properly_start() {
                        let buf = $buf;
                        let mut position = 1;
                        let mut input = b"!- -->other content".as_ref();
                        //                ^= 1

                        match $source(&mut input).read_bang_element(buf, &mut position) $(.$await)? {
                            Err(Error::Syntax(SyntaxError::UnclosedComment)) => {}
                            x => panic!(
                                "Expected `Err(Syntax(UnclosedComment))`, but got `{:?}`",
                                x
                            ),
                        }
                        assert_eq!(position, 1);
                    }

                    #[$test]
                    $($async)? fn not_properly_end() {
                        let buf = $buf;
                        let mut position = 1;
                        let mut input = b"!->other content".as_ref();
                        //                ^= 1            ^= 17

                        match $source(&mut input).read_bang_element(buf, &mut position) $(.$await)? {
                            Err(Error::Syntax(SyntaxError::UnclosedComment)) => {}
                            x => panic!(
                                "Expected `Err(Syntax(UnclosedComment))`, but got `{:?}`",
                                x
                            ),
                        }
                        assert_eq!(position, 17);
                    }

                    #[$test]
                    $($async)? fn not_closed1() {
                        let buf = $buf;
                        let mut position = 1;
                        let mut input = b"!--other content".as_ref();
                        //                ^= 1            ^= 17

                        match $source(&mut input).read_bang_element(buf, &mut position) $(.$await)? {
                            Err(Error::Syntax(SyntaxError::UnclosedComment)) => {}
                            x => panic!(
                                "Expected `Err(Syntax(UnclosedComment))`, but got `{:?}`",
                                x
                            ),
                        }
                        assert_eq!(position, 17);
                    }

                    #[$test]
                    $($async)? fn not_closed2() {
                        let buf = $buf;
                        let mut position = 1;
                        let mut input = b"!-->other content".as_ref();
                        //                ^= 1             ^= 18

                        match $source(&mut input).read_bang_element(buf, &mut position) $(.$await)? {
                            Err(Error::Syntax(SyntaxError::UnclosedComment)) => {}
                            x => panic!(
                                "Expected `Err(Syntax(UnclosedComment))`, but got `{:?}`",
                                x
                            ),
                        }
                        assert_eq!(position, 18);
                    }

                    #[$test]
                    $($async)? fn not_closed3() {
                        let buf = $buf;
                        let mut position = 1;
                        let mut input = b"!--->other content".as_ref();
                        //                ^= 1              ^= 19

                        match $source(&mut input).read_bang_element(buf, &mut position) $(.$await)? {
                            Err(Error::Syntax(SyntaxError::UnclosedComment)) => {}
                            x => panic!(
                                "Expected `Err(Syntax(UnclosedComment))`, but got `{:?}`",
                                x
                            ),
                        }
                        assert_eq!(position, 19);
                    }

                    #[$test]
                    $($async)? fn empty() {
                        let buf = $buf;
                        let mut position = 1;
                        let mut input = b"!---->other content".as_ref();
                        //                ^= 1  ^= 7

                        let (ty, bytes) = $source(&mut input)
                            .read_bang_element(buf, &mut position)
                            $(.$await)?
                            .unwrap();
                        assert_eq!(
                            (ty, Bytes(bytes)),
                            (BangType::Comment, Bytes(b"!----"))
                        );
                        assert_eq!(position, 7);
                    }

                    #[$test]
                    $($async)? fn with_content() {
                        let buf = $buf;
                        let mut position = 1;
                        let mut input = b"!--->comment<--->other content".as_ref();
                        //                ^= 1             ^= 18

                        let (ty, bytes) = $source(&mut input)
                            .read_bang_element(buf, &mut position)
                            $(.$await)?
                            .unwrap();
                        assert_eq!(
                            (ty, Bytes(bytes)),
                            (BangType::Comment, Bytes(b"!--->comment<---"))
                        );
                        assert_eq!(position, 18);
                    }
                }

                /// Checks that reading DOCTYPE definition works correctly
                mod doctype {
                    use super::*;

                    mod uppercase {
                        use super::*;
                        use pretty_assertions::assert_eq;

                        #[$test]
                        $($async)? fn not_properly_start() {
                            let buf = $buf;
                            let mut position = 1;
                            let mut input = b"!D other content".as_ref();
                            //                ^= 1            ^= 17

                            match $source(&mut input).read_bang_element(buf, &mut position) $(.$await)? {
                                Err(Error::Syntax(SyntaxError::UnclosedDoctype)) => {}
                                x => panic!(
                                    "Expected `Err(Syntax(UnclosedDoctype))`, but got `{:?}`",
                                    x
                                ),
                            }
                            assert_eq!(position, 17);
                        }

                        #[$test]
                        $($async)? fn without_space() {
                            let buf = $buf;
                            let mut position = 1;
                            let mut input = b"!DOCTYPEother content".as_ref();
                            //                ^= 1                 ^= 22

                            match $source(&mut input).read_bang_element(buf, &mut position) $(.$await)? {
                                Err(Error::Syntax(SyntaxError::UnclosedDoctype)) => {}
                                x => panic!(
                                    "Expected `Err(Syntax(UnclosedDoctype))`, but got `{:?}`",
                                    x
                                ),
                            }
                            assert_eq!(position, 22);
                        }

                        #[$test]
                        $($async)? fn empty() {
                            let buf = $buf;
                            let mut position = 1;
                            let mut input = b"!DOCTYPE>other content".as_ref();
                            //                ^= 1     ^= 10

                            let (ty, bytes) = $source(&mut input)
                                .read_bang_element(buf, &mut position)
                                $(.$await)?
                                .unwrap();
                            assert_eq!(
                                (ty, Bytes(bytes)),
                                (BangType::DocType, Bytes(b"!DOCTYPE"))
                            );
                            assert_eq!(position, 10);
                        }

                        #[$test]
                        $($async)? fn not_closed() {
                            let buf = $buf;
                            let mut position = 1;
                            let mut input = b"!DOCTYPE other content".as_ref();
                            //                ^= 1                  ^23

                            match $source(&mut input).read_bang_element(buf, &mut position) $(.$await)? {
                                Err(Error::Syntax(SyntaxError::UnclosedDoctype)) => {}
                                x => panic!(
                                    "Expected `Err(Syntax(UnclosedDoctype))`, but got `{:?}`",
                                    x
                                ),
                            }
                            assert_eq!(position, 23);
                        }
                    }

                    mod lowercase {
                        use super::*;
                        use pretty_assertions::assert_eq;

                        #[$test]
                        $($async)? fn not_properly_start() {
                            let buf = $buf;
                            let mut position = 1;
                            let mut input = b"!d other content".as_ref();
                            //                ^= 1            ^= 17

                            match $source(&mut input).read_bang_element(buf, &mut position) $(.$await)? {
                                Err(Error::Syntax(SyntaxError::UnclosedDoctype)) => {}
                                x => panic!(
                                    "Expected `Err(Syntax(UnclosedDoctype))`, but got `{:?}`",
                                    x
                                ),
                            }
                            assert_eq!(position, 17);
                        }

                        #[$test]
                        $($async)? fn without_space() {
                            let buf = $buf;
                            let mut position = 1;
                            let mut input = b"!doctypeother content".as_ref();
                            //                ^= 1                 ^= 22

                            match $source(&mut input).read_bang_element(buf, &mut position) $(.$await)? {
                                Err(Error::Syntax(SyntaxError::UnclosedDoctype)) => {}
                                x => panic!(
                                    "Expected `Err(Syntax(UnclosedDoctype))`, but got `{:?}`",
                                    x
                                ),
                            }
                            assert_eq!(position, 22);
                        }

                        #[$test]
                        $($async)? fn empty() {
                            let buf = $buf;
                            let mut position = 1;
                            let mut input = b"!doctype>other content".as_ref();
                            //                ^= 1     ^= 10

                            let (ty, bytes) = $source(&mut input)
                                .read_bang_element(buf, &mut position)
                                $(.$await)?
                                .unwrap();
                            assert_eq!(
                                (ty, Bytes(bytes)),
                                (BangType::DocType, Bytes(b"!doctype"))
                            );
                            assert_eq!(position, 10);
                        }

                        #[$test]
                        $($async)? fn not_closed() {
                            let buf = $buf;
                            let mut position = 1;
                            let mut input = b"!doctype other content".as_ref();
                            //                ^= 1                  ^= 23

                            match $source(&mut input).read_bang_element(buf, &mut position) $(.$await)? {
                                Err(Error::Syntax(SyntaxError::UnclosedDoctype)) => {}
                                x => panic!(
                                    "Expected `Err(Syntax(UnclosedDoctype))`, but got `{:?}`",
                                    x
                                ),
                            }
                            assert_eq!(position, 23);
                        }
                    }
                }
            }

            mod read_element {
                use super::*;
                use crate::errors::{Error, SyntaxError};
                use crate::reader::ElementParser;
                use crate::utils::Bytes;
                use pretty_assertions::assert_eq;

                /// Checks that nothing was read from empty buffer
                #[$test]
                $($async)? fn empty() {
                    let buf = $buf;
                    let mut position = 1;
                    let mut input = b"".as_ref();
                    //                ^= 1

                    match $source(&mut input).read_with(ElementParser::default(), buf, &mut position) $(.$await)? {
                        Err(Error::Syntax(SyntaxError::UnclosedTag)) => {}
                        x => panic!(
                            "Expected `Err(Syntax(UnclosedTag))`, but got `{:?}`",
                            x
                        ),
                    }
                    assert_eq!(position, 1);
                }

                mod open {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[$test]
                    $($async)? fn empty_tag() {
                        let buf = $buf;
                        let mut position = 1;
                        let mut input = b">".as_ref();
                        //                 ^= 2

                        assert_eq!(
                            Bytes($source(&mut input).read_with(ElementParser::default(), buf, &mut position) $(.$await)? .unwrap()),
                            Bytes(b"")
                        );
                        assert_eq!(position, 2);
                    }

                    #[$test]
                    $($async)? fn normal() {
                        let buf = $buf;
                        let mut position = 1;
                        let mut input = b"tag>".as_ref();
                        //                    ^= 5

                        assert_eq!(
                            Bytes($source(&mut input).read_with(ElementParser::default(), buf, &mut position) $(.$await)? .unwrap()),
                            Bytes(b"tag")
                        );
                        assert_eq!(position, 5);
                    }

                    #[$test]
                    $($async)? fn empty_ns_empty_tag() {
                        let buf = $buf;
                        let mut position = 1;
                        let mut input = b":>".as_ref();
                        //                  ^= 3

                        assert_eq!(
                            Bytes($source(&mut input).read_with(ElementParser::default(), buf, &mut position) $(.$await)? .unwrap()),
                            Bytes(b":")
                        );
                        assert_eq!(position, 3);
                    }

                    #[$test]
                    $($async)? fn empty_ns() {
                        let buf = $buf;
                        let mut position = 1;
                        let mut input = b":tag>".as_ref();
                        //                     ^= 6

                        assert_eq!(
                            Bytes($source(&mut input).read_with(ElementParser::default(), buf, &mut position) $(.$await)? .unwrap()),
                            Bytes(b":tag")
                        );
                        assert_eq!(position, 6);
                    }

                    #[$test]
                    $($async)? fn with_attributes() {
                        let buf = $buf;
                        let mut position = 1;
                        let mut input = br#"tag  attr-1=">"  attr2  =  '>'  3attr>"#.as_ref();
                        //                                                        ^= 39

                        assert_eq!(
                            Bytes($source(&mut input).read_with(ElementParser::default(), buf, &mut position) $(.$await)? .unwrap()),
                            Bytes(br#"tag  attr-1=">"  attr2  =  '>'  3attr"#)
                        );
                        assert_eq!(position, 39);
                    }
                }

                mod self_closed {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[$test]
                    $($async)? fn empty_tag() {
                        let buf = $buf;
                        let mut position = 1;
                        let mut input = b"/>".as_ref();
                        //                  ^= 3

                        assert_eq!(
                            Bytes($source(&mut input).read_with(ElementParser::default(), buf, &mut position) $(.$await)? .unwrap()),
                            Bytes(b"/")
                        );
                        assert_eq!(position, 3);
                    }

                    #[$test]
                    $($async)? fn normal() {
                        let buf = $buf;
                        let mut position = 1;
                        let mut input = b"tag/>".as_ref();
                        //                     ^= 6

                        assert_eq!(
                            Bytes($source(&mut input).read_with(ElementParser::default(), buf, &mut position) $(.$await)? .unwrap()),
                            Bytes(b"tag/")
                        );
                        assert_eq!(position, 6);
                    }

                    #[$test]
                    $($async)? fn empty_ns_empty_tag() {
                        let buf = $buf;
                        let mut position = 1;
                        let mut input = b":/>".as_ref();
                        //                   ^= 4

                        assert_eq!(
                            Bytes($source(&mut input).read_with(ElementParser::default(), buf, &mut position) $(.$await)? .unwrap()),
                            Bytes(b":/")
                        );
                        assert_eq!(position, 4);
                    }

                    #[$test]
                    $($async)? fn empty_ns() {
                        let buf = $buf;
                        let mut position = 1;
                        let mut input = b":tag/>".as_ref();
                        //                      ^= 7

                        assert_eq!(
                            Bytes($source(&mut input).read_with(ElementParser::default(), buf, &mut position) $(.$await)? .unwrap()),
                            Bytes(b":tag/")
                        );
                        assert_eq!(position, 7);
                    }

                    #[$test]
                    $($async)? fn with_attributes() {
                        let buf = $buf;
                        let mut position = 1;
                        let mut input = br#"tag  attr-1="/>"  attr2  =  '/>'  3attr/>"#.as_ref();
                        //                                                           ^= 42

                        assert_eq!(
                            Bytes($source(&mut input).read_with(ElementParser::default(), buf, &mut position) $(.$await)? .unwrap()),
                            Bytes(br#"tag  attr-1="/>"  attr2  =  '/>'  3attr/"#)
                        );
                        assert_eq!(position, 42);
                    }
                }
            }

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
                    let mut reader = Reader::from_str("<?xml-stylesheet '? >\" ?>");

                    assert_eq!(
                        reader.$read_event($buf) $(.$await)? .unwrap(),
                        Event::PI(BytesText::from_escaped("xml-stylesheet '? >\" "))
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

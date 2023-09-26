//! A low-level XML parser. For advanced use. It is very low-level and you
//! typically should not use it. Use a [`Reader`] instead.
//!
//! To use a parser create an instance of [`Parser`] and [`feed`] data into it.
//! After successful search the parser will return [`FeedResult`] with position
//! where match was found and returned variant will represent what exactly was
//! found. In case if the provided data is not enough to made any decision, a
//! [`FeedResult::NeedData`] is returned. Finally, if parser encounters a byte
//! that should not be there, a [`SyntaxError`] is returned.
//!
//! To fully parse a document you should pass unconsumed data to [`feed`] in a
//! loop, that means `&bytes[offset..]` for `Emit*` cases and a completely new
//! slice for a `NeedData` case:
//!
//! ```
//! # use quick_xml::parser::Parser;
//! use quick_xml::parser::FeedResult::*;
//! // Use `without_encoding_detection` instead if you don't want
//! // automatic encoding detection
//! let mut parser = Parser::default();
//! // Buffer for data of one event
//! let mut buf = Vec::new();
//! // Feed data by 3 bytes at once
//! for (i, mut chunk) in b"<xml-element attribute='true'>".chunks(3).enumerate() {
//!     loop {
//!         match parser.feed(chunk).unwrap() {
//!             // Return to the outer loop to request new chunk
//!             NeedData => break,
//!
//!             EncodingUtf8Like(offset) |
//!             EncodingUtf16BeLike(offset) |
//!             EncodingUtf16LeLike(offset) => {
//!                 // Consume BOM, but do not add it to the data
//!                 chunk = &chunk[offset..];
//!             }
//!             EmitText(offset) |
//!             EmitCData(offset) |
//!             EmitComment(offset) |
//!             EmitDoctype(offset) |
//!             EmitPI(offset) |
//!             EmitEmptyTag(offset) |
//!             EmitStartTag(offset) |
//!             EmitEndTag(offset) => {
//!                 // Append data of an event to the buffer
//!                 buf.extend_from_slice(&chunk[..offset]);
//!
//!                 // Consume already read data
//!                 chunk = &chunk[offset..];
//!
//!                 // Emit new event using `buf`
//!                 // ...
//!
//!                 // If content of buffer is not required anymore, it can be cleared
//!                 buf.clear();
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! [`Reader`]: crate::Reader
//! [`feed`]: Parser::feed()

use crate::errors::SyntaxError;
use bom::BomParser;
use cdata::CDataParser;
use quick_dtd::{CommentParser, DtdParser, PiParser, QuotedParser, OneOf};

mod bom;
mod cdata;

/// An internal state of a parser. Used to preserve information about currently
/// parsed event between calls to [`Parser::feed()`].
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum State {
    /// Initial state used to begin parse XML events.
    Start,
    Bom(BomParser),
    Text,

    /// A `<` was seen, but nothing else.
    Markup,
    /// A `<!` was seen, but nothing else. It is unable to understand right now
    /// what data follow.
    MaybeCommentOrCDataOrDoctype,

    /// A `<!-` was seen, but nothing else. It is unable to understand right now
    /// what data follow.
    MaybeComment,
    /// A `<!--` was seen and we now inside a comment.
    Comment(CommentParser),

    /// A `<![` was seen, but nothing else. It is unable to understand right now
    /// what data follow.
    MaybeCData1,
    /// A `<![C` was seen, but nothing else.
    MaybeCData2,
    /// A `<![CD` was seen, but nothing else.
    MaybeCData3,
    /// A `<![CDA` was seen, but nothing else.
    MaybeCData4,
    /// A `<![CDAT` was seen, but nothing else.
    MaybeCData5,
    /// A `<![CDATA` was seen, but nothing else.
    MaybeCData6,
    /// A `<![CDATA[` was seen and we now inside a character data content.
    CData(CDataParser),

    /// A `<!D` (in any case) was seen, but nothing else. It is unable to understand
    /// right now what data follow.
    MaybeDoctype1,
    /// A `<!DO` (in any case) was seen, but nothing else.
    MaybeDoctype2,
    /// A `<!DOC` (in any case) was seen, but nothing else.
    MaybeDoctype3,
    /// A `<!DOCT` (in any case) was seen, but nothing else.
    MaybeDoctype4,
    /// A `<!DOCTY` (in any case) was seen, but nothing else.
    MaybeDoctype5,
    /// A `<!DOCTYP` (in any case) was seen, but nothing else.
    MaybeDoctype6,
    /// A `<!DOCTYPE` (in any case) was seen, and we are looking for `<` or `>`.
    Doctype(QuotedParser),
    /// We are inside of `[]` of `<!DOCTYPE e []>` definition.
    Dtd(DtdParser),
    /// We are after `]` of `<!DOCTYPE e []>` definition, looking for `>`.
    DoctypeFinish,

    /// A `<?` was seen, but nothing else. We parsing a processing instruction.
    /// If parameter is `true`, then the `?` was the last symbol on the last
    /// consumed buffer.
    PI(PiParser),
    /// A `</` was seen, but `>` was not. Parser expect more data to close a tag
    /// and emit [`FeedResult::EmitEmptyTag`].
    EndTag,
    /// A `<*` was seen, but nothing else where `*` is an any byte, except `!`, `?`, or `/`.
    /// It is unable to understand right now what data follow.
    StartOrEmptyTag(QuotedParser, bool),
}

impl Default for State {
    fn default() -> Self {
        Self::Start
    }
}

/// A result of feeding data into [`Parser`].
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FeedResult {
    /// All fed bytes should be consumed, new portion should be feed
    NeedData,

    /// The specified amount of bytes should be consumed from the input and
    /// encoding of the document set to the UTF-8 compatible.
    /// The encoding should be refined after reading XML declaration.
    EncodingUtf8Like(usize),
    /// The specified amount of bytes should be consumed from the input and
    /// encoding of the document set to the UTF-16 Big-Endian compatible.
    /// The encoding should be refined after reading XML declaration.
    EncodingUtf16BeLike(usize),
    /// The specified amount of bytes should be consumed from the input and
    /// encoding of the document set to the UTF-16 Little-Endian compatible.
    /// The encoding should be refined after reading XML declaration.
    EncodingUtf16LeLike(usize),

    /// The specified amount of bytes should be consumed from the input and
    /// [`Event::Text`] should be emitted.
    ///
    /// [`Event::Text`]: crate::events::Event::Text
    EmitText(usize),

    /// The specified amount of bytes should be consumed from the input and
    /// [`Event::CData`] should be emitted.
    ///
    /// [`Event::CData`]: crate::events::Event::CData
    EmitCData(usize),
    /// The specified amount of bytes should be consumed from the input and
    /// [`Event::Comment`] should be emitted.
    ///
    /// [`Event::Comment`]: crate::events::Event::Comment
    EmitComment(usize),
    /// The specified amount of bytes should be consumed from the input and
    /// [`Event::DocType`] should be emitted.
    ///
    /// [`Event::DocType`]: crate::events::Event::DocType
    EmitDoctype(usize),

    /// The specified amount of bytes should be consumed from the input and
    /// [`Event::PI`] should be emitted.
    ///
    /// [`Event::PI`]: crate::events::Event::PI
    EmitPI(usize),

    /// The specified amount of bytes should be consumed from the input and
    /// [`Event::Empty`] should be emitted.
    ///
    /// [`Event::Empty`]: crate::events::Event::Empty
    EmitEmptyTag(usize),
    /// The specified amount of bytes should be consumed from the input and
    /// [`Event::Start`] should be emitted.
    ///
    /// [`Event::Start`]: crate::events::Event::Start
    EmitStartTag(usize),
    /// The specified amount of bytes should be consumed from the input and
    /// [`Event::End`] should be emitted.
    ///
    /// [`Event::End`]: crate::events::Event::End
    EmitEndTag(usize),
}

// convert `mermaid` block to a diagram
#[cfg_attr(doc, aquamarine::aquamarine)]
/// A low-level XML parser that searches a boundaries of various kinds of XML
/// events in the provided slice.
///
/// The parser represents a state machine with following states:
///
/// ```mermaid
/// flowchart TD
///    Text -->|<| Markup
///    Text -->|*| Text
///
///    Markup --> |!| CommentOrCDataOrDoctype
///    Markup --->|?| PIParser1
///    Markup --->|/| EndTagParser
///    Markup --> |*| StartOrEmptyTag
///
///    CommentOrCDataOrDoctype -->|-| CommentParser
///    CommentOrCDataOrDoctype -->|D| DoctypeParser1
///    CommentOrCDataOrDoctype -->|d| DoctypeParser1
///    CommentOrCDataOrDoctype -->|"["| CDataParser1
///    CommentOrCDataOrDoctype -->|*| Error
///
///    subgraph comment
///        CommentParser -->|-| CommentContent1
///        CommentParser ----->|*| CommentError
///
///        CommentContent1 -->|-| CommentContent2
///        CommentContent1 -->|*| CommentContent1
///
///        CommentContent2 -->|-| CommentContent3
///        CommentContent2 -->|*| CommentContent1
///
///        CommentContent3 -->|>| Comment
///        CommentContent3 -->|*| CommentContent1
///    end
///    subgraph doctype
///        DoctypeParser1 -->|O| DoctypeParser2
///        DoctypeParser1 -->|o| DoctypeParser2
///        DoctypeParser1 ---->|*| DoctypeError
///
///        DoctypeParser2 -->|C| DoctypeParser3
///        DoctypeParser2 -->|c| DoctypeParser3
///        DoctypeParser2 ---->|*| DoctypeError
///
///        DoctypeParser3 -->|T| DoctypeParser4
///        DoctypeParser3 -->|t| DoctypeParser4
///        DoctypeParser3 ---->|*| DoctypeError
///
///        DoctypeParser4 -->|Y| DoctypeParser5
///        DoctypeParser4 -->|y| DoctypeParser5
///        DoctypeParser4 ---->|*| DoctypeError
///
///        DoctypeParser5 -->|P| DoctypeParser6
///        DoctypeParser5 -->|p| DoctypeParser6
///        DoctypeParser5 ---->|*| DoctypeError
///
///        DoctypeParser6 -->|E| DoctypeContent1
///        DoctypeParser6 -->|e| DoctypeContent1
///        DoctypeParser6 ---->|*| DoctypeError
///
///        DoctypeContent1 -->|!| DoctypeContent2
///        DoctypeContent1 -->|*| DoctypeContent1
///
///        DoctypeContent2 -->|>| Doctype
///        DoctypeContent2 -->|*| DoctypeContent1
///    end
///    subgraph cdata
///        CDataParser1 -->|C| CDataParser2
///        CDataParser1 ----->|*| CDataError
///        CDataParser2 -->|D| CDataParser3
///        CDataParser2 ----->|*| CDataError
///        CDataParser3 -->|A| CDataParser4
///        CDataParser3 ----->|*| CDataError
///        CDataParser4 -->|T| CDataParser5
///        CDataParser4 ----->|*| CDataError
///        CDataParser5 -->|A| CDataParser6
///        CDataParser5 ----->|*| CDataError
///        CDataParser6 -->|"["| CDataContent1
///        CDataParser6 ----->|*| CDataError
///
///        CDataContent1 -->|"]"| CDataContent2
///        CDataContent1 -->|*| CDataContent1
///
///        CDataContent2 -->|"]"| CDataContent3
///        CDataContent2 -->|*| CDataContent1
///
///        CDataContent3 -->|>| CData
///        CDataContent3 -->|*| CDataContent1
///    end
///
///    subgraph pi_parser
///        PIParser1 -->|?| PIParser2
///        PIParser1 -->|*| PIParser1
///
///        PIParser2 -->|>| PI
///        PIParser2 -->|*| PIError
///    end
///
///    subgraph end_tag
///        EndTagParser -->|>| EndTag
///        EndTagParser -->|*| EndTagError
///    end
///
///    StartOrEmptyTag --> |/| EmptyTagParser
///    StartOrEmptyTag --->|>| StartTag
///    StartOrEmptyTag --> |*| StartOrEmptyTag
///
///    subgraph empty_tag
///        EmptyTagParser -->|>| EmptyTag
///        EmptyTagParser -->|*| EmptyTagError
///    end
/// ```
///
/// Every arrow on that diagram is marked with a byte that initiates that transition.
/// Transition marked with asterisks (`*`) represents any byte except explicitly
/// mentioned in other transitions from that state.
///
/// Each `Error` state on that diagram represents a [`SyntaxError`].
/// Every successful match (`Emit*`) returns the parser to state `Text`.
#[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
pub struct Parser(State);
impl Parser {
    /// Creates a parser that would not try to guess encoding from the input text.
    /// This is useful when you already knows the encoding and parses a part of document.
    #[inline]
    pub fn without_encoding_detection() -> Self {
        Self(State::Text)
    }

    /// Performs parsing of the provided byte slice and returns the outcome.
    /// See [`Parser`] for more info.
    ///
    /// # Parameters
    /// - `bytes`: a slice to search a new XML event. Should contain text in
    ///   ASCII-compatible encoding
    pub fn feed(&mut self, bytes: &[u8]) -> Result<FeedResult, SyntaxError> {
        for (offset, &byte) in bytes.iter().enumerate() {
            let trail = &bytes[offset..];
            let start = offset + 1;
            let rest = &bytes[start..];
            self.0 = match self.0 {
                State::Start => match byte {
                    0x00 => State::Bom(BomParser::X00),
                    b'<' => State::Bom(BomParser::X3C),
                    0xEF => State::Bom(BomParser::XEF),
                    0xFE => State::Bom(BomParser::XFE),
                    0xFF => State::Bom(BomParser::XFF),
                    _ => return Ok(self.parse_text(trail, offset)),
                },
                State::Bom(ref mut parser) => {
                    let encoding = match parser.feed(trail) {
                        bom::FeedResult::Unknown => FeedResult::EncodingUtf8Like(0),
                        bom::FeedResult::Utf8 => FeedResult::EncodingUtf8Like(0),
                        bom::FeedResult::Utf16Be => FeedResult::EncodingUtf16BeLike(0),
                        bom::FeedResult::Utf16Le => FeedResult::EncodingUtf16LeLike(0),
                        bom::FeedResult::Utf8Bom => FeedResult::EncodingUtf8Like(3),
                        bom::FeedResult::Utf16BeBom => FeedResult::EncodingUtf16BeLike(2),
                        bom::FeedResult::Utf16LeBom => FeedResult::EncodingUtf16LeLike(2),
                        bom::FeedResult::NeedData => return Ok(FeedResult::NeedData),
                    };
                    self.0 = State::Text;
                    return Ok(encoding);
                }
                State::Text => match byte {
                    b'<' => State::Markup,
                    _ => return Ok(self.parse_text(trail, offset)),
                },
                State::Markup => match byte {
                    b'!' => State::MaybeCommentOrCDataOrDoctype,
                    b'?' => return Ok(self.parse_pi(rest, start, PiParser::default())),
                    b'/' => return Ok(self.parse_end(rest, start)),
                    _ => {
                        return Ok(self.parse_start_or_empty(
                            trail,
                            offset,
                            QuotedParser::Outside,
                            false,
                        ))
                    }
                },
                State::MaybeCommentOrCDataOrDoctype => match byte {
                    b'-' => State::MaybeComment,
                    b'[' => State::MaybeCData1,
                    b'D' | b'd' => State::MaybeDoctype1,
                    _ => return Err(SyntaxError::InvalidBangMarkup),
                },

                //----------------------------------------------------------------------------------
                // <!-- comment -->
                //----------------------------------------------------------------------------------
                State::MaybeComment => match byte {
                    b'-' => return Ok(self.parse_comment(rest, start, CommentParser::default())),
                    _ => return Err(SyntaxError::UnclosedComment),
                },
                State::Comment(parser) => {
                    return Ok(self.parse_comment(trail, offset, parser));
                }

                //----------------------------------------------------------------------------------
                // <![CDATA[]]>
                //----------------------------------------------------------------------------------
                State::MaybeCData1 => match byte {
                    b'C' => State::MaybeCData2,
                    _ => return Err(SyntaxError::UnclosedCData),
                },
                State::MaybeCData2 => match byte {
                    b'D' => State::MaybeCData3,
                    _ => return Err(SyntaxError::UnclosedCData),
                },
                State::MaybeCData3 => match byte {
                    b'A' => State::MaybeCData4,
                    _ => return Err(SyntaxError::UnclosedCData),
                },
                State::MaybeCData4 => match byte {
                    b'T' => State::MaybeCData5,
                    _ => return Err(SyntaxError::UnclosedCData),
                },
                State::MaybeCData5 => match byte {
                    b'A' => State::MaybeCData6,
                    _ => return Err(SyntaxError::UnclosedCData),
                },
                State::MaybeCData6 => match byte {
                    b'[' => return Ok(self.parse_cdata(rest, start, CDataParser::default())),
                    _ => return Err(SyntaxError::UnclosedCData),
                },
                State::CData(parser) => return Ok(self.parse_cdata(trail, offset, parser)),

                //----------------------------------------------------------------------------------
                // <!DOCTYPE>
                //----------------------------------------------------------------------------------
                State::MaybeDoctype1 => match byte {
                    b'O' | b'o' => State::MaybeDoctype2,
                    _ => return Err(SyntaxError::UnclosedDoctype),
                },
                State::MaybeDoctype2 => match byte {
                    b'C' | b'c' => State::MaybeDoctype3,
                    _ => return Err(SyntaxError::UnclosedDoctype),
                },
                State::MaybeDoctype3 => match byte {
                    b'T' | b't' => State::MaybeDoctype4,
                    _ => return Err(SyntaxError::UnclosedDoctype),
                },
                State::MaybeDoctype4 => match byte {
                    b'Y' | b'y' => State::MaybeDoctype5,
                    _ => return Err(SyntaxError::UnclosedDoctype),
                },
                State::MaybeDoctype5 => match byte {
                    b'P' | b'p' => State::MaybeDoctype6,
                    _ => return Err(SyntaxError::UnclosedDoctype),
                },
                State::MaybeDoctype6 => match byte {
                    b'E' | b'e' => return self.parse_doctype(rest, start, QuotedParser::Outside),
                    _ => return Err(SyntaxError::UnclosedDoctype),
                },
                State::Doctype(parser) => return self.parse_doctype(trail, offset, parser),
                State::Dtd(parser) => return self.parse_dtd(trail, offset, parser),
                State::DoctypeFinish => return Ok(self.parse_doctype_finish(trail, offset)),

                State::PI(parser) => return Ok(self.parse_pi(trail, offset, parser)),
                State::EndTag => return Ok(self.parse_end(trail, offset)),
                State::StartOrEmptyTag(parser, has_slash) => {
                    return Ok(self.parse_start_or_empty(trail, offset, parser, has_slash));
                }
            }
        }
        Ok(FeedResult::NeedData)
    }

    /// This method should be called when all data was feed into parser.
    ///
    /// If parser in intermediate state it will return a corresponding syntax
    /// error, otherwise it returns successfully.
    // rustfmt tend to move pipes to the begin of a line which ruins the nice look
    #[rustfmt::skip]
    pub fn finish(self) -> Result<(), SyntaxError> {
        match self.0 {
            // If nothing was fed into parser, document is empty.
            // We allow empty documents, at least for now
            State::Start |
            State::Text => Ok(()),

            // We need data when we tried to determine document encoding
            // <
            State::Bom(BomParser::X00_3C) |
            State::Bom(BomParser::X00_3C_00) |
            State::Bom(BomParser::X3C) |
            State::Bom(BomParser::X3C_00) => Err(SyntaxError::UnclosedTag),
            // <?
            State::Bom(BomParser::X3C_3F) |
            State::Bom(BomParser::X3C_00_3F) |
            // <?x
            State::Bom(BomParser::X3C_3F_78) => Err(SyntaxError::UnclosedPIOrXmlDecl),
            // Threat unrecognized BOMs as text
            State::Bom(_) => Ok(()),

            State::Markup |
            State::StartOrEmptyTag(..) |
            State::EndTag => Err(SyntaxError::UnclosedTag),

            State::MaybeCommentOrCDataOrDoctype => Err(SyntaxError::InvalidBangMarkup),

            State::MaybeComment |
            State::Comment(_) => Err(SyntaxError::UnclosedComment),

            State::MaybeCData1 |
            State::MaybeCData2 |
            State::MaybeCData3 |
            State::MaybeCData4 |
            State::MaybeCData5 |
            State::MaybeCData6 |
            State::CData(_) => Err(SyntaxError::UnclosedCData),

            State::MaybeDoctype1 |
            State::MaybeDoctype2 |
            State::MaybeDoctype3 |
            State::MaybeDoctype4 |
            State::MaybeDoctype5 |
            State::MaybeDoctype6 |
            State::Doctype(_) |
            State::Dtd(_) |
            State::DoctypeFinish => Err(SyntaxError::UnclosedDoctype),

            State::PI(_) => Err(SyntaxError::UnclosedPIOrXmlDecl),
        }
    }

    /// Check if parser currently parses text
    #[inline]
    pub fn is_text_parsing(&self) -> bool {
        self.0 == State::Text
    }

    /// Text cannot contain `<` inside, so we emit it as soon as we find `<`.
    ///
    /// # Parameters
    /// - `bytes`: sub-slice to the original slice that was passed to `feed()`.
    ///   That sub-slice begins on the byte that represents a text content
    /// - `offset`: a position of `bytes` sub-slice in the one that was passed to `feed()`
    #[inline]
    fn parse_text(&mut self, bytes: &[u8], offset: usize) -> FeedResult {
        self.0 = State::Text;
        match bytes.iter().position(|&b| b == b'<') {
            Some(i) => FeedResult::EmitText(offset + i),
            None => FeedResult::NeedData,
        }
    }

    /// Determines the end position of a comment in the provided slice.
    /// Comment ends on the first occurrence of `-->` which cannot be escaped.
    ///
    /// # Parameters
    /// - `bytes`: sub-slice to the original slice that was passed to `feed()`.
    ///   That sub-slice begins on the byte that represents a comment content
    /// - `offset`: a position of `bytes` sub-slice in the one that was passed to `feed()`
    /// - `dashes_left`: count of dashes that wasn't seen yet in the end of previous data chunk
    fn parse_comment(
        &mut self,
        bytes: &[u8],
        offset: usize,
        mut parser: CommentParser,
    ) -> FeedResult {
        match parser.feed(bytes) {
            Some(i) => {
                self.0 = State::Text;
                FeedResult::EmitComment(offset + i)
            }
            None => {
                self.0 = State::Comment(parser);
                FeedResult::NeedData
            }
        }
    }

    /// Determines the end position of a CDATA block in the provided slice.
    /// CDATA block ends on the first occurrence of `]]>` which cannot be escaped.
    ///
    /// `<![CDATA[ ]]>` can contain `>` inside.
    ///
    /// # Parameters
    /// - `bytes`: sub-slice to the original slice that was passed to `feed()`.
    ///   That sub-slice begins on the byte that represents a CDATA content
    /// - `offset`: a position of `bytes` sub-slice in the one that was passed to `feed()`
    /// - `braces_left`: count of braces that wasn't seen yet in the end of previous data chunk
    fn parse_cdata(&mut self, bytes: &[u8], offset: usize, mut parser: CDataParser) -> FeedResult {
        match parser.feed(bytes) {
            Some(i) => {
                self.0 = State::Text;
                FeedResult::EmitCData(offset + i)
            }
            None => {
                self.0 = State::CData(parser);
                FeedResult::NeedData
            }
        }
    }

    fn parse_doctype(
        &mut self,
        bytes: &[u8],
        offset: usize,
        mut parser: QuotedParser,
    ) -> Result<FeedResult, SyntaxError> {
        // Search `[` (start of DTD definitions) or `>` (end of <!DOCTYPE> tag)
        match parser.one_of(bytes) {
            OneOf::Open(i) => self.parse_dtd(&bytes[i..], offset + i, DtdParser::default()),
            OneOf::Close(i) => {
                self.0 = State::Text;
                // +1 for `>` which should be included in event
                Ok(FeedResult::EmitDoctype(offset + i + 1))
            }
            OneOf::None => {
                self.0 = State::Doctype(parser);
                Ok(FeedResult::NeedData)
            }
        }
    }

    /// Skips DTD representation, correctly following DTD grammar.
    ///
    /// # Parameters
    /// - `bytes`: sub-slice to the original slice that was passed to `feed()`.
    ///   That sub-slice begins on a byte that would represent first byte of DTD event
    /// - `offset`: a position of `bytes` sub-slice in the one that was passed to `feed()`
    /// - `parser`: the DTD parser persisted between `feed()` calls
    fn parse_dtd(
        &mut self,
        mut bytes: &[u8],
        mut offset: usize,
        mut parser: DtdParser,
    ) -> Result<FeedResult, SyntaxError> {
        loop {
            let result = match parser.feed(bytes) {
                // Skip recognized DTD structure
                // TODO: Emit DTD events while parsing
                quick_dtd::FeedResult::EmitPI(off)
                | quick_dtd::FeedResult::EmitAttList(off)
                | quick_dtd::FeedResult::EmitComment(off)
                | quick_dtd::FeedResult::EmitElement(off)
                | quick_dtd::FeedResult::EmitEntity(off)
                | quick_dtd::FeedResult::EmitNotation(off) => {
                    bytes = &bytes[off..];
                    offset += off;
                    continue;
                }

                // `]` finishes DOCTYPE subsets: <!DOCTYPE name []>
                // After that we should find the close `>`
                quick_dtd::FeedResult::Unexpected(off, b']') => {
                    return Ok(self.parse_doctype_finish(&bytes[off..], offset + off))
                }
                // Other bytes not expected, so return error
                quick_dtd::FeedResult::Unexpected(..) => Err(SyntaxError::UnclosedDoctype),
                quick_dtd::FeedResult::NeedData => Ok(FeedResult::NeedData),
            };
            self.0 = State::Dtd(parser);
            return result;
        }
    }

    fn parse_doctype_finish(&mut self, bytes: &[u8], offset: usize) -> FeedResult {
        match bytes.iter().position(|&b| b == b'>') {
            Some(i) => {
                self.0 = State::Text;
                // +1 for `>` which should be included in event
                FeedResult::EmitDoctype(offset + i + 1)
            }
            None => {
                self.0 = State::DoctypeFinish;
                FeedResult::NeedData
            }
        }
    }

    /// Determines the end position of a processing instruction in the provided slice.
    /// Processing instruction ends on the first occurrence of `?>` which cannot be
    /// escaped.
    ///
    /// # Parameters
    /// - `bytes`: sub-slice to the original slice that was passed to `feed()`.
    ///   That sub-slice begins on the byte that represents a PI target
    /// - `offset`: a position of `bytes` sub-slice in the one that was passed to `feed()`
    /// - `has_mark`: a flag that indicates was the previous fed data ended with `?`
    fn parse_pi(&mut self, bytes: &[u8], offset: usize, mut parser: PiParser) -> FeedResult {
        match parser.feed(bytes) {
            Some(i) => {
                self.0 = State::Text;
                FeedResult::EmitPI(offset + i)
            }
            None => {
                self.0 = State::PI(parser);
                FeedResult::NeedData
            }
        }
    }

    /// Determines the end position of an end tag in the provided slice.
    ///
    /// # Parameters
    /// - `bytes`: sub-slice to the original slice that was passed to `feed()`.
    ///   That sub-slice begins on the byte that represents a tag name
    /// - `offset`: a position of `bytes` sub-slice in the one that was passed to `feed()`
    fn parse_end(&mut self, bytes: &[u8], offset: usize) -> FeedResult {
        match bytes.iter().position(|&b| b == b'>') {
            Some(i) => {
                self.0 = State::Text;
                // +1 for `>` which should be included in event
                FeedResult::EmitEndTag(offset + i + 1)
            }
            None => {
                self.0 = State::EndTag;
                FeedResult::NeedData
            }
        }
    }

    /// Determines the end position of a start or empty tag in the provided slice.
    ///
    /// # Parameters
    /// - `bytes`: sub-slice to the original slice that was passed to `feed()`.
    ///   That sub-slice begins on the byte that represents a second byte of
    ///   a tag name
    /// - `offset`: a position of `bytes` sub-slice in the one that was passed to `feed()`
    /// - `parser`: the state of a quotes used to skip `>` inside attribute values
    /// - `has_slash`: a flag that indicates was the previous fed data ended with `/`
    fn parse_start_or_empty(
        &mut self,
        bytes: &[u8],
        offset: usize,
        mut parser: QuotedParser,
        has_slash: bool,
    ) -> FeedResult {
        match parser.feed(bytes) {
            Some(0) if has_slash => {
                self.0 = State::Text;
                // +1 for `>` which should be included in event
                FeedResult::EmitEmptyTag(offset + 1)
            }
            Some(i) => {
                self.0 = State::Text;
                // This slash cannot follow immediately after `<`, because otherwise
                // we would be in a `parse_end` and not here
                if i > 0 && bytes[i - 1] == b'/' {
                    // +1 for `>` which should be included in event
                    FeedResult::EmitEmptyTag(offset + i + 1)
                } else {
                    // +1 for `>` which should be included in event
                    FeedResult::EmitStartTag(offset + i + 1)
                }
            }
            None => {
                self.0 = State::StartOrEmptyTag(parser, bytes.last().copied() == Some(b'/'));
                FeedResult::NeedData
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FeedResult::*;
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn text() {
        let mut parser = Parser::without_encoding_detection();
        assert_eq!(parser.feed(b"text with > symbol"), Ok(NeedData));
        assert_eq!(parser.0, State::Text);

        let mut parser = Parser::without_encoding_detection();
        assert_eq!(parser.feed(b"text with < symbol"), Ok(EmitText(10)));
        //                       ^^^^^^^^^^
        assert_eq!(parser.0, State::Text);
    }

    #[test]
    fn cdata() {
        let mut parser = Parser::without_encoding_detection();
        assert_eq!(parser.feed(b"<![CDATA[cdata"), Ok(NeedData));
        assert!(matches!(parser.0, State::CData(_)));
        assert_eq!(parser.feed(b"]"), Ok(NeedData));
        assert!(matches!(parser.0, State::CData(_)));
        assert_eq!(parser.feed(b"]"), Ok(NeedData));
        assert!(matches!(parser.0, State::CData(_)));
        assert_eq!(parser.feed(b">"), Ok(EmitCData(1)));
        assert_eq!(parser.0, State::Text);

        let mut parser = Parser::without_encoding_detection();
        assert_eq!(parser.feed(b"<![CDATA[cdata]"), Ok(NeedData));
        assert!(matches!(parser.0, State::CData(_)));
        assert_eq!(parser.feed(b"]>"), Ok(EmitCData(2)));
        assert_eq!(parser.0, State::Text);

        let mut parser = Parser::without_encoding_detection();
        assert_eq!(parser.feed(b"<![CDATA[cdata]]"), Ok(NeedData));
        assert!(matches!(parser.0, State::CData(_)));
        assert_eq!(parser.feed(b"><trail>"), Ok(EmitCData(1)));
        assert_eq!(parser.0, State::Text);

        let mut parser = Parser::without_encoding_detection();
        assert_eq!(
            parser.feed(b"<![CDATA[cdata content with ]] and ]> ]]>"),
            //            0                                       ^ = 40
            Ok(EmitCData(41))
        );
        assert_eq!(parser.0, State::Text);
    }

    #[test]
    fn comment() {
        let mut parser = Parser::without_encoding_detection();
        assert_eq!(parser.feed(b"<!--"), Ok(NeedData));
        assert!(matches!(parser.0, State::Comment(_)));
        assert_eq!(parser.feed(b"-"), Ok(NeedData));
        assert!(matches!(parser.0, State::Comment(_)));
        assert_eq!(parser.feed(b"-"), Ok(NeedData));
        assert!(matches!(parser.0, State::Comment(_)));
        assert_eq!(parser.feed(b">"), Ok(EmitComment(1)));
        assert_eq!(parser.0, State::Text);

        let mut parser = Parser::without_encoding_detection();
        assert_eq!(parser.feed(b"<!---"), Ok(NeedData));
        assert!(matches!(parser.0, State::Comment(_)));
        assert_eq!(parser.feed(b"->"), Ok(EmitComment(2)));
        assert_eq!(parser.0, State::Text);

        let mut parser = Parser::without_encoding_detection();
        assert_eq!(parser.feed(b"<!----"), Ok(NeedData));
        assert!(matches!(parser.0, State::Comment(_)));
        assert_eq!(parser.feed(b"><trail>"), Ok(EmitComment(1)));
        assert_eq!(parser.0, State::Text);

        let mut parser = Parser::without_encoding_detection();
        assert_eq!(parser.feed(b"<!-->"), Ok(NeedData));
        assert!(matches!(parser.0, State::Comment(_)));
        assert_eq!(parser.feed(b"-->"), Ok(EmitComment(3)));
        assert_eq!(parser.0, State::Text);

        let mut parser = Parser::without_encoding_detection();
        assert_eq!(
            parser.feed(b"<!--comment with >, -> and ---->"),
            //            0                              ^ = 31
            Ok(EmitComment(32))
        );
        assert_eq!(parser.0, State::Text);
    }

    mod doctype {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn only_name() {
            let mut parser = Parser::without_encoding_detection();
            assert_eq!(parser.feed(b"<!DOCTYPE name>"), Ok(EmitDoctype(15)));
            //                       0             ^ = 14
            assert_eq!(parser.0, State::Text);
        }

        #[test]
        fn with_external_id() {
            let mut parser = Parser::without_encoding_detection();
            assert_eq!(
                parser.feed(b"<!DOCTYPE with SYSTEM \"[>']\">"),
                //            0                             ^ = 28
                Ok(EmitDoctype(29))
            );
            assert_eq!(parser.0, State::Text);

            let mut parser = Parser::without_encoding_detection();
            assert_eq!(
                parser.feed(b"<!DOCTYPE with SYSTEM '[>\"]'>"),
                //            0                            ^ = 28
                Ok(EmitDoctype(29))
            );
            assert_eq!(parser.0, State::Text);

            let mut parser = Parser::without_encoding_detection();
            assert_eq!(
                parser.feed(b"<!DOCTYPE with PUBLIC \"'\" '[>\"]'>"),
                //            0                                  ^ = 32
                Ok(EmitDoctype(33))
            );
            assert_eq!(parser.0, State::Text);

            let mut parser = Parser::without_encoding_detection();
            assert_eq!(
                parser.feed(b"<!DOCTYPE with PUBLIC '' \"[>']\">"),
                //            0                                ^ = 31
                Ok(EmitDoctype(32))
            );
            assert_eq!(parser.0, State::Text);
        }

        #[test]
        fn with_subset() {
            let mut parser = Parser::without_encoding_detection();
            assert_eq!(
                parser.feed(b"<!DOCTYPE with [<!ENTITY gt '>'>]>"),
                //            0                                ^ = 33
                Ok(EmitDoctype(34))
            );
            assert_eq!(parser.0, State::Text);

            let mut parser = Parser::without_encoding_detection();
            assert_eq!(
                parser.feed(b"<!DOCTYPE with SYSTEM \">'\" []>"),
                //            0                              ^ = 29
                Ok(EmitDoctype(30))
            );
            assert_eq!(parser.0, State::Text);

            let mut parser = Parser::without_encoding_detection();
            assert_eq!(
                parser.feed(b"<!DOCTYPE with SYSTEM '>\"' []>"),
                //            0                             ^ = 29
                Ok(EmitDoctype(30))
            );
            assert_eq!(parser.0, State::Text);

            let mut parser = Parser::without_encoding_detection();
            assert_eq!(
                parser.feed(b"<!DOCTYPE with PUBLIC \"'\" '>\"' []>"),
                //            0                                   ^ = 33
                Ok(EmitDoctype(34))
            );
            assert_eq!(parser.0, State::Text);

            let mut parser = Parser::without_encoding_detection();
            assert_eq!(
                parser.feed(b"<!DOCTYPE with PUBLIC '' \">'\" []>"),
                //            0                                 ^ = 32
                Ok(EmitDoctype(33))
            );
            assert_eq!(parser.0, State::Text);
        }
    }

    #[test]
    fn pi() {
        let mut parser = Parser::without_encoding_detection();
        assert_eq!(parser.feed(b"<??>"), Ok(EmitPI(4)));
        assert_eq!(parser.0, State::Text);

        let mut parser = Parser::without_encoding_detection();
        assert_eq!(parser.feed(b"<?target?>"), Ok(EmitPI(10)));
        assert_eq!(parser.0, State::Text);

        let mut parser = Parser::without_encoding_detection();
        assert_eq!(parser.feed(b"<?>?>"), Ok(EmitPI(5)));
        assert_eq!(parser.0, State::Text);

        let mut parser = Parser::without_encoding_detection();
        assert_eq!(parser.feed(b"<???>"), Ok(EmitPI(5)));
        assert_eq!(parser.0, State::Text);
    }

    #[test]
    fn empty() {
        let mut parser = Parser::without_encoding_detection();
        assert_eq!(parser.feed(b"<empty/>"), Ok(EmitEmptyTag(8)));
        assert_eq!(parser.0, State::Text);

        let mut parser = Parser::without_encoding_detection();
        assert_eq!(
            parser.feed(b"<empty one=\"'/>\" two='\"/>'/>"),
            Ok(EmitEmptyTag(28))
        );
        assert_eq!(parser.0, State::Text);
    }

    #[test]
    fn start() {
        let mut parser = Parser::without_encoding_detection();
        assert_eq!(parser.feed(b"<>"), Ok(EmitStartTag(2)));
        assert_eq!(parser.0, State::Text);

        let mut parser = Parser::without_encoding_detection();
        assert_eq!(parser.feed(b"<start>"), Ok(EmitStartTag(7)));
        assert_eq!(parser.0, State::Text);

        let mut parser = Parser::without_encoding_detection();
        assert_eq!(
            parser.feed(b"<start one=\"'>\" two='\">'>"),
            Ok(EmitStartTag(25))
        );
        assert_eq!(parser.0, State::Text);
    }

    #[test]
    fn end() {
        let mut parser = Parser::without_encoding_detection();
        assert_eq!(parser.feed(b"</end>"), Ok(EmitEndTag(6)));
        assert_eq!(parser.0, State::Text);

        let mut parser = Parser::without_encoding_detection();
        assert_eq!(parser.feed(b"</ \r\n\t>"), Ok(EmitEndTag(7)));
        assert_eq!(parser.0, State::Text);

        let mut parser = Parser::without_encoding_detection();
        assert_eq!(parser.feed(b"</>"), Ok(EmitEndTag(3)));
        assert_eq!(parser.0, State::Text);
    }
}

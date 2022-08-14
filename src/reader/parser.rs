#[cfg(feature = "encoding")]
use encoding_rs::UTF_8;

use crate::encoding::Decoder;
use crate::errors::{Error, Result};
use crate::events::{BytesCData, BytesDecl, BytesEnd, BytesStart, BytesText, Event};
#[cfg(feature = "encoding")]
use crate::reader::EncodingRef;
use crate::reader::{is_whitespace, BangType, ParseState};

use memchr;

/// A struct that holds a current parse state and a parser configuration.
/// It is independent on a way of reading data: the reader feed data into it and
/// get back produced [`Event`]s.
#[derive(Clone)]
pub(super) struct Parser {
    /// Number of bytes read from the source of data since the parser was created
    pub offset: usize,
    /// Defines how to process next byte
    pub state: ParseState,
    /// Expand empty element into an opening and closing element
    pub expand_empty_elements: bool,
    /// Trims leading whitespace in Text events, skip the element if text is empty
    pub trim_text_start: bool,
    /// Trims trailing whitespace in Text events.
    pub trim_text_end: bool,
    /// Trims trailing whitespaces from markup names in closing tags `</a >`
    pub trim_markup_names_in_closing_tags: bool,
    /// Check if [`Event::End`] nodes match last [`Event::Start`] node
    pub check_end_names: bool,
    /// Check if comments contains `--` (false per default)
    pub check_comments: bool,
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
    opened_buffer: String,
    /// Opened name start indexes into [`Self::opened_buffer`]. See documentation
    /// for that field for details
    opened_starts: Vec<usize>,

    #[cfg(feature = "encoding")]
    /// Reference to the encoding used to read an XML
    pub encoding: EncodingRef,
}

// TODO: str::from_utf8() can in the future be replaced by str::from_utf8_unchecked() as
// decoding ensures that all underlying bytes are UTF-8 and the parser can ensure that
// slices happen at character boundaries

impl Parser {
    /// Trims whitespaces from `bytes`, if required, and returns a [`Text`] event.
    ///
    /// # Parameters
    /// - `bytes`: data from the start of stream to the first `<` or from `>` to `<`
    ///
    /// [`Text`]: Event::Text
    pub fn emit_text<'b>(&mut self, bytes: &'b [u8]) -> Result<Event<'b>> {
        let mut content = bytes;

        if self.trim_text_end {
            // Skip the ending '<'
            let len = bytes
                .iter()
                .rposition(|&b| !is_whitespace(b))
                .map_or_else(|| bytes.len(), |p| p + 1);
            content = &bytes[..len];
        }

        Ok(Event::Text(BytesText::from_escaped(
            std::str::from_utf8(content).unwrap(),
        )))
    }

    /// reads `BytesElement` starting with a `!`,
    /// return `Comment`, `CData` or `DocType` event
    pub fn emit_bang<'b>(&mut self, bang_type: BangType, buf: &'b [u8]) -> Result<Event<'b>> {
        let uncased_starts_with = |string: &str, prefix: &str| {
            string.len() >= prefix.len() && string[..prefix.len()].eq_ignore_ascii_case(prefix)
        };

        let len = buf.len();
        let buf = std::str::from_utf8(buf).unwrap();
        match bang_type {
            BangType::Comment if buf.starts_with("!--") => {
                debug_assert!(buf.ends_with("--"));
                if self.check_comments {
                    // search if '--' not in comments
                    if let Some(p) = memchr::memchr_iter(b'-', &buf[3..len - 2].as_bytes())
                        .position(|p| buf.bytes().nth(3 + p + 1) == Some(b'-'))
                    {
                        self.offset += len - p;
                        return Err(Error::UnexpectedToken("--".to_string()));
                    }
                }
                Ok(Event::Comment(BytesText::new(&buf[3..len - 2])))
            }
            BangType::CData if uncased_starts_with(buf, "![CDATA[") => {
                debug_assert!(buf.ends_with("]]"));
                Ok(Event::CData(BytesCData::new(&buf[8..len - 2])))
            }
            BangType::DocType if uncased_starts_with(buf, "!DOCTYPE") => {
                let start = buf[8..]
                    .bytes()
                    .position(|b| !is_whitespace(b))
                    .unwrap_or_else(|| len - 8);
                if start + 8 >= len {
                    return Err(Error::EmptyDocType);
                }
                Ok(Event::DocType(BytesText::new(&buf[8 + start..])))
            }
            _ => Err(bang_type.to_err()),
        }
    }

    /// Wraps content of `buf` into the [`Event::End`] event. Does the check that
    /// end name matches the last opened start name if `self.check_end_names` is set.
    pub fn emit_end<'b>(&mut self, buf: &'b [u8]) -> Result<Event<'b>> {
        // XML standard permits whitespaces after the markup name in closing tags.
        // Let's strip them from the buffer before comparing tag names.
        let buf = std::str::from_utf8(buf).unwrap();

        let name = if self.trim_markup_names_in_closing_tags {
            if let Some(pos_end_name) = buf[1..].bytes().rposition(|b| !b.is_ascii_whitespace()) {
                let (name, _) = buf[1..].split_at(pos_end_name + 1);
                name
            } else {
                &buf[1..]
            }
        } else {
            &buf[1..]
        };

        let mismatch_err = |expected: String, found: &str, offset: &mut usize| {
            *offset -= buf.len();
            Err(Error::EndEventMismatch {
                expected,
                found: found.to_owned(),
            })
        };

        // Get the index in self.opened_buffer of the name of the last opened tag
        match self.opened_starts.pop() {
            Some(start) => {
                if self.check_end_names {
                    let expected = &self.opened_buffer[start..];
                    if name != expected {
                        let expected = expected.to_owned();
                        // #513: In order to allow error recovery we should drop content of the buffer
                        self.opened_buffer.truncate(start);

                        return mismatch_err(expected, name, &mut self.offset);
                    }
                }

                self.opened_buffer.truncate(start);
            }
            None => {
                if self.check_end_names {
                    return mismatch_err("".to_string(), &buf[1..], &mut self.offset);
                }
            }
        }

        Ok(Event::End(BytesEnd::new(name)))
    }

    /// reads `BytesElement` starting with a `?`,
    /// return `Decl` or `PI` event
    pub fn emit_question_mark<'b>(&mut self, buf: &'b [u8]) -> Result<Event<'b>> {
        let buf = std::str::from_utf8(buf).unwrap();
        let len = buf.len();
        if len > 2 && buf.bytes().nth(len - 1) == Some(b'?') {
            if len > 5 && &buf[1..4] == "xml" && is_whitespace(buf.bytes().nth(4).unwrap()) {
                let event = BytesDecl::from_start(BytesStart::from_content(&buf[1..len - 1], 3));

                // Try getting encoding from the declaration event
                #[cfg(feature = "encoding")]
                if self.encoding.can_be_refined() {
                    if let Some(encoding) = event.encoder() {
                        self.encoding = EncodingRef::XmlDetected(encoding);
                    }
                }

                Ok(Event::Decl(event))
            } else {
                Ok(Event::PI(BytesText::new(&buf[1..len - 1])))
            }
        } else {
            self.offset -= len;
            Err(Error::UnexpectedEof("XmlDecl".to_string()))
        }
    }

    /// Converts content of a tag to a `Start` or an `Empty` event
    ///
    /// # Parameters
    /// - `content`: Content of a tag between `<` and `>`
    pub fn emit_start<'b>(&mut self, content: &'b [u8]) -> Result<Event<'b>> {
        // TODO: do this directly when reading bufreader ...
        let len = content.len();
        let content = std::str::from_utf8(content).unwrap();
        let name_end = content
            .bytes()
            .position(|b| is_whitespace(b))
            .unwrap_or(len);
        if let Some(b'/') = content.bytes().last() {
            // This is self-closed tag `<something/>`
            let name_len = if name_end < len { name_end } else { len - 1 };
            let event = BytesStart::from_content(&content[..len - 1], name_len);

            if self.expand_empty_elements {
                self.state = ParseState::Empty;
                self.opened_starts.push(self.opened_buffer.len());
                self.opened_buffer.push_str(&content[..name_len]);
                Ok(Event::Start(event))
            } else {
                Ok(Event::Empty(event))
            }
        } else {
            // #514: Always store names event when .check_end_names == false,
            // because checks can be temporary disabled and when they would be
            // enabled, we should have that information
            self.opened_starts.push(self.opened_buffer.len());
            self.opened_buffer.push_str(&content[..name_end]);
            Ok(Event::Start(BytesStart::from_content(content, name_end)))
        }
    }

    #[inline]
    pub fn close_expanded_empty(&mut self) -> Result<Event<'static>> {
        self.state = ParseState::ClosedTag;
        let name = self
            .opened_buffer
            .split_off(self.opened_starts.pop().unwrap());
        Ok(Event::End(BytesEnd::new(name)))
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

impl Default for Parser {
    fn default() -> Self {
        Self {
            offset: 0,
            state: ParseState::Init,
            expand_empty_elements: false,
            trim_text_start: false,
            trim_text_end: false,
            trim_markup_names_in_closing_tags: true,
            check_end_names: true,
            check_comments: false,
            opened_buffer: String::new(),
            opened_starts: Vec::new(),

            #[cfg(feature = "encoding")]
            encoding: EncodingRef::Implicit(UTF_8),
        }
    }
}

#[cfg(feature = "encoding")]
use encoding_rs::UTF_8;

use crate::encoding::Decoder;
use crate::errors::{Error, IllFormedError, Result, SyntaxError};
use crate::events::{BytesCData, BytesDecl, BytesEnd, BytesStart, BytesText, Event};
#[cfg(feature = "encoding")]
use crate::reader::EncodingRef;
use crate::reader::{is_whitespace, BangType, Config, ParseState};

use memchr;

/// A struct that holds a current reader state and a parser configuration.
/// It is independent on a way of reading data: the reader feed data into it and
/// get back produced [`Event`]s.
#[derive(Clone, Debug)]
pub(super) struct ReaderState {
    /// Number of bytes read from the source of data since the reader was created
    pub offset: usize,
    /// Defines how to process next byte
    pub state: ParseState,
    /// User-defined settings that affect parsing
    pub config: Config,
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
    pub encoding: EncodingRef,
}

impl ReaderState {
    /// Trims end whitespaces from `bytes`, if required, and returns a [`Text`]
    /// event or an [`Eof`] event, if text after trimming is empty.
    ///
    /// # Parameters
    /// - `bytes`: data from the start of stream to the first `<` or from `>` to `<`
    ///
    /// [`Text`]: Event::Text
    /// [`Eof`]: Event::Eof
    pub fn emit_text<'b>(&mut self, bytes: &'b [u8]) -> Result<Event<'b>> {
        let mut content = bytes;

        if self.config.trim_text_end {
            // Skip the ending '<'
            let len = bytes
                .iter()
                .rposition(|&b| !is_whitespace(b))
                .map_or_else(|| bytes.len(), |p| p + 1);
            content = &bytes[..len];
        }

        if content.is_empty() {
            Ok(Event::Eof)
        } else {
            Ok(Event::Text(BytesText::wrap(content, self.decoder())))
        }
    }

    /// reads `BytesElement` starting with a `!`,
    /// return `Comment`, `CData` or `DocType` event
    pub fn emit_bang<'b>(&mut self, bang_type: BangType, buf: &'b [u8]) -> Result<Event<'b>> {
        let uncased_starts_with = |string: &[u8], prefix: &[u8]| {
            string.len() >= prefix.len() && string[..prefix.len()].eq_ignore_ascii_case(prefix)
        };

        let len = buf.len();
        match bang_type {
            BangType::Comment if buf.starts_with(b"!--") => {
                debug_assert!(buf.ends_with(b"--"));
                if self.config.check_comments {
                    // search if '--' not in comments
                    let mut haystack = &buf[3..len - 2];
                    let mut off = 0;
                    while let Some(p) = memchr::memchr(b'-', haystack) {
                        off += p + 1;
                        // if next byte after `-` is also `-`, return an error
                        if buf[3 + off] == b'-' {
                            self.offset -= len - 2 - p;
                            return Err(Error::IllFormed(IllFormedError::DoubleHyphenInComment));
                        }
                        haystack = &haystack[p + 1..];
                    }
                }
                Ok(Event::Comment(BytesText::wrap(
                    &buf[3..len - 2],
                    self.decoder(),
                )))
            }
            BangType::CData if uncased_starts_with(buf, b"![CDATA[") => {
                debug_assert!(buf.ends_with(b"]]"));
                Ok(Event::CData(BytesCData::wrap(
                    &buf[8..len - 2],
                    self.decoder(),
                )))
            }
            BangType::DocType if uncased_starts_with(buf, b"!DOCTYPE") => {
                match buf[8..].iter().position(|&b| !is_whitespace(b)) {
                    Some(start) => Ok(Event::DocType(BytesText::wrap(
                        &buf[8 + start..],
                        self.decoder(),
                    ))),
                    None => {
                        // Because we here, we at least read `<!DOCTYPE>` and offset after `>`.
                        // We want report error at place where name is expected - this is just
                        // before `>`
                        self.offset -= 1;
                        return Err(Error::IllFormed(IllFormedError::MissingDoctypeName));
                    }
                }
            }
            _ => {
                // <!....>
                //  ^^^^^ - `buf` does not contain `<` and `>`, but `self.offset` is after `>`.
                // ^------- We report error at that position, so we need to subtract 2 and buf len
                self.offset -= len + 2;
                Err(bang_type.to_err())
            }
        }
    }

    /// Wraps content of `buf` into the [`Event::End`] event. Does the check that
    /// end name matches the last opened start name if `self.config.check_end_names` is set.
    pub fn emit_end<'b>(&mut self, buf: &'b [u8]) -> Result<Event<'b>> {
        // Strip the `/` character. `content` contains data between `</` and `>`
        let content = &buf[1..];
        // XML standard permits whitespaces after the markup name in closing tags.
        // Let's strip them from the buffer before comparing tag names.
        let name = if self.config.trim_markup_names_in_closing_tags {
            if let Some(pos_end_name) = content.iter().rposition(|&b| !is_whitespace(b)) {
                &content[..pos_end_name + 1]
            } else {
                content
            }
        } else {
            content
        };

        let decoder = self.decoder();

        // Get the index in self.opened_buffer of the name of the last opened tag
        match self.opened_starts.pop() {
            Some(start) => {
                if self.config.check_end_names {
                    let expected = &self.opened_buffer[start..];
                    if name != expected {
                        let expected = decoder.decode(expected).unwrap_or_default().into_owned();
                        // #513: In order to allow error recovery we should drop content of the buffer
                        self.opened_buffer.truncate(start);

                        // Report error at start of the end tag at `<` character
                        // +2 for `<` and `>`
                        self.offset -= buf.len() + 2;
                        return Err(Error::IllFormed(IllFormedError::MismatchedEndTag {
                            expected,
                            found: decoder.decode(name).unwrap_or_default().into_owned(),
                        }));
                    }
                }

                self.opened_buffer.truncate(start);
            }
            None => {
                // Report error at start of the end tag at `<` character
                // +2 for `<` and `>`
                self.offset -= buf.len() + 2;
                return Err(Error::IllFormed(IllFormedError::UnmatchedEndTag(
                    decoder.decode(name).unwrap_or_default().into_owned(),
                )));
            }
        }

        Ok(Event::End(BytesEnd::wrap(name.into())))
    }

    /// `buf` contains data between `<` and `>` and the first byte is `?`.
    /// `self.offset` already after the `>`
    ///
    /// Returns `Decl` or `PI` event
    pub fn emit_question_mark<'b>(&mut self, buf: &'b [u8]) -> Result<Event<'b>> {
        debug_assert!(buf.len() > 0);
        debug_assert_eq!(buf[0], b'?');

        let len = buf.len();
        // We accept at least <??>
        //                     ~~ - len = 2
        if len > 1 && buf[len - 1] == b'?' {
            let content = &buf[1..len - 1];
            let len = content.len();

            if content.starts_with(b"xml") && (len == 3 || is_whitespace(content[3])) {
                let event = BytesDecl::from_start(BytesStart::wrap(content, 3));

                // Try getting encoding from the declaration event
                #[cfg(feature = "encoding")]
                if self.encoding.can_be_refined() {
                    if let Some(encoding) = event.encoder() {
                        self.encoding = EncodingRef::XmlDetected(encoding);
                    }
                }

                Ok(Event::Decl(event))
            } else {
                Ok(Event::PI(BytesText::wrap(content, self.decoder())))
            }
        } else {
            // <?....EOF
            //  ^^^^^ - `buf` does not contains `<`, but we want to report error at `<`,
            //          so we move offset to it (+2 for `<`and `>`)
            self.offset -= len + 2;
            Err(Error::Syntax(SyntaxError::UnclosedPIOrXmlDecl))
        }
    }

    /// Converts content of a tag to a `Start` or an `Empty` event
    ///
    /// # Parameters
    /// - `content`: Content of a tag between `<` and `>`
    pub fn emit_start<'b>(&mut self, content: &'b [u8]) -> Result<Event<'b>> {
        let len = content.len();
        let name_end = content
            .iter()
            .position(|&b| is_whitespace(b))
            .unwrap_or(len);
        if let Some(&b'/') = content.last() {
            // This is self-closed tag `<something/>`
            let name_len = if name_end < len { name_end } else { len - 1 };
            let event = BytesStart::wrap(&content[..len - 1], name_len);

            if self.config.expand_empty_elements {
                self.state = ParseState::Empty;
                self.opened_starts.push(self.opened_buffer.len());
                self.opened_buffer.extend(&content[..name_len]);
                Ok(Event::Start(event))
            } else {
                Ok(Event::Empty(event))
            }
        } else {
            // #514: Always store names event when .check_end_names == false,
            // because checks can be temporary disabled and when they would be
            // enabled, we should have that information
            self.opened_starts.push(self.opened_buffer.len());
            self.opened_buffer.extend(&content[..name_end]);
            Ok(Event::Start(BytesStart::wrap(content, name_end)))
        }
    }

    #[inline]
    pub fn close_expanded_empty(&mut self) -> Result<Event<'static>> {
        self.state = ParseState::ClosedTag;
        let name = self
            .opened_buffer
            .split_off(self.opened_starts.pop().unwrap());
        Ok(Event::End(BytesEnd::wrap(name.into())))
    }

    /// Get the decoder, used to decode bytes, read by this reader, to the strings.
    ///
    /// If [`encoding`] feature is enabled, the used encoding may change after
    /// parsing the XML declaration, otherwise encoding is fixed to UTF-8.
    ///
    /// If [`encoding`] feature is enabled and no encoding is specified in declaration,
    /// defaults to UTF-8.
    ///
    /// [`encoding`]: ../../index.html#encoding
    pub fn decoder(&self) -> Decoder {
        Decoder {
            #[cfg(feature = "encoding")]
            encoding: self.encoding.encoding(),
        }
    }
}

impl Default for ReaderState {
    fn default() -> Self {
        Self {
            offset: 0,
            state: ParseState::Init,
            config: Config::default(),
            opened_buffer: Vec::new(),
            opened_starts: Vec::new(),

            #[cfg(feature = "encoding")]
            encoding: EncodingRef::Implicit(UTF_8),
        }
    }
}

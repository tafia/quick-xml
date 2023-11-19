#[cfg(feature = "encoding")]
use encoding_rs::{UTF_16BE, UTF_16LE, UTF_8};

use crate::encoding::Decoder;
use crate::errors::{Error, IllFormedError, Result};
use crate::events::{BytesCData, BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use crate::parser::{FeedResult, Parser};
#[cfg(feature = "encoding")]
use crate::reader::EncodingRef;
use crate::reader::{is_whitespace, Config};
use crate::utils::Bytes;

use memchr;

/// Result of a [`ReaderState::parse_into`] method.
#[derive(Debug)]
pub enum ParseOutcome {
    /// The specified amount of data should be consumed. The parser result should
    /// be converted to an [`Event`] using previously accumulated data and newly
    /// consumed data.
    Consume(usize, FeedResult),
    /// The specified amount of data should be consumed. All accumulated data
    /// and newly consumed data should be converted to an [`Event::Text`].
    ConsumeAndEmitText(usize),
    /// The specified amount of data should be consumed, but no event should be
    /// generated. Used to skip whitespaces and BOM.
    ConsumeAndContinue(usize),
}

/// A struct that holds a current reader state and a parser configuration.
/// It is independent on a way of reading data: the reader feed data into it and
/// get back produced [`Event`]s.
#[derive(Clone, Debug)]
pub(super) struct ReaderState {
    /// Current parsing state
    pub parser: Parser,
    /// Number of bytes read from the source of data since the reader was created
    pub offset: usize,
    /// A snapshot of an `offset` of the last error returned. It can be less than
    /// `offset`, because some errors conveniently report at earlier position,
    /// and changing `offset` is not possible, because `Error::IllFormed` errors
    /// are recoverable.
    pub last_error_offset: usize,
    /// User-defined settings that affect parsing
    pub config: Config,
    /// When text trimming from start is enabled, we need to track is we seen
    /// a non-space symbol between getting chunks from the reader, because we
    /// trim each chunk individually. If such symbol was seen, trim is not
    /// required until current text event would be emitted.
    ///
    /// Used only together with buffering readers, because borrowing reader
    /// already have all data available.
    can_trim_start: bool,
    /// If case of [`Config::expand_empty_elements`] is true, this field will
    /// be `true` if synthetic end event should be emitted on next call to read
    /// event.
    pending: bool,
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
                        // -2 for `<` and `>`
                        self.last_error_offset = self.offset - buf.len() - 2;
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
                // -2 for `<` and `>`
                self.last_error_offset = self.offset - buf.len() - 2;
                return Err(Error::IllFormed(IllFormedError::UnmatchedEndTag(
                    decoder.decode(name).unwrap_or_default().into_owned(),
                )));
            }
        }

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

    /// Parses `bytes`, appending data to a `buf`. Used in buffered readers
    pub fn parse_into<'a, 'b>(
        &mut self,
        bytes: &'a [u8],
        buf: &'b mut Vec<u8>,
    ) -> Result<ParseOutcome> {
        let result = self.parser.feed(bytes)?;
        match result {
            FeedResult::NeedData => {
                let mut content = bytes;
                if self.config.trim_text_start
                    && self.can_trim_start
                    && self.parser.is_text_parsing()
                {
                    content = crate::events::trim_xml_start(bytes);
                    // if we got some data while parsing text, we shouldn't to
                    // trim text anymore, because this is spaces inside text content
                    self.can_trim_start = content.is_empty();
                }
                buf.extend_from_slice(content);
                let len = bytes.len();
                self.offset += len;
                Ok(ParseOutcome::ConsumeAndContinue(len))
            }

            FeedResult::EncodingUtf8Like(offset) => {
                #[cfg(feature = "encoding")]
                if self.encoding.can_be_refined() {
                    self.encoding = EncodingRef::BomDetected(UTF_8);
                }
                self.offset += offset;
                Ok(ParseOutcome::ConsumeAndContinue(offset))
            }
            FeedResult::EncodingUtf16BeLike(offset) => {
                #[cfg(feature = "encoding")]
                if self.encoding.can_be_refined() {
                    self.encoding = EncodingRef::BomDetected(UTF_16BE);
                }
                self.offset += offset;
                Ok(ParseOutcome::ConsumeAndContinue(offset))
            }
            FeedResult::EncodingUtf16LeLike(offset) => {
                #[cfg(feature = "encoding")]
                if self.encoding.can_be_refined() {
                    self.encoding = EncodingRef::BomDetected(UTF_16LE);
                }
                self.offset += offset;
                Ok(ParseOutcome::ConsumeAndContinue(offset))
            }

            FeedResult::EmitText(offset) => {
                let mut content = &bytes[..offset];
                if self.config.trim_text_start && self.can_trim_start {
                    content = crate::events::trim_xml_start(content);
                }
                // Reset ability to trim start
                self.can_trim_start = true;
                if self.config.trim_text_end {
                    content = crate::events::trim_xml_end(content);
                }
                buf.extend_from_slice(content);
                self.offset += offset;
                if buf.is_empty() {
                    Ok(ParseOutcome::ConsumeAndContinue(offset))
                } else {
                    Ok(ParseOutcome::ConsumeAndEmitText(offset))
                }
            }
            FeedResult::EmitComment(offset)
            | FeedResult::EmitCData(offset)
            | FeedResult::EmitDoctype(offset)
            | FeedResult::EmitPI(offset)
            | FeedResult::EmitEmptyTag(offset)
            | FeedResult::EmitStartTag(offset)
            | FeedResult::EmitEndTag(offset) => {
                buf.extend_from_slice(&bytes[..offset]);
                self.offset += offset;
                Ok(ParseOutcome::Consume(offset, result))
            }
        }
    }

    /// Converts result from a parser to reader's event.
    ///
    /// # Parameters
    /// - `result`: a result from [`Parser::feed()`]
    /// - `content`: a buffer with event data
    ///
    /// [`Parser::feed()`]: crate::parser::Parser::feed()
    pub fn make_event<'a>(&mut self, result: FeedResult, content: &'a [u8]) -> Result<Event<'a>> {
        debug_assert!(!self.pending, "synthetic end event won't be emitted");

        match result {
            FeedResult::EmitText(_) | FeedResult::NeedData => {
                Ok(Event::Text(BytesText::wrap(content, self.decoder())))
            }
            FeedResult::EmitCData(_) => {
                debug_assert!(content.starts_with(b"<![CDATA["), "{:?}", Bytes(content));
                debug_assert!(content.ends_with(b"]]>"), "{:?}", Bytes(content));

                Ok(Event::CData(BytesCData::wrap(
                    &content[9..content.len() - 3],
                    self.decoder(),
                )))
            }
            FeedResult::EmitComment(_) => {
                // `--` from start and end should not be overlapped
                debug_assert!(content.len() >= 4 + 3, "{:?}", Bytes(content));
                debug_assert!(content.starts_with(b"<!--"), "{:?}", Bytes(content));
                debug_assert!(content.ends_with(b"-->"), "{:?}", Bytes(content));

                let len = content.len();
                if self.config.check_comments {
                    // search if '--' not in comments
                    // Skip `<!--` and `-->`
                    let mut haystack = &content[4..len - 3];
                    let mut off = 0;
                    while let Some(p) = memchr::memchr(b'-', haystack) {
                        off += p + 1;
                        // if next byte after `-` is also `-`, return an error
                        if content[4 + off] == b'-' {
                            // Explanation of the magic:
                            //
                            // - `self.offset`` just after `>`,
                            // - `buf` contains `!-- con--tent --`
                            // - `p` is counted from byte after `<!--`
                            //
                            // <!-- con--tent -->:
                            // ~~~~~~~~~~~~~~~~~~: - buf
                            //   : ===========   : - zone of search (possible values of `p`)
                            //   : |---p         : - p is counted from | (| is 0)
                            //   : :   :         ^ - self.offset
                            // ^ :     :           - self.offset - len
                            //     ^   :           - self.offset - len + 4
                            //         ^           - self.offset - len + 4 + p
                            self.last_error_offset = self.offset - len + 4 + p;
                            return Err(Error::IllFormed(IllFormedError::DoubleHyphenInComment));
                        }
                        haystack = &haystack[p + 1..];
                    }
                }
                Ok(Event::Comment(BytesText::wrap(
                    &content[4..len - 3],
                    self.decoder(),
                )))
            }
            FeedResult::EmitDoctype(_) => {
                debug_assert!(content.len() > 9, "{:?}", Bytes(content));
                debug_assert!(
                    content[0..9].eq_ignore_ascii_case(b"<!DOCTYPE"),
                    "{:?}",
                    Bytes(content)
                );
                debug_assert!(content.ends_with(b">"), "{:?}", Bytes(content));

                // Skip `<!DOCTYPE` and `>`
                let buf = &content[9..content.len() - 1];
                match buf.iter().position(|&b| !is_whitespace(b)) {
                    // Found the first non-space symbol after `<!DOCTYPE`
                    // Actually, parser will guarantee, that after `<!DOCTYPE`
                    // at least one is_whitespace() symbol
                    Some(start) => Ok(Event::DocType(BytesText::wrap(
                        &buf[start..],
                        self.decoder(),
                    ))),
                    None => {
                        // Because we here, we at least read `<!DOCTYPE>` and offset after `>`.
                        // We want report error at place where name is expected - this is just
                        // before `>`
                        self.last_error_offset = self.offset - 1;
                        return Err(Error::IllFormed(IllFormedError::MissingDoctypeName));
                    }
                }
            }
            FeedResult::EmitPI(_) => {
                debug_assert!(content.starts_with(b"<?"), "{:?}", Bytes(content));
                debug_assert!(content.ends_with(b"?>"), "{:?}", Bytes(content));

                // Cut of `<?` and `?>` from start and end
                let content = &content[2..content.len() - 2];
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
            }
            FeedResult::EmitEmptyTag(_) => {
                debug_assert!(content.starts_with(b"<"), "{:?}", Bytes(content));
                debug_assert!(content.ends_with(b"/>"), "{:?}", Bytes(content));

                let content = &content[1..content.len() - 1];
                let len = content.len();
                let name_end = content
                    .iter()
                    .position(|&b| is_whitespace(b))
                    .unwrap_or(len);
                // This is self-closed tag `<something/>`
                let name_len = if name_end < len { name_end } else { len - 1 };
                let event = BytesStart::wrap(&content[..len - 1], name_len);

                if self.config.expand_empty_elements {
                    self.pending = true;
                    self.opened_starts.push(self.opened_buffer.len());
                    self.opened_buffer.extend(&content[..name_len]);
                    Ok(Event::Start(event))
                } else {
                    Ok(Event::Empty(event))
                }
            }
            FeedResult::EmitStartTag(_) => {
                debug_assert!(content.starts_with(b"<"), "{:?}", Bytes(content));
                debug_assert!(content.ends_with(b">"), "{:?}", Bytes(content));

                let content = &content[1..content.len() - 1];
                let len = content.len();
                let name_end = content
                    .iter()
                    .position(|&b| is_whitespace(b))
                    .unwrap_or(len);
                // #514: Always store names event when .check_end_names == false,
                // because checks can be temporary disabled and when they would be
                // enabled, we should have that information
                self.opened_starts.push(self.opened_buffer.len());
                self.opened_buffer.extend(&content[..name_end]);
                Ok(Event::Start(BytesStart::wrap(content, name_end)))
            }
            FeedResult::EmitEndTag(_) => {
                debug_assert!(content.starts_with(b"</"), "{:?}", Bytes(content));
                debug_assert!(content.ends_with(b">"), "{:?}", Bytes(content));

                self.emit_end(&content[1..content.len() - 1])
            }
            FeedResult::EncodingUtf8Like(_)
            | FeedResult::EncodingUtf16BeLike(_)
            | FeedResult::EncodingUtf16LeLike(_) => unreachable!("processed outside"),
        }
    }

    /// Get the pending event if the last returned event was a synthetic `Start`
    /// event due to [`Config::expand_empty_elements`] setting.
    ///
    /// If this method returns something, the read next event should return this
    /// event.
    pub fn pending_end(&mut self) -> Option<Event<'static>> {
        if self.pending {
            self.pending = false;
            let name = self
                .opened_buffer
                .split_off(self.opened_starts.pop().unwrap());
            return Some(Event::End(BytesEnd::wrap(name.into())));
        }
        None
    }
}

impl Default for ReaderState {
    fn default() -> Self {
        Self {
            parser: Parser::default(),
            offset: 0,
            last_error_offset: 0,
            config: Config::default(),
            can_trim_start: true,
            pending: false,
            opened_buffer: Vec::new(),
            opened_starts: Vec::new(),

            #[cfg(feature = "encoding")]
            encoding: EncodingRef::Implicit(UTF_8),
        }
    }
}

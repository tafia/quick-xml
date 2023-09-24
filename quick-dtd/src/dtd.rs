//! Contains the Document Type Definition pull-based parser.

use crate::{CommentParser, PiParser, QuotedParser};
use core::iter::Iterator;

/// An internal state of a parser. Used to preserve information about currently
/// parsed event between calls to [`DtdParser::feed()`].
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum State {
    /// Initial state used to begin parsing DTD events.
    Start,
    /// A `<` was seen, but nothing else.
    Markup,
    /// A `<!` was seen, but nothing else. It is unable to understand right now
    /// what data follow.
    MarkupBang,
    /// A `<?` was seen, but nothing else. We parsing a processing instruction.
    /// If parameter is `true`, then the `?` was the last symbol on the last
    /// consumed buffer.
    PI(PiParser),
    /// A `<!E` was seen, but nothing else. It is unable to understand right now
    /// is this an `<!ELEMENT`, or `<!ENTITY` or something else.
    MaybeElementOrEntity,

    /// A `<!EL` was seen, but nothing else. It is unable to understand right now
    /// is this an `<!ELEMENT` or something else.
    MaybeElement1,
    /// A `<!ELE` was seen, but nothing else. It is unable to understand right now
    /// is this an `<!ELEMENT` or something else.
    MaybeElement2,
    /// A `<!ELEM` was seen, but nothing else. It is unable to understand right now
    /// is this an `<!ELEMENT` or something else.
    MaybeElement3,
    /// A `<!ELEME` was seen, but nothing else. It is unable to understand right now
    /// is this an `<!ELEMENT` or something else.
    MaybeElement4,
    /// A `<!ELEMEN` was seen, but nothing else. It is unable to understand right now
    /// is this an `<!ELEMENT` or something else.
    MaybeElement5,
    /// A `<!ELEMENT` was seen, but nothing else. It is unable to understand right now
    /// is this an `<!ELEMENT` and space symbol or something else.
    MaybeElement6,

    /// A `<!EN` was seen, but nothing else. It is unable to understand right now
    /// is this an `<!ENTITY` or something else.
    MaybeEntity1,
    /// A `<!ENT` was seen, but nothing else. It is unable to understand right now
    /// is this an `<!ENTITY` or something else.
    MaybeEntity2,
    /// A `<!ENTI` was seen, but nothing else. It is unable to understand right now
    /// is this an `<!ENTITY` or something else.
    MaybeEntity3,
    /// A `<!ENTIT` was seen, but nothing else. It is unable to understand right now
    /// is this an `<!ENTITY` or something else.
    MaybeEntity4,
    /// A `<!ENTITY` was seen, but nothing else. It is unable to understand right now
    /// is this an `<!ENTITY` and space symbol or something else.
    MaybeEntity5,

    /// A `<!A` was seen, but nothing else. It is unable to understand right now
    /// is this an `<!ATTLIST` or something else.
    MaybeAttList1,
    /// A `<!AT` was seen, but nothing else. It is unable to understand right now
    /// is this an `<!ATTLIST` or something else.
    MaybeAttList2,
    /// A `<!ATT` was seen, but nothing else. It is unable to understand right now
    /// is this an `<!ATTLIST` or something else.
    MaybeAttList3,
    /// A `<!ATTL` was seen, but nothing else. It is unable to understand right now
    /// is this an `<!ATTLIST` or something else.
    MaybeAttList4,
    /// A `<!ATTLI` was seen, but nothing else. It is unable to understand right now
    /// is this an `<!ATTLIST` or something else.
    MaybeAttList5,
    /// A `<!ATTLIS` was seen, but nothing else. It is unable to understand right now
    /// is this an `<!ATTLIST` or something else.
    MaybeAttList6,
    /// A `<!ATTLIST` was seen, but nothing else. It is unable to understand right now
    /// is this an `<!ATTLIST` and space symbol or something else.
    MaybeAttList7,

    /// A `<!N` was seen, but nothing else. It is unable to understand right now
    /// is this an `<!NOTATION` or something else.
    MaybeNotation1,
    /// A `<!NO` was seen, but nothing else. It is unable to understand right now
    /// is this an `<!NOTATION` or something else.
    MaybeNotation2,
    /// A `<!NOT` was seen, but nothing else. It is unable to understand right now
    /// is this an `<!NOTATION` or something else.
    MaybeNotation3,
    /// A `<!NOTA` was seen, but nothing else. It is unable to understand right now
    /// is this an `<!NOTATION` or something else.
    MaybeNotation4,
    /// A `<!NOTAT` was seen, but nothing else. It is unable to understand right now
    /// is this an `<!NOTATION` or something else.
    MaybeNotation5,
    /// A `<!NOTATI` was seen, but nothing else. It is unable to understand right now
    /// is this an `<!NOTATION` or something else.
    MaybeNotation6,
    /// A `<!NOTATIO` was seen, but nothing else. It is unable to understand right now
    /// is this an `<!NOTATION` or something else.
    MaybeNotation7,
    /// A `<!NOTATION` was seen, but nothing else. It is unable to understand right now
    /// is this an `<!NOTATION` and space symbol or something else.
    MaybeNotation8,

    /// A `<!-` was seen, but nothing else. It is unable to understand right now
    /// is this an `<!-` or something else.
    MaybeComment,

    /// A `<!ELEMENT` was seen and we now inside an element definition.
    Element,
    /// A `<!ENTITY` was seen and we now inside an entity definition.
    Entity(QuotedParser),
    /// A `<!ATTLIST` was seen and we now inside an attribute list definition.
    AttList(QuotedParser),
    /// A `<!NOTATION` was seen and we now inside a notation definition.
    Notation(QuotedParser),
    /// A `<!--` was seen and we now inside a comment.
    Comment(CommentParser),
}

impl Default for State {
    fn default() -> Self {
        Self::Start
    }
}

/// A result of feeding data into [`DtdParser`].
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FeedResult {
    /// All fed bytes should be consumed, new portion should be feed.
    NeedData,
    /// The specified count of bytes should be consumed from the input.
    EmitElement(usize),
    /// The specified count of bytes should be consumed from the input.
    EmitAttList(usize),
    /// The specified count of bytes should be consumed from the input.
    EmitEntity(usize),
    /// The specified count of bytes should be consumed from the input.
    EmitNotation(usize),
    /// The specified count of bytes should be consumed from the input.
    EmitPI(usize),
    /// The specified count of bytes should be consumed from the input.
    EmitComment(usize),

    /// Unexpected byte (`u8`) at the specified offset (`usize`) from begin of
    /// chunk that was pushed to [`DtdParser::feed()`].
    ///
    /// After getting this error the parser returned to the initial state and
    /// you can start parsing another DTD event by feeding data. You should,
    /// however, skip all unparsed data until `<` byte which is indication of
    /// start of a new DTD event.
    Unexpected(usize, u8),
}

/// A parser of Document Type Definition (DTD) schemas. The parser operates on
/// user-provided buffers with content of DTD. The content can be in any ASCII-compatible
/// encoding.
///
/// # Example
///
/// ```
/// # use pretty_assertions::assert_eq;
/// use quick_dtd::{DtdParser, FeedResult};
///
/// let mut parser = DtdParser::default();
/// let mut result = Vec::new();
/// let mut buf = Vec::new();
/// // Suppose that you read `chunk` chunks from network, for example
/// 'outer: for chunk in &[
///     "<!ELE",
///     "MENT",
///     " empty ",
///     "EMPTY>garbage\n<!ENTITY gt '>'>",
/// ] {
///     let mut input = chunk.as_bytes();
///     loop {
///         let consumed = match parser.feed(input) {
///             // All data in `input` was read and parser state didn't changed
///             // You should provide another chunk of data. The `input` should
///             // considered as fully consumed
///             FeedResult::NeedData => {
///                 // Store all input to buffer for current event, request the
///                 // new data from reader
///                 buf.extend_from_slice(input);
///                 continue 'outer;
///             }
///             FeedResult::Unexpected(offset, byte) => {
///                 match input[offset..].iter().position(|b| *b == b'<') {
///                     // Skip all garbage until start of new event
///                     Some(end) => {
///                         assert_eq!(&input[offset..end], b"garbage\n");
///                         offset + end
///                     }
///                     None => input.len(),
///                 }
///             }
///
///             FeedResult::EmitElement(offset) |
///             FeedResult::EmitAttList(offset) |
///             FeedResult::EmitEntity(offset) |
///             FeedResult::EmitNotation(offset) |
///             FeedResult::EmitPI(offset) |
///             FeedResult::EmitComment(offset) => {
///                 // Store consumed input to buffer for current event
///                 buf.extend_from_slice(&input[..offset]);
///                 // ..process `buf` with data of events here
///                 result.push(String::from_utf8(buf).unwrap());
///                 // Prepare buffer for new data
///                 buf = Vec::new();
///                 offset
///             }
///         };
///         // Skip consumed input, feed the rest on next iteration
///         input = &input[consumed..];
///     }
/// }
///
/// assert_eq!(result, [
///     "<!ELEMENT empty EMPTY>",
///     "<!ENTITY gt '>'>",
/// ]);
/// ```
#[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
pub struct DtdParser(State);
impl DtdParser {
    /// Provides new portion of data to the parser to parse. When this method
    /// returns [`FeedResult::NeedData`], the whole buffer was analyzed and no
    pub fn feed(&mut self, bytes: &[u8]) -> FeedResult {
        for (offset, &byte) in bytes.iter().enumerate() {
            let start = offset + 1;
            let rest = &bytes[start..];
            self.0 = match self.0 {
                State::Start => match byte {
                    b'<' => State::Markup,
                    // Skip spaces defined by XML standard
                    b' ' | b'\t' | b'\r' | b'\n' => continue,
                    b => return FeedResult::Unexpected(offset, b),
                },
                State::Markup => match byte {
                    b'!' => State::MarkupBang,
                    b'?' => return self.parse_pi(rest, start, PiParser::default()),
                    b => return FeedResult::Unexpected(offset, b),
                },
                State::MarkupBang => match byte {
                    b'E' => State::MaybeElementOrEntity,
                    b'A' => State::MaybeAttList1,
                    b'N' => State::MaybeNotation1,
                    b'-' => State::MaybeComment,
                    b => return FeedResult::Unexpected(offset, b),
                },
                State::MaybeElementOrEntity => match byte {
                    b'L' => State::MaybeElement1,
                    b'N' => State::MaybeEntity1,
                    b => return FeedResult::Unexpected(offset, b),
                },

                //----------------------------------------------------------------------------------
                // <!-- comment -->
                //----------------------------------------------------------------------------------
                State::MaybeComment => match byte {
                    b'-' => return self.parse_comment(rest, start, CommentParser::default()),
                    b => return FeedResult::Unexpected(offset, b),
                },
                State::Comment(parser) => return self.parse_comment(bytes, offset, parser),
                State::PI(parser) => return self.parse_pi(bytes, offset, parser),

                //----------------------------------------------------------------------------------
                // <!ELEMENT>
                //----------------------------------------------------------------------------------
                State::MaybeElement1 => match byte {
                    b'E' => State::MaybeElement2,
                    b => return FeedResult::Unexpected(offset, b),
                },
                State::MaybeElement2 => match byte {
                    b'M' => State::MaybeElement3,
                    b => return FeedResult::Unexpected(offset, b),
                },
                State::MaybeElement3 => match byte {
                    b'E' => State::MaybeElement4,
                    b => return FeedResult::Unexpected(offset, b),
                },
                State::MaybeElement4 => match byte {
                    b'N' => State::MaybeElement5,
                    b => return FeedResult::Unexpected(offset, b),
                },
                State::MaybeElement5 => match byte {
                    b'T' => State::MaybeElement6,
                    b => return FeedResult::Unexpected(offset, b),
                },
                State::MaybeElement6 => match byte {
                    b' ' | b'\t' | b'\r' | b'\n' => return self.parse_element(rest, start),
                    b => return FeedResult::Unexpected(offset, b),
                },
                State::Element => return self.parse_element(bytes, offset),

                //----------------------------------------------------------------------------------
                // <!ENTITY>
                //----------------------------------------------------------------------------------
                State::MaybeEntity1 => match byte {
                    b'T' => State::MaybeEntity2,
                    b => return FeedResult::Unexpected(offset, b),
                },
                State::MaybeEntity2 => match byte {
                    b'I' => State::MaybeEntity3,
                    b => return FeedResult::Unexpected(offset, b),
                },
                State::MaybeEntity3 => match byte {
                    b'T' => State::MaybeEntity4,
                    b => return FeedResult::Unexpected(offset, b),
                },
                State::MaybeEntity4 => match byte {
                    b'Y' => State::MaybeEntity5,
                    b => return FeedResult::Unexpected(offset, b),
                },
                State::MaybeEntity5 => match byte {
                    b' ' | b'\t' | b'\r' | b'\n' => {
                        return self.parse_entity(rest, start, QuotedParser::Outside)
                    }
                    b => return FeedResult::Unexpected(offset, b),
                },
                State::Entity(parser) => return self.parse_entity(bytes, offset, parser),

                //----------------------------------------------------------------------------------
                // <!ATTLIST>
                //----------------------------------------------------------------------------------
                State::MaybeAttList1 => match byte {
                    b'T' => State::MaybeAttList2,
                    b => return FeedResult::Unexpected(offset, b),
                },
                State::MaybeAttList2 => match byte {
                    b'T' => State::MaybeAttList3,
                    b => return FeedResult::Unexpected(offset, b),
                },
                State::MaybeAttList3 => match byte {
                    b'L' => State::MaybeAttList4,
                    b => return FeedResult::Unexpected(offset, b),
                },
                State::MaybeAttList4 => match byte {
                    b'I' => State::MaybeAttList5,
                    b => return FeedResult::Unexpected(offset, b),
                },
                State::MaybeAttList5 => match byte {
                    b'S' => State::MaybeAttList6,
                    b => return FeedResult::Unexpected(offset, b),
                },
                State::MaybeAttList6 => match byte {
                    b'T' => State::MaybeAttList7,
                    b => return FeedResult::Unexpected(offset, b),
                },
                State::MaybeAttList7 => match byte {
                    b' ' | b'\t' | b'\r' | b'\n' => {
                        return self.parse_attlist(rest, start, QuotedParser::Outside)
                    }
                    b => return FeedResult::Unexpected(offset, b),
                },
                State::AttList(parser) => return self.parse_attlist(bytes, offset, parser),

                //----------------------------------------------------------------------------------
                // <!NOTATION>
                //----------------------------------------------------------------------------------
                State::MaybeNotation1 => match byte {
                    b'O' => State::MaybeNotation2,
                    b => return FeedResult::Unexpected(offset, b),
                },
                State::MaybeNotation2 => match byte {
                    b'T' => State::MaybeNotation3,
                    b => return FeedResult::Unexpected(offset, b),
                },
                State::MaybeNotation3 => match byte {
                    b'A' => State::MaybeNotation4,
                    b => return FeedResult::Unexpected(offset, b),
                },
                State::MaybeNotation4 => match byte {
                    b'T' => State::MaybeNotation5,
                    b => return FeedResult::Unexpected(offset, b),
                },
                State::MaybeNotation5 => match byte {
                    b'I' => State::MaybeNotation6,
                    b => return FeedResult::Unexpected(offset, b),
                },
                State::MaybeNotation6 => match byte {
                    b'O' => State::MaybeNotation7,
                    b => return FeedResult::Unexpected(offset, b),
                },
                State::MaybeNotation7 => match byte {
                    b'N' => State::MaybeNotation8,
                    b => return FeedResult::Unexpected(offset, b),
                },
                State::MaybeNotation8 => match byte {
                    b' ' | b'\t' | b'\r' | b'\n' => {
                        return self.parse_notation(rest, start, QuotedParser::Outside);
                    }
                    b => return FeedResult::Unexpected(offset, b),
                },
                State::Notation(parser) => return self.parse_notation(bytes, offset, parser),
            };
        }
        FeedResult::NeedData
    }

    /// `<!ELEMENT >` cannot contain `>` inside, so we emit it as soon as we found `>`
    fn parse_element(&mut self, bytes: &[u8], offset: usize) -> FeedResult {
        match bytes.iter().position(|&b| b == b'>') {
            Some(i) => {
                self.0 = State::Start;
                // +1 for `>` which should be included in event
                FeedResult::EmitElement(offset + i + 1)
            }
            None => {
                self.0 = State::Element;
                FeedResult::NeedData
            }
        }
    }

    /// `<!ENTITY >` can contain `>` inside, but all those symbols either in single or double quotes
    fn parse_entity(
        &mut self,
        bytes: &[u8],
        offset: usize,
        mut parser: QuotedParser,
    ) -> FeedResult {
        match parser.feed(bytes) {
            Some(i) => {
                self.0 = State::Start;
                // +1 for `>` which should be included in event
                FeedResult::EmitEntity(offset + i + 1)
            }
            None => {
                self.0 = State::Entity(parser);
                FeedResult::NeedData
            }
        }
    }

    /// `<!ATTLIST >` can contain `>` inside, but all those symbols either in single or double quotes
    fn parse_attlist(
        &mut self,
        bytes: &[u8],
        offset: usize,
        mut parser: QuotedParser,
    ) -> FeedResult {
        match parser.feed(bytes) {
            Some(i) => {
                self.0 = State::Start;
                // +1 for `>` which should be included in event
                FeedResult::EmitAttList(offset + i + 1)
            }
            None => {
                self.0 = State::AttList(parser);
                FeedResult::NeedData
            }
        }
    }

    /// `<!NOTATION >` can contain `>` inside, but all those symbols either in single or double quotes
    fn parse_notation(
        &mut self,
        bytes: &[u8],
        offset: usize,
        mut parser: QuotedParser,
    ) -> FeedResult {
        match parser.feed(bytes) {
            Some(i) => {
                self.0 = State::Start;
                // +1 for `>` which should be included in event
                FeedResult::EmitNotation(offset + i + 1)
            }
            None => {
                self.0 = State::Notation(parser);
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
    ///   That sub-slice begins on the byte that represents a PI target (at least, should)
    /// - `offset`: a position of `bytes` sub-slice in the one that was passed to `feed()`
    /// - `has_mark`: a flag that indicates was the previous fed data ended with `?`
    fn parse_pi(&mut self, bytes: &[u8], offset: usize, mut parser: PiParser) -> FeedResult {
        match parser.feed(bytes) {
            Some(i) => {
                self.0 = State::Start;
                FeedResult::EmitPI(offset + i)
            }
            None => {
                self.0 = State::PI(parser);
                FeedResult::NeedData
            }
        }
    }

    /// Determines the end position of a comment in the provided slice.
    /// Comment ends on the first occurrence of `-->` which cannot be escaped.
    ///
    /// # Parameters
    /// - `bytes`: sub-slice to the original slice that was passed to `feed()`.
    ///   That sub-slice begins on the byte that represents a comment content (at least, should)
    /// - `offset`: a position of `bytes` sub-slice in the one that was passed to `feed()`
    /// - `parser`: the state of comment parser saved after consuming the previous chunk of data
    fn parse_comment(
        &mut self,
        bytes: &[u8],
        offset: usize,
        mut parser: CommentParser,
    ) -> FeedResult {
        match parser.feed(bytes) {
            Some(i) => {
                self.0 = State::Start;
                FeedResult::EmitComment(offset + i)
            }
            None => {
                self.0 = State::Comment(parser);
                FeedResult::NeedData
            }
        }
    }

    /// Convert this parser to an iterator producing [`FeedResult`]s from specified
    /// bytes.
    pub fn into_iter<'a>(self, bytes: &'a [u8]) -> DtdIter<'a> {
        DtdIter {
            chunk: bytes,
            parser: self,
        }
    }
}

/// This struct is created by the [`into_iter`] method of [`DtdParser`].
///
/// [`into_iter`]: DtdParser::into_iter
pub struct DtdIter<'a> {
    chunk: &'a [u8],
    parser: DtdParser,
}
impl<'a> DtdIter<'a> {
    /// Replaces current chunk of the iterator with nee one. All not-consumed
    /// data would be loss, so call it only when you get `FeedResult::NeedData`
    /// from the iterator.
    pub fn feed(&mut self, chunk: &'a [u8]) {
        self.chunk = chunk;
    }
}
impl<'a> Iterator for DtdIter<'a> {
    type Item = FeedResult;

    fn next(&mut self) -> Option<Self::Item> {
        if self.chunk.is_empty() {
            return None;
        }
        let result = self.parser.feed(self.chunk);
        match result {
            FeedResult::NeedData => {
                // All data consumed, so replace it empty data
                self.chunk = b"";
                None
            }
            FeedResult::EmitPI(off)
            | FeedResult::EmitEntity(off)
            | FeedResult::EmitAttList(off)
            | FeedResult::EmitComment(off)
            | FeedResult::EmitElement(off)
            | FeedResult::EmitNotation(off)
            | FeedResult::Unexpected(off, _) => {
                self.chunk = &self.chunk[off..];
                Some(result)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FeedResult::*;
    use super::*;
    use pretty_assertions::assert_eq;

    fn check(chunk_size: usize, bytes: &[u8]) {
        let mut iter = DtdParser::default().into_iter(b"");
        for (i, chunk) in bytes.chunks(chunk_size).enumerate() {
            iter.feed(chunk);
            while let Some(event) = iter.next() {
                assert!(
                    !matches!(event, FeedResult::Unexpected(..)),
                    "#{}: {:?} => {:?}\n{:?}",
                    i * chunk_size,
                    iter.parser.0,
                    event,
                    core::str::from_utf8(chunk).unwrap(),
                );
            }
        }
    }

    mod by_chunks {
        use super::*;

        const BYTES: &[u8] = include_bytes!("../tests/example.dtd");

        #[test]
        fn _1() {
            check(1, BYTES);
        }

        #[test]
        fn _2() {
            check(2, BYTES);
        }

        #[test]
        fn _3() {
            check(3, BYTES);
        }

        #[test]
        fn _5() {
            check(5, BYTES);
        }

        #[test]
        fn _7() {
            check(7, BYTES);
        }

        #[test]
        fn _11() {
            check(11, BYTES);
        }

        #[test]
        fn _13() {
            check(13, BYTES);
        }

        #[test]
        fn _17() {
            check(17, BYTES);
        }

        #[test]
        fn _19() {
            check(19, BYTES);
        }

        #[test]
        fn _23() {
            check(23, BYTES);
        }

        #[test]
        fn _29() {
            check(29, BYTES);
        }

        #[test]
        fn _31() {
            check(31, BYTES);
        }

        #[test]
        fn _37() {
            check(37, BYTES);
        }

        #[test]
        fn _41() {
            check(41, BYTES);
        }

        #[test]
        fn _43() {
            check(43, BYTES);
        }

        #[test]
        fn _47() {
            check(47, BYTES);
        }
    }

    #[test]
    fn element() {
        let mut parser = DtdParser(State::Element);
        assert_eq!(parser.feed(b""), NeedData);
        assert_eq!(parser.0, State::Element);

        let mut parser = DtdParser(State::Element);
        assert_eq!(parser.feed(b"a"), NeedData);
        assert_eq!(parser.0, State::Element);

        let mut parser = DtdParser(State::Element);
        assert_eq!(parser.feed(b">"), EmitElement(1));
        assert_eq!(parser.0, State::Start);

        let mut parser = DtdParser(State::Element);
        assert_eq!(parser.feed(b">a"), EmitElement(1));
        assert_eq!(parser.0, State::Start);
    }

    #[test]
    fn attlist() {
        let mut parser = DtdParser(State::AttList(QuotedParser::Outside));
        assert_eq!(parser.feed(b""), NeedData);
        assert_eq!(parser.0, State::AttList(QuotedParser::Outside));

        let mut parser = DtdParser(State::AttList(QuotedParser::Outside));
        assert_eq!(parser.feed(b"a"), NeedData);
        assert_eq!(parser.0, State::AttList(QuotedParser::Outside));

        let mut parser = DtdParser(State::AttList(QuotedParser::Outside));
        assert_eq!(parser.feed(b"'"), NeedData);
        assert_eq!(parser.0, State::AttList(QuotedParser::SingleQ));
        assert_eq!(parser.feed(b">"), NeedData);
        assert_eq!(parser.0, State::AttList(QuotedParser::SingleQ));
        assert_eq!(parser.feed(b"\""), NeedData);
        assert_eq!(parser.0, State::AttList(QuotedParser::SingleQ));
        assert_eq!(parser.feed(b"'"), NeedData);
        assert_eq!(parser.0, State::AttList(QuotedParser::Outside));
        assert_eq!(parser.feed(b">"), EmitAttList(1));
        assert_eq!(parser.0, State::Start);

        let mut parser = DtdParser(State::AttList(QuotedParser::Outside));
        assert_eq!(parser.feed(b"\""), NeedData);
        assert_eq!(parser.0, State::AttList(QuotedParser::DoubleQ));
        assert_eq!(parser.feed(b">"), NeedData);
        assert_eq!(parser.0, State::AttList(QuotedParser::DoubleQ));
        assert_eq!(parser.feed(b"'"), NeedData);
        assert_eq!(parser.0, State::AttList(QuotedParser::DoubleQ));
        assert_eq!(parser.feed(b"\""), NeedData);
        assert_eq!(parser.0, State::AttList(QuotedParser::Outside));
        assert_eq!(parser.feed(b">"), EmitAttList(1));
        assert_eq!(parser.0, State::Start);

        let mut parser = DtdParser(State::AttList(QuotedParser::Outside));
        assert_eq!(parser.feed(b">"), EmitAttList(1));
        assert_eq!(parser.0, State::Start);

        let mut parser = DtdParser(State::AttList(QuotedParser::Outside));
        assert_eq!(parser.feed(b">a"), EmitAttList(1));
        assert_eq!(parser.0, State::Start);

        let mut parser = DtdParser(State::AttList(QuotedParser::Outside));
        assert_eq!(parser.feed(b"'>\"'>"), EmitAttList(5));
        assert_eq!(parser.0, State::Start);

        let mut parser = DtdParser(State::AttList(QuotedParser::Outside));
        assert_eq!(parser.feed(b"\"'>\">"), EmitAttList(5));
        assert_eq!(parser.0, State::Start);
    }

    #[test]
    fn entity() {
        let mut parser = DtdParser(State::Entity(QuotedParser::Outside));
        assert_eq!(parser.feed(b""), NeedData);
        assert_eq!(parser.0, State::Entity(QuotedParser::Outside));

        let mut parser = DtdParser(State::Entity(QuotedParser::Outside));
        assert_eq!(parser.feed(b"a"), NeedData);
        assert_eq!(parser.0, State::Entity(QuotedParser::Outside));

        let mut parser = DtdParser(State::Entity(QuotedParser::Outside));
        assert_eq!(parser.feed(b"'"), NeedData);
        assert_eq!(parser.0, State::Entity(QuotedParser::SingleQ));
        assert_eq!(parser.feed(b">"), NeedData);
        assert_eq!(parser.0, State::Entity(QuotedParser::SingleQ));
        assert_eq!(parser.feed(b"\""), NeedData);
        assert_eq!(parser.0, State::Entity(QuotedParser::SingleQ));
        assert_eq!(parser.feed(b"'"), NeedData);
        assert_eq!(parser.0, State::Entity(QuotedParser::Outside));
        assert_eq!(parser.feed(b">"), EmitEntity(1));
        assert_eq!(parser.0, State::Start);

        let mut parser = DtdParser(State::Entity(QuotedParser::Outside));
        assert_eq!(parser.feed(b"\""), NeedData);
        assert_eq!(parser.0, State::Entity(QuotedParser::DoubleQ));
        assert_eq!(parser.feed(b">"), NeedData);
        assert_eq!(parser.0, State::Entity(QuotedParser::DoubleQ));
        assert_eq!(parser.feed(b"'"), NeedData);
        assert_eq!(parser.0, State::Entity(QuotedParser::DoubleQ));
        assert_eq!(parser.feed(b"\""), NeedData);
        assert_eq!(parser.0, State::Entity(QuotedParser::Outside));
        assert_eq!(parser.feed(b">"), EmitEntity(1));
        assert_eq!(parser.0, State::Start);

        let mut parser = DtdParser(State::Entity(QuotedParser::Outside));
        assert_eq!(parser.feed(b">"), EmitEntity(1));
        assert_eq!(parser.0, State::Start);

        let mut parser = DtdParser(State::Entity(QuotedParser::Outside));
        assert_eq!(parser.feed(b">a"), EmitEntity(1));
        assert_eq!(parser.0, State::Start);

        let mut parser = DtdParser(State::Entity(QuotedParser::Outside));
        assert_eq!(parser.feed(b"'>\"'>"), EmitEntity(5));
        assert_eq!(parser.0, State::Start);

        let mut parser = DtdParser(State::Entity(QuotedParser::Outside));
        assert_eq!(parser.feed(b"\"'>\">"), EmitEntity(5));
        assert_eq!(parser.0, State::Start);
    }

    #[test]
    fn notation() {
        let mut parser = DtdParser(State::Notation(QuotedParser::Outside));
        assert_eq!(parser.feed(b""), NeedData);
        assert_eq!(parser.0, State::Notation(QuotedParser::Outside));

        let mut parser = DtdParser(State::Notation(QuotedParser::Outside));
        assert_eq!(parser.feed(b"a"), NeedData);
        assert_eq!(parser.0, State::Notation(QuotedParser::Outside));

        let mut parser = DtdParser(State::Notation(QuotedParser::Outside));
        assert_eq!(parser.feed(b"'"), NeedData);
        assert_eq!(parser.0, State::Notation(QuotedParser::SingleQ));
        assert_eq!(parser.feed(b">"), NeedData);
        assert_eq!(parser.0, State::Notation(QuotedParser::SingleQ));
        assert_eq!(parser.feed(b"\""), NeedData);
        assert_eq!(parser.0, State::Notation(QuotedParser::SingleQ));
        assert_eq!(parser.feed(b"'"), NeedData);
        assert_eq!(parser.0, State::Notation(QuotedParser::Outside));
        assert_eq!(parser.feed(b">"), EmitNotation(1));
        assert_eq!(parser.0, State::Start);

        let mut parser = DtdParser(State::Notation(QuotedParser::Outside));
        assert_eq!(parser.feed(b"\""), NeedData);
        assert_eq!(parser.0, State::Notation(QuotedParser::DoubleQ));
        assert_eq!(parser.feed(b">"), NeedData);
        assert_eq!(parser.0, State::Notation(QuotedParser::DoubleQ));
        assert_eq!(parser.feed(b"'"), NeedData);
        assert_eq!(parser.0, State::Notation(QuotedParser::DoubleQ));
        assert_eq!(parser.feed(b"\""), NeedData);
        assert_eq!(parser.0, State::Notation(QuotedParser::Outside));
        assert_eq!(parser.feed(b">"), EmitNotation(1));
        assert_eq!(parser.0, State::Start);

        let mut parser = DtdParser(State::Notation(QuotedParser::Outside));
        assert_eq!(parser.feed(b">"), EmitNotation(1));
        assert_eq!(parser.0, State::Start);

        let mut parser = DtdParser(State::Notation(QuotedParser::Outside));
        assert_eq!(parser.feed(b">a"), EmitNotation(1));
        assert_eq!(parser.0, State::Start);

        let mut parser = DtdParser(State::Notation(QuotedParser::Outside));
        assert_eq!(parser.feed(b"'>\"'>"), EmitNotation(5));
        assert_eq!(parser.0, State::Start);

        let mut parser = DtdParser(State::Notation(QuotedParser::Outside));
        assert_eq!(parser.feed(b"\"'>\">"), EmitNotation(5));
        assert_eq!(parser.0, State::Start);
    }

    /*#[test]
    fn pi() {
        let mut parser = DtdParser(State::PI(false));
        assert_eq!(parser.feed(b""), NeedData);
        assert_eq!(parser.0, State::PI(false));
        let mut parser = DtdParser(State::PI(true));
        assert_eq!(parser.feed(b""), NeedData);
        assert_eq!(parser.0, State::PI(true));

        let mut parser = DtdParser(State::PI(false));
        assert_eq!(parser.feed(b"a"), NeedData);
        assert_eq!(parser.0, State::PI(false));
        let mut parser = DtdParser(State::PI(true));
        assert_eq!(parser.feed(b"a"), NeedData);
        assert_eq!(parser.0, State::PI(false));

        let mut parser = DtdParser(State::PI(false));
        assert_eq!(parser.feed(b"aa"), NeedData);
        assert_eq!(parser.0, State::PI(false));
        let mut parser = DtdParser(State::PI(true));
        assert_eq!(parser.feed(b"aa"), NeedData);
        assert_eq!(parser.0, State::PI(false));

        //----------------------------------------------------------------------

        let mut parser = DtdParser(State::PI(false));
        assert_eq!(parser.feed(b"?"), NeedData);
        assert_eq!(parser.0, State::PI(true));
        let mut parser = DtdParser(State::PI(true));
        assert_eq!(parser.feed(b"?"), NeedData);
        assert_eq!(parser.0, State::PI(true));

        let mut parser = DtdParser(State::PI(false));
        assert_eq!(parser.feed(b"?a"), NeedData);
        assert_eq!(parser.0, State::PI(false));
        let mut parser = DtdParser(State::PI(true));
        assert_eq!(parser.feed(b"?a"), NeedData);
        assert_eq!(parser.0, State::PI(false));

        let mut parser = DtdParser(State::PI(false));
        assert_eq!(parser.feed(b"a?"), NeedData);
        assert_eq!(parser.0, State::PI(true));
        let mut parser = DtdParser(State::PI(true));
        assert_eq!(parser.feed(b"a?"), NeedData);
        assert_eq!(parser.0, State::PI(true));

        //----------------------------------------------------------------------

        let mut parser = DtdParser(State::PI(false));
        assert_eq!(parser.feed(b">"), NeedData);
        assert_eq!(parser.0, State::PI(false));
        let mut parser = DtdParser(State::PI(true));
        assert_eq!(parser.feed(b">"), EmitPI(1));
        assert_eq!(parser.0, State::Start);

        let mut parser = DtdParser(State::PI(false));
        assert_eq!(parser.feed(b">a"), NeedData);
        assert_eq!(parser.0, State::PI(false));
        let mut parser = DtdParser(State::PI(true));
        assert_eq!(parser.feed(b">a"), EmitPI(1));
        assert_eq!(parser.0, State::Start);

        let mut parser = DtdParser(State::PI(false));
        assert_eq!(parser.feed(b"a>"), NeedData);
        assert_eq!(parser.0, State::PI(false));
        let mut parser = DtdParser(State::PI(true));
        assert_eq!(parser.feed(b"a>"), NeedData);
        assert_eq!(parser.0, State::PI(false));

        //----------------------------------------------------------------------

        let mut parser = DtdParser(State::PI(false));
        assert_eq!(parser.feed(b"?>"), EmitPI(2));
        assert_eq!(parser.0, State::Start);
        let mut parser = DtdParser(State::PI(true));
        assert_eq!(parser.feed(b"?>"), EmitPI(2));
        assert_eq!(parser.0, State::Start);

        let mut parser = DtdParser(State::PI(false));
        assert_eq!(parser.feed(b"?>a"), EmitPI(2));
        assert_eq!(parser.0, State::Start);
        let mut parser = DtdParser(State::PI(true));
        assert_eq!(parser.feed(b"?>a"), EmitPI(2));
        assert_eq!(parser.0, State::Start);

        let mut parser = DtdParser(State::PI(false));
        assert_eq!(parser.feed(b"a?>"), EmitPI(3));
        assert_eq!(parser.0, State::Start);
        let mut parser = DtdParser(State::PI(true));
        assert_eq!(parser.feed(b"a?>"), EmitPI(3));
        assert_eq!(parser.0, State::Start);
    }*/
}

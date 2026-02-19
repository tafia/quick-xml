//! Contains a parser for an XML element.

use crate::errors::SyntaxError;

/// A parser that search a `>` symbol in the slice outside of quoted regions.
///
/// The parser considers two quoted regions: a double-quoted (`"..."`) and
/// a single-quoted (`'...'`) region. Matches found inside those regions are not
/// considered as results. Each region starts and ends by its quote symbol,
/// which cannot be escaped (but can be encoded as XML character entity or named
/// entity. Anyway, that encoding does not contain literal quotes).
///
/// To use a parser create an instance of parser and [`feed`] data into it.
/// After successful search the parser will return [`Some`] with the length
/// of the element name and the position of
/// found symbol. If search is unsuccessful, a [`None`] will be returned. You
/// typically would expect positive result of search, so that you should feed
/// new data until you get it.
///
/// NOTE: after successful match the parser does not returned to the initial
/// state and should not be used anymore. Create a new parser if you want to perform
/// new search.
///
/// # Example
///
/// ```
/// # use pretty_assertions::assert_eq;
/// use quick_xml::parser::{ElementParser, Parser};
///
/// let mut parser = ElementParser::default();
///
/// // Parse `<my-element  with = 'some > inside'>and the text follow...`
/// // splitted into three chunks
/// assert_eq!(parser.feed(b"<my-element"), None);
/// // ...get new chunk of data
/// assert_eq!(parser.feed(b" with = 'some >"), None);
/// // ...get another chunk of data
/// assert_eq!(parser.feed(b" inside'>and the text follow..."), Some(8));
/// //                       ^       ^
/// //                       0       8
/// ```
///
/// [`feed`]: Self::feed()
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartElementParser {
    /// The initial state, inside the Tag name.
    /// Contains the current length of the tag name.
    Tag(usize),
    /// The name fast completely parsed. Now look for the '>'.
    Attributes(usize, AttributeParser),
}

/// The internal state of the attribute parser.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttributeParser {
    /// The initial state, not within ' or ".
    Outside,
    /// Inside a single-quoted region (`'...'`).
    SingleQ,
    /// Inside a double-quoted region (`"..."`).
    DoubleQ,
}

impl StartElementParser {
    /// Returns the length of the name and the number of consumed bytes of the current call or `None` if `>` was not found in `bytes`.
    /// A return-value of None implies, that the full bytes array was consumed.
    /// Assumes, that the initial '<' is already consumed.
    #[inline]
    pub fn feed(&mut self, bytes: &[u8]) -> Option<(usize, usize)> {
        // The number of bytes consumed in the current feed iteration.
        let mut consumed: usize = 0;

        let (name_len, mut attr_parser) = 'name_len: {
            match *self {
                Self::Tag(name_len) => {
                    for i in 0..bytes.len() {
                        let byte = bytes[i];

                        if matches!(byte, b' ' | b'\r' | b'\n' | b'\t' | b'/') {
                            // TODO(flxbe): Somehow make sure, that the only expect a '>' after the '/'.
                            let name_len = name_len + i;
                            let attr_parser = AttributeParser::Outside;
                            *self = Self::Attributes(name_len, attr_parser);

                            consumed += i;
                            break 'name_len (name_len, attr_parser);
                        } else if byte == b'>' {
                            return Some((name_len + i, consumed + i));
                        }
                    }

                    *self = Self::Tag(name_len + bytes.len());
                    return None;
                }
                Self::Attributes(name_len, attr_parser) => (name_len, attr_parser),
            }
        };

        let new_data = &bytes[consumed..];
        for i in memchr::memchr3_iter(b'>', b'\'', b'"', new_data) {
            attr_parser = match (attr_parser, new_data[i]) {
                // only allowed to match `>` while we are in state `Outside`
                (AttributeParser::Outside, b'>') => return Some((name_len, consumed + i)),
                (AttributeParser::Outside, b'\'') => AttributeParser::SingleQ,
                (AttributeParser::Outside, b'"') => AttributeParser::DoubleQ,

                // the only end_byte that gets us out if the same character
                (AttributeParser::SingleQ, b'\'') | (AttributeParser::DoubleQ, b'"') => {
                    AttributeParser::Outside
                }

                // all other bytes: no state change
                _ => continue,
            };
        }

        *self = Self::Attributes(name_len, attr_parser);
        None
    }

    /// Return the correct EOF SyntaxError based on the current internal state.
    #[inline]
    pub fn eof_error(self, _content: &[u8]) -> SyntaxError {
        match self {
            Self::Tag(_) => SyntaxError::UnclosedTag,
            Self::Attributes(_, attr) => match attr {
                AttributeParser::Outside => SyntaxError::UnclosedTag,
                AttributeParser::SingleQ => SyntaxError::UnclosedSingleQuotedAttributeValue,
                AttributeParser::DoubleQ => SyntaxError::UnclosedDoubleQuotedAttributeValue,
            },
        }
    }
}

impl Default for StartElementParser {
    #[inline]
    fn default() -> Self {
        Self::Tag(0)
    }
}

#[test]
fn parse_all() {
    use pretty_assertions::assert_eq;

    fn parse_input(input: &[u8], name_len: usize) {
        let mut parser = StartElementParser::default();

        assert_eq!(parser.feed(input), Some((name_len, input.len() - 1)));
    }

    parse_input(b"tag key='value' key=\"value\">", 3);
    parse_input(b"tag>", 3);
    parse_input(b"tag />", 3);
    parse_input(b"tag/>", 3);
}

#[test]
fn parse_internal_state() {
    use pretty_assertions::assert_eq;

    let mut parser = StartElementParser::default();
    assert_eq!(parser.feed(b""), None);
    assert_eq!(parser, StartElementParser::Tag(0));

    // start feeding the tag
    assert_eq!(parser.feed(b"tag"), None);
    assert_eq!(parser, StartElementParser::Tag(3));

    // Finish the tag parsing after seeing some whitespace
    assert_eq!(parser.feed(b" "), None);
    assert_eq!(
        parser,
        StartElementParser::Attributes(3, AttributeParser::Outside)
    );

    // Remain in state when no progress is made
    assert_eq!(parser.feed(b""), None);
    assert_eq!(
        parser,
        StartElementParser::Attributes(3, AttributeParser::Outside)
    );
    assert_eq!(parser.feed(b"some random content"), None);
    assert_eq!(
        parser,
        StartElementParser::Attributes(3, AttributeParser::Outside)
    );

    // Handle single qoute
    assert_eq!(parser.feed(b"\'"), None);
    assert_eq!(
        parser,
        StartElementParser::Attributes(3, AttributeParser::SingleQ)
    );

    // Remain in state when no progress is made
    assert_eq!(parser.feed(b""), None);
    assert_eq!(
        parser,
        StartElementParser::Attributes(3, AttributeParser::SingleQ)
    );
    assert_eq!(parser.feed(b"some random content \">"), None);
    assert_eq!(
        parser,
        StartElementParser::Attributes(3, AttributeParser::SingleQ)
    );

    // Close single quote
    assert_eq!(parser.feed(b"'"), None);
    assert_eq!(
        parser,
        StartElementParser::Attributes(3, AttributeParser::Outside)
    );

    // Handle double qoute
    assert_eq!(parser.feed(b"\""), None);
    assert_eq!(
        parser,
        StartElementParser::Attributes(3, AttributeParser::DoubleQ)
    );

    // Remain in state when no progress is made
    assert_eq!(parser.feed(b""), None);
    assert_eq!(
        parser,
        StartElementParser::Attributes(3, AttributeParser::DoubleQ)
    );
    assert_eq!(parser.feed(b"some random content '>"), None);
    assert_eq!(
        parser,
        StartElementParser::Attributes(3, AttributeParser::DoubleQ)
    );

    // Close double quote
    assert_eq!(parser.feed(b"\""), None);
    assert_eq!(
        parser,
        StartElementParser::Attributes(3, AttributeParser::Outside)
    );

    assert_eq!(parser.feed(b">"), Some((3, 0)));
}

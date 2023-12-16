//! Contains a parser for an XML CDATA content.

/// A parser that search a `]]>` sequence in the slice.
///
/// To use a parser create an instance of parser and [`feed`] data into it.
/// After successful search the parser will return [`Some`] with position where
/// comment is ended (the position after `]]>`). If search was unsuccessful,
/// a [`None`] will be returned. You typically would expect positive result of
/// search, so that you should feed new data until yo'll get it.
///
/// NOTE: after successful match the parser does not returned to the initial
/// state and should not be used anymore. Create a new parser if you want to perform
/// new search.
///
/// [`feed`]: Self::feed()
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum CDataParser {
    /// The parser does not yet seen any braces at the end of previous slice.
    Seen0,
    /// The parser already seen one brace on the end of previous slice.
    Seen1,
    /// The parser already seen two braces on the end of previous slice.
    Seen2,
}

impl CDataParser {
    /// Determines the end position of an XML character data in the provided slice.
    /// Character data (CDATA) is a pieces of text enclosed in `<![CDATA[` and `]]>` braces.
    /// Character data ends on the first occurrence of `]]>` which cannot be escaped.
    ///
    /// # Parameters
    /// - `bytes`: a slice to search end of CDATA. Should contain text in
    ///   ASCII-compatible encoding
    pub fn feed(&mut self, bytes: &[u8]) -> Option<usize> {
        let mut it = bytes.iter().enumerate();
        while let Some((i, _)) = it.find(|(_, &b)| b == b'>') {
            // ]]|>
            if i == 0 && *self == Self::Seen2 {
                // +1 for `>` which should be included in event
                return Some(1);
            }
            // x]|]>
            // ]]|]>
            if i == 1 && bytes[0] == b']' && matches!(self, Self::Seen1 | Self::Seen2) {
                // +1 for `>` which should be included in event
                return Some(2);
            }
            if bytes[..i].ends_with(b"]]") {
                // +1 for `>` which should be included in event
                return Some(i + 1);
            }
        }
        if bytes.ends_with(b"]]") {
            *self = Self::Seen2;
        } else {
            *self = self.next_state(bytes.last().copied());
        }
        None
    }

    #[inline]
    fn next_state(self, last: Option<u8>) -> Self {
        match (self, last) {
            (Self::Seen0, Some(b']')) => Self::Seen1,

            (Self::Seen1, Some(b']')) => Self::Seen2,
            (Self::Seen1, Some(_)) => Self::Seen0,

            (Self::Seen2, Some(b']')) => self,
            (Self::Seen2, Some(_)) => Self::Seen0,

            _ => self,
        }
    }
}

impl Default for CDataParser {
    fn default() -> Self {
        Self::Seen0
    }
}

#[test]
fn test() {
    use pretty_assertions::assert_eq;
    use CDataParser::*;

    fn parse_cdata(bytes: &[u8], mut parser: CDataParser) -> Result<usize, CDataParser> {
        match parser.feed(bytes) {
            Some(i) => Ok(i),
            None => Err(parser),
        }
    }

    assert_eq!(parse_cdata(b"", Seen0), Err(Seen0)); // xx|
    assert_eq!(parse_cdata(b"", Seen1), Err(Seen1)); // x]|
    assert_eq!(parse_cdata(b"", Seen2), Err(Seen2)); // ]]|

    assert_eq!(parse_cdata(b"]", Seen0), Err(Seen1)); // xx|]
    assert_eq!(parse_cdata(b"]", Seen1), Err(Seen2)); // x]|]
    assert_eq!(parse_cdata(b"]", Seen2), Err(Seen2)); // ]]|]

    assert_eq!(parse_cdata(b">", Seen0), Err(Seen0)); // xx|>
    assert_eq!(parse_cdata(b">", Seen1), Err(Seen0)); // x]|>
    assert_eq!(parse_cdata(b">", Seen2), Ok(1)); // ]]|>

    assert_eq!(parse_cdata(b"]]", Seen0), Err(Seen2)); // xx|]]
    assert_eq!(parse_cdata(b"]]", Seen1), Err(Seen2)); // x]|]]
    assert_eq!(parse_cdata(b"]]", Seen2), Err(Seen2)); // ]]|]]

    assert_eq!(parse_cdata(b"]>", Seen0), Err(Seen0)); // xx|]>
    assert_eq!(parse_cdata(b"]>", Seen1), Ok(2)); // x]|]>
    assert_eq!(parse_cdata(b"]>", Seen2), Ok(2)); // ]]|]>

    assert_eq!(parse_cdata(b"]]>", Seen0), Ok(3)); // xx|]]>
    assert_eq!(parse_cdata(b"]]>", Seen1), Ok(3)); // x]|]]>
    assert_eq!(parse_cdata(b"]]>", Seen2), Ok(3)); // ]]|]]>

    assert_eq!(parse_cdata(b">]]>", Seen0), Ok(4)); // xx|>]]>
    assert_eq!(parse_cdata(b">]]>", Seen1), Ok(4)); // x]|>]]>
    assert_eq!(parse_cdata(b">]]>", Seen2), Ok(1)); // ]]|>]]>

    assert_eq!(parse_cdata(b"]>]]>", Seen0), Ok(5)); // xx|]>]]>
    assert_eq!(parse_cdata(b"]>]]>", Seen1), Ok(2)); // x]|]>]]>
    assert_eq!(parse_cdata(b"]>]]>", Seen2), Ok(2)); // ]]|]>]]>
}

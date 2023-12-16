//! Contains a parser for an XML processing instruction.

/// A parser that search a `?>` sequence in the slice.
///
/// To use a parser create an instance of parser and [`feed`] data into it.
/// After successful search the parser will return [`Some`] with position where
/// processing instruction is ended (the position after `?>`). If search was
/// unsuccessful, a [`None`] will be returned. You typically would expect positive
/// result of search, so that you should feed new data until yo'll get it.
///
/// NOTE: after successful match the parser does not returned to the initial
/// state and should not be used anymore. Create a new parser if you want to perform
/// new search.
///
/// # Example
///
/// ```
/// # use quick_dtd::PiParser;
/// # use pretty_assertions::assert_eq;
/// let mut parser = PiParser::default();
///
/// // Parse `<my-element  with = 'some > inside'>and the text follow...`
/// // splitted into three chunks
/// assert_eq!(parser.feed(b"<?instruction"), None);
/// // ...get new chunk of data
/// assert_eq!(parser.feed(b" with some > and ?"), None);
/// // ...get another chunk of data
/// assert_eq!(parser.feed(b"inside?>and the text follow..."), Some(8));
/// //                       ^      ^
/// //                       0      7
/// ```
///
/// [`feed`]: Self::feed()
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PiParser(
    /// A flag that indicates was the `bytes` in the previous attempt to find the
    /// end ended with `?`.
    bool,
);

impl PiParser {
    /// Determines the end position of a processing instruction in the provided slice.
    /// Processing instruction ends on the first occurrence of `?>` which cannot be
    /// escaped.
    ///
    /// # Parameters
    /// - `bytes`: a slice to find the end of a processing instruction.
    ///   Should contain text in ASCII-compatible encoding
    pub fn feed(&mut self, bytes: &[u8]) -> Option<usize> {
        let mut it = bytes.iter().enumerate();
        while let Some((i, _)) = it.find(|(_, &b)| b == b'>') {
            match i {
                // +1 for `>` which should be included in event
                0 if self.0 => return Some(1),
                // If the previous byte is `?`, then we found `?>`
                // +1 for `>` which should be included in event
                i if i > 0 && bytes[i - 1] == b'?' => return Some(i + 1),
                _ => {}
            }
        }
        self.0 = bytes.last().copied() == Some(b'?');
        None
    }
}

#[test]
fn pi() {
    use pretty_assertions::assert_eq;

    fn parse_pi(bytes: &[u8], had_question_mark: bool) -> Result<usize, bool> {
        let mut parser = PiParser(had_question_mark);
        match parser.feed(bytes) {
            Some(i) => Ok(i),
            None => Err(parser.0),
        }
    }

    assert_eq!(parse_pi(b"", false), Err(false)); // x|
    assert_eq!(parse_pi(b"", true), Err(false)); // ?|

    assert_eq!(parse_pi(b"?", false), Err(true)); // x|?
    assert_eq!(parse_pi(b"?", true), Err(true)); // ?|?

    assert_eq!(parse_pi(b">", false), Err(false)); // x|>
    assert_eq!(parse_pi(b">", true), Ok(1)); // ?|>

    assert_eq!(parse_pi(b"?>", false), Ok(2)); // x|?>
    assert_eq!(parse_pi(b"?>", true), Ok(2)); // ?|?>

    assert_eq!(parse_pi(b">?>", false), Ok(3)); // x|>?>
    assert_eq!(parse_pi(b">?>", true), Ok(1)); // ?|>?>
}

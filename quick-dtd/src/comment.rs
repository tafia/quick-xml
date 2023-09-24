//! Contains a parser for an XML comment.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum State {
    /// The parser does not yet seen any dashes at the end of previous slice.
    Seen0,
    /// The parser already seen one dash on the end of previous slice.
    Seen1,
    /// The parser already seen two dashes on the end of previous slice.
    Seen2,
}

impl Default for State {
    fn default() -> Self {
        Self::Seen0
    }
}

/// A parser that search a `-->` sequence in the slice.
///
/// To use a parser create an instance of parser and [`feed`] data into it.
/// After successful search the parser will return [`Some`] with position where
/// comment is ended (the position after `-->`). If search was unsuccessful,
/// a [`None`] will be returned. You typically would expect positive result of
/// search, so that you should feed new data until yo'll get it.
///
/// NOTE: after successful match the parser does not returned to the initial
/// state and should not be used anymore. Create a new parser if you want to perform
/// new search.
///
/// # Example
///
/// ```
/// # use quick_dtd::CommentParser;
/// # use pretty_assertions::assert_eq;
/// let mut parser = CommentParser::default();
///
/// // Parse `<my-element  with = 'some > inside'>and the text follow...`
/// // splitted into three chunks
/// assert_eq!(parser.feed(b"<!-- comment"), None);
/// // ...get new chunk of data
/// assert_eq!(parser.feed(b" with some -> and -"), None);
/// // ...get another chunk of data
/// assert_eq!(parser.feed(b"-- inside-->and the text follow..."), Some(12));
/// //                       ^          ^
/// //                       0          11
/// ```
///
/// [`feed`]: Self::feed()
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CommentParser(State);

impl CommentParser {
    /// Determines the end position of an XML comment in the provided slice.
    /// Comments is a pieces of text enclosed in `<!--` and `-->` braces.
    /// Comment ends on the first occurrence of `-->` which cannot be escaped.
    ///
    /// # Parameters
    /// - `bytes`: a slice to search end of comment. Should contain text in
    ///   ASCII-compatible encoding
    pub fn feed(&mut self, bytes: &[u8]) -> Option<usize> {
        let mut it = bytes.iter().enumerate();
        while let Some((i, _)) = it.find(|(_, &b)| b == b'>') {
            // --|>
            if i == 0 && self.0 == State::Seen2 {
                // +1 for `>` which should be included in event
                return Some(1);
            }
            // x-|->
            // --|->
            if i == 1 && bytes[0] == b'-' && matches!(self.0, State::Seen1 | State::Seen2) {
                // +1 for `>` which should be included in event
                return Some(2);
            }
            if bytes[..i].ends_with(b"--") {
                // +1 for `>` which should be included in event
                return Some(i + 1);
            }
        }
        if bytes.ends_with(b"--") {
            self.0 = State::Seen2;
        } else {
            self.next_state(bytes.last().copied());
        }
        None
    }

    #[inline]
    fn next_state(&mut self, last: Option<u8>) {
        match (self.0, last) {
            (State::Seen0, Some(b'-')) => self.0 = State::Seen1,

            (State::Seen1, Some(b'-')) => self.0 = State::Seen2,
            (State::Seen1, Some(_)) => self.0 = State::Seen0,

            (State::Seen2, Some(b'-')) => {}
            (State::Seen2, Some(_)) => self.0 = State::Seen0,

            _ => {}
        }
    }
}

#[test]
fn test() {
    use pretty_assertions::assert_eq;
    use State::*;

    fn parse_comment(bytes: &[u8], initial: State) -> Result<usize, State> {
        let mut parser = CommentParser(initial);
        match parser.feed(bytes) {
            Some(i) => Ok(i),
            None => Err(parser.0),
        }
    }

    assert_eq!(parse_comment(b"", Seen0), Err(Seen0)); // xx|
    assert_eq!(parse_comment(b"", Seen1), Err(Seen1)); // x-|
    assert_eq!(parse_comment(b"", Seen2), Err(Seen2)); // --|

    assert_eq!(parse_comment(b"-", Seen0), Err(Seen1)); // xx|-
    assert_eq!(parse_comment(b"-", Seen1), Err(Seen2)); // x-|-
    assert_eq!(parse_comment(b"-", Seen2), Err(Seen2)); // --|-

    assert_eq!(parse_comment(b">", Seen0), Err(Seen0)); // xx|>
    assert_eq!(parse_comment(b">", Seen1), Err(Seen0)); // x-|>
    assert_eq!(parse_comment(b">", Seen2), Ok(1)); // --|>

    assert_eq!(parse_comment(b"--", Seen0), Err(Seen2)); // xx|--
    assert_eq!(parse_comment(b"--", Seen1), Err(Seen2)); // x-|--
    assert_eq!(parse_comment(b"--", Seen2), Err(Seen2)); // --|--

    assert_eq!(parse_comment(b"->", Seen0), Err(Seen0)); // xx|->
    assert_eq!(parse_comment(b"->", Seen1), Ok(2)); // x-|->
    assert_eq!(parse_comment(b"->", Seen2), Ok(2)); // --|->

    assert_eq!(parse_comment(b"-->", Seen0), Ok(3)); // xx|-->
    assert_eq!(parse_comment(b"-->", Seen1), Ok(3)); // x-|-->
    assert_eq!(parse_comment(b"-->", Seen2), Ok(3)); // --|-->

    assert_eq!(parse_comment(b">-->", Seen0), Ok(4)); // xx|>-->
    assert_eq!(parse_comment(b">-->", Seen1), Ok(4)); // x-|>-->
    assert_eq!(parse_comment(b">-->", Seen2), Ok(1)); // --|>-->

    assert_eq!(parse_comment(b"->-->", Seen0), Ok(5)); // xx|->-->
    assert_eq!(parse_comment(b"->-->", Seen1), Ok(2)); // x-|->-->
    assert_eq!(parse_comment(b"->-->", Seen2), Ok(2)); // --|->-->
}

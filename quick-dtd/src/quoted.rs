/// Represents the result of [`QuotedParser::one_of`] operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OneOf {
    /// The open angle bracket (`<`) was found as specified position.
    ///
    /// The open angle bracket could only be part of a tag inside DTD
    /// if DTD is correctly formed.
    Open(usize),
    /// The close angle bracket (`>`) was found as specified position.
    Close(usize),
    /// Nothing was found in the provided slice.
    None,
}

/// A parser that search a `>` symbol in the slice outside of quoted regions.
///
/// The parser considers two quoted regions: a double-quoted (`"..."`) and
/// a single-quoted (`'...'`) region. Matches found inside those regions are not
/// considered, as results. Each region starts and ends by its quote symbol,
/// which cannot be escaped (but can be encoded as XML character entity or named
/// entity. Anyway, that encoding does not contain literal quotes).
///
/// To use a parser create an instance of parser and [`feed`] data into it.
/// After successful search the parser will return [`Some`] with position of
/// found symbol. If search is unsuccessful, a [`None`] will be returned. You
/// typically would expect positive result of search, so that you should feed
/// new data until yo'll get it.
///
/// # Example
///
/// ```
/// # use quick_dtd::QuotedParser;
/// # use pretty_assertions::assert_eq;
/// let mut parser = QuotedParser::default();
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
pub enum QuotedParser {
    /// The initial state (inside element, but outside of attribute value).
    Outside,
    /// Inside a single-quoted region.
    SingleQ,
    /// Inside a double-quoted region.
    DoubleQ,
}
impl QuotedParser {
    /// Returns number of consumed bytes or `None` if `>` was not found in `bytes`.
    pub fn feed(&mut self, bytes: &[u8]) -> Option<usize> {
        let mut it = bytes.iter().enumerate();
        while let Some((i, &byte)) = it.find(|(_, &b)| matches!(b, b'>' | b'\'' | b'"')) {
            match (*self, byte) {
                // only allowed to match `>` while we are in state `Outside`
                (Self::Outside, b'>') => return Some(i),
                (Self::Outside, b'\'') => *self = Self::SingleQ,
                (Self::Outside, b'\"') => *self = Self::DoubleQ,

                // the only end_byte that gets us out if the same character
                (Self::SingleQ, b'\'') | (Self::DoubleQ, b'"') => *self = Self::Outside,

                // all other bytes: no state change
                _ => {}
            }
        }
        None
    }

    /// Returns number of consumed bytes or `None` if `<` or `>` was not found in `bytes`.
    pub fn one_of(&mut self, bytes: &[u8]) -> OneOf {
        let mut it = bytes.iter().enumerate();
        while let Some((i, &byte)) = it.find(|(_, &b)| matches!(b, b'<' | b'>' | b'\'' | b'"')) {
            match (*self, byte) {
                // only allowed to match `>` while we are in state `Outside`
                (Self::Outside, b'<') => return OneOf::Open(i),
                (Self::Outside, b'>') => return OneOf::Close(i),
                (Self::Outside, b'\'') => *self = Self::SingleQ,
                (Self::Outside, b'\"') => *self = Self::DoubleQ,

                // the only end_byte that gets us out if the same character
                (Self::SingleQ, b'\'') | (Self::DoubleQ, b'"') => *self = Self::Outside,

                // all other bytes: no state change
                _ => {}
            }
        }
        OneOf::None
    }
}

impl Default for QuotedParser {
    fn default() -> Self {
        Self::Outside
    }
}

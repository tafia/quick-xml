//! Xml Attributes module
//!
//! Provides an iterator over attributes key/value pairs
use std::borrow::Cow;
use std::ops::Range;
use std::io::BufRead;
use errors::{Error, Result};
use escape::{escape, unescape};
use reader::{is_whitespace, Reader};

/// Iterator over attributes key/value pairs
#[derive(Clone)]
pub struct Attributes<'a> {
    /// slice of `Element` corresponding to attributes
    bytes: &'a [u8],
    /// current position of the iterator
    position: usize,
    /// if true, checks for duplicate names
    with_checks: bool,
    /// allows attribute without quote or `=`
    html: bool,
    /// if `with_checks`, contains the ranges corresponding to the
    /// attribute names already parsed in this `Element`
    consumed: Vec<Range<usize>>,
}

impl<'a> Attributes<'a> {
    /// creates a new attribute iterator from a buffer
    pub fn new(buf: &'a [u8], pos: usize) -> Attributes<'a> {
        Attributes {
            bytes: buf,
            position: pos,
            html: false,
            with_checks: true,
            consumed: Vec::new(),
        }
    }

    /// creates a new attribute iterator from a buffer, allowing html attribute syntax
    pub fn html(buf: &'a [u8], pos: usize) -> Attributes<'a> {
        Attributes {
            bytes: buf,
            position: pos,
            html: true,
            with_checks: true,
            consumed: Vec::new(),
        }
    }

    /// check if attributes are distincts
    pub fn with_checks(&mut self, val: bool) -> &mut Attributes<'a> {
        self.with_checks = val;
        self
    }
}

/// A struct representing a key/value for a xml attribute
///
/// Parses either `key="value"` or `key='value'`.
/// Field `value` stores raw bytes, possibly containing escape-sequences.
#[derive(Debug, Clone, PartialEq)]
pub struct Attribute<'a> {
    /// the key to uniquely define the attribute
    pub key: &'a [u8],
    /// the raw value of attribute
    pub value: Cow<'a, [u8]>,
}

impl<'a> Attribute<'a> {
    /// unescapes the value
    pub fn unescaped_value(&self) -> Result<Cow<[u8]>> {
        unescape(&*self.value).map_err(Error::EscapeError)
    }

    /// unescapes then decode the value
    ///
    /// for performance reasons (could avoid allocating a `String`),
    /// it might be wiser to manually use
    /// 1. Attributes::unescaped_value()
    /// 2. Reader::decode(...)
    pub fn unescape_and_decode_value<B: BufRead>(&self, reader: &Reader<B>) -> Result<String> {
        self.unescaped_value()
            .map(|e| reader.decode(&*e).into_owned())
    }
}

impl<'a> From<(&'a [u8], &'a [u8])> for Attribute<'a> {
    /// Creates new attribute from raw bytes.
    /// Does not apply any transformation to both key and value.
    ///
    /// # Example
    /// ```
    /// use quick_xml::events::attributes::Attribute;
    ///
    /// let features = Attribute::from(("features".as_bytes(), "Bells &amp; whistles".as_bytes()));
    /// assert_eq!(features.value, "Bells &amp; whistles".as_bytes());
    /// ```
    fn from(val: (&'a [u8], &'a [u8])) -> Attribute<'a> {
        Attribute {
            key: val.0,
            value: Cow::from(val.1),
        }
    }
}

impl<'a> From<(&'a str, &'a str)> for Attribute<'a> {
    /// Creates new attribute from text representation.
    /// Key is stored as-is, but the value will be escaped.
    ///
    /// # Example
    /// ```
    /// use quick_xml::events::attributes::Attribute;
    ///
    /// let features = Attribute::from(("features", "Bells & whistles"));
    /// assert_eq!(features.value, "Bells &amp; whistles".as_bytes());
    /// ```
    fn from(val: (&'a str, &'a str)) -> Attribute<'a> {
        Attribute {
            key: val.0.as_bytes(),
            value: escape(val.1.as_bytes()),
        }
    }
}

impl<'a> Iterator for Attributes<'a> {
    type Item = Result<Attribute<'a>>;
    fn next(&mut self) -> Option<Self::Item> {

        let len = self.bytes.len();

        macro_rules! err {
            ($err: expr) => {{
                self.position = len;
                return Some(Err($err.into()));
            }}
        }

        macro_rules! attr {
            ($key: expr) => {{
                self.position = len;
                if self.html {
                    attr!($key, 0..0)
                } else {
                    return None;
                };
            }};
            ($key:expr, $val: expr) => {
                return Some(Ok(Attribute {
                    key: &self.bytes[$key],
                    value: Cow::Borrowed(&self.bytes[$val]),
                }));
            };
        }

        if len <= self.position {
            return None;
        }

        let mut bytes = self.bytes.iter().enumerate().skip(self.position);

        // key starts after the whitespace
        let start_key = match bytes.by_ref()
            .skip_while(|&(_, &b)| !is_whitespace(b))
            .find(|&(_, &b)| !is_whitespace(b)) {
            Some((i, _)) => i,
            None => attr!(self.position..len),
        };

        // key ends with either whitespace or =
        let end_key = match bytes.by_ref().find(|&(_, &b)| b == b'=' || is_whitespace(b)) {
            Some((i, &b'=')) => i,
            Some((i, &b'\'')) | Some((i, &b'"')) if self.with_checks => {
                err!(Error::NameWithQuote(i));
            }
            Some((i, _)) => {
                // consume until `=` or return if html
                match bytes.by_ref().find(|&(_, &b)| !is_whitespace(b)) {
                    Some((_, &b'=')) => i,
                    Some((j, _)) if self.html => {
                        self.position = j - 1;
                        attr!(start_key..i, 0..0);
                    }
                    Some((j, _)) => err!(Error::NoEqAfterName(j)),
                    None if self.html => {
                        self.position = len;
                        attr!(start_key..len, 0..0);
                    }
                    None => err!(Error::NoEqAfterName(len)),
                }
            }
            None => attr!(start_key..len),
        };

        if self.with_checks {
            if let Some(start) = self.consumed
                .iter()
                .filter(|r| r.len() == end_key - start_key)
                .find(|r| &self.bytes[(*r).clone()] == &self.bytes[start_key..end_key])
                .map(|ref r| r.start)
            {
                err!(Error::DuplicatedAttribute(start_key, start));
            }
            self.consumed.push(start_key..end_key);
        }

        // value has quote if not html
        match bytes.by_ref().find(|&(_, &b)| !is_whitespace(b)) {
            Some((i, quote @ &b'\'')) | Some((i, quote @ &b'"')) => {
                match bytes.by_ref().find(|&(_, &b)| b == *quote) {
                    Some((j, _)) => {
                        self.position = j + 1;
                        attr!(start_key..end_key, i + 1..j)
                    }
                    None => err!(Error::UnquotedValue(i)),
                }
            },
            Some((i, _)) if self.html => {
                let j = bytes.by_ref().find(|&(_, &b)| is_whitespace(b)).map_or(len, |(j, _)| j);
                self.position = j;
                attr!(start_key..end_key, i..j)
            }
            Some((i, _)) => err!(Error::UnquotedValue(i)),
            None => attr!(start_key..end_key),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regular() {
        let event = b"name a='a' b = 'b'";
        let mut attributes = Attributes::new(event, 0);
        attributes.with_checks(true);
        let a = attributes.next().unwrap().unwrap();
        assert_eq!(a.key, b"a");
        assert_eq!(&*a.value, b"a");
        let a = attributes.next().unwrap().unwrap();
        assert_eq!(a.key, b"b");
        assert_eq!(&*a.value, b"b");
        assert!(attributes.next().is_none());
    }

    #[test]
    fn mixed_quote() {
        let event = b"name a='a' b = \"b\" c='cc\"cc'";
        let mut attributes = Attributes::new(event, 0);
        attributes.with_checks(true);
        let a = attributes.next().unwrap().unwrap();
        assert_eq!(a.key, b"a");
        assert_eq!(&*a.value, b"a");
        let a = attributes.next().unwrap().unwrap();
        assert_eq!(a.key, b"b");
        assert_eq!(&*a.value, b"b");
        let a = attributes.next().unwrap().unwrap();
        assert_eq!(a.key, b"c");
        assert_eq!(&*a.value, b"cc\"cc");
        assert!(attributes.next().is_none());
    }

    #[test]
    fn html_fail() {
        let event = b"name a='a' b=b c";
        let mut attributes = Attributes::new(event, 0);
        attributes.with_checks(true);
        let a = attributes.next().unwrap().unwrap();
        assert_eq!(a.key, b"a");
        assert_eq!(&*a.value, b"a");
        assert!(attributes.next().unwrap().is_err());
    }

    #[test]
    fn html_ok() {
        let event = b"name a='a' e b=b c d ee=ee";
        let mut attributes = Attributes::html(event, 0);
        attributes.with_checks(true);
        let a = attributes.next().unwrap().unwrap();
        assert_eq!(a.key, b"a");
        assert_eq!(&*a.value, b"a");
        let a = attributes.next().unwrap().unwrap();
        assert_eq!(a.key, b"e");
        assert_eq!(&*a.value, b"");
        let a = attributes.next().unwrap().unwrap();
        assert_eq!(a.key, b"b");
        assert_eq!(&*a.value, b"b");
        let a = attributes.next().unwrap().unwrap();
        assert_eq!(a.key, b"c");
        assert_eq!(&*a.value, b"");
        let a = attributes.next().unwrap().unwrap();
        assert_eq!(a.key, b"d");
        assert_eq!(&*a.value, b"");
        let a = attributes.next().unwrap().unwrap();
        assert_eq!(a.key, b"ee");
        assert_eq!(&*a.value, b"ee");
        assert!(attributes.next().is_none());
    }
}

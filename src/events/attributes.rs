//! Xml Attributes module
//!
//! Provides an iterator over attributes key/value pairs

use errors::{Error, Result};
use escape::{escape, unescape};
use reader::{is_whitespace, Reader};
use std::borrow::Cow;
use std::io::BufRead;
use std::ops::Range;

/// Iterator over XML attributes.
///
/// Yields `Result<Attribute>`. An `Err` will be yielded if an attribute is malformed or duplicated.
/// The duplicate check can be turned off by calling [`with_checks(false)`].
///
/// [`with_checks(false)`]: #method.with_checks
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
    /// Creates a new attribute iterator from a buffer.
    pub fn new(buf: &'a [u8], pos: usize) -> Attributes<'a> {
        Attributes {
            bytes: buf,
            position: pos,
            html: false,
            with_checks: true,
            consumed: Vec::new(),
        }
    }

    /// Creates a new attribute iterator from a buffer, allowing HTML attribute syntax.
    pub fn html(buf: &'a [u8], pos: usize) -> Attributes<'a> {
        Attributes {
            bytes: buf,
            position: pos,
            html: true,
            with_checks: true,
            consumed: Vec::new(),
        }
    }

    /// Changes whether attributes should be checked for uniqueness.
    ///
    /// The XML specification requires attribute keys in the same element to be unique. This check
    /// can be disabled to improve performance slightly.
    ///
    /// (`true` by default)
    pub fn with_checks(&mut self, val: bool) -> &mut Attributes<'a> {
        self.with_checks = val;
        self
    }
}

/// A struct representing a key/value XML attribute.
///
/// Field `value` stores raw bytes, possibly containing escape-sequences. Most users will likely
/// want to access the value using one of the [`unescaped_value`] and [`unescape_and_decode_value`]
/// functions.
///
/// [`unescaped_value`]: #method.unescaped_value
/// [`unescape_and_decode_value`]: #method.unescape_and_decode_value
#[derive(Debug, Clone, PartialEq)]
pub struct Attribute<'a> {
    /// The key to uniquely define the attribute.
    ///
    /// If [`Attributes::with_checks`] is turned off, the key might not be unique.
    ///
    /// [`Attributes::with_checks`]: struct.Attributes.html#method.with_checks
    pub key: &'a [u8],
    /// The raw value of the attribute.
    pub value: Cow<'a, [u8]>,
}

impl<'a> Attribute<'a> {
    /// Returns the unescaped value.
    ///
    /// This is normally the value you are interested in. Escape sequences such as `&gt;` are
    /// replaced with their unescaped equivalents such as `>`.
    ///
    /// This will allocate if the value contains any escape sequences.
    pub fn unescaped_value(&self) -> Result<Cow<[u8]>> {
        unescape(&*self.value).map_err(Error::EscapeError)
    }

    /// Returns the unescaped and decoded string value.
    ///
    /// This allocates a `String` in all cases. For performance reasons it might be a better idea to
    /// instead use one of:
    ///
    /// * [`unescaped_value()`], as it doesn't allocate when no escape sequences are used.
    /// * [`Reader::decode()`], as it only allocates when the decoding can't be performed otherwise.
    ///
    /// [`unescaped_value()`]: #method.unescaped_value
    /// [`Reader::decode()`]: ../../reader/struct.Reader.html#method.decode
    pub fn unescape_and_decode_value<B: BufRead>(&self, reader: &Reader<B>) -> Result<String> {
        self.unescaped_value()
            .map(|e| reader.decode(&*e).into_owned())
    }
}

impl<'a> From<(&'a [u8], &'a [u8])> for Attribute<'a> {
    /// Creates new attribute from raw bytes.
    /// Does not apply any transformation to both key and value.
    ///
    /// # Examples
    ///
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
    /// # Examples
    ///
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
            ($err:expr) => {{
                self.position = len;
                return Some(Err($err.into()));
            }};
        }

        macro_rules! attr {
            ($key:expr) => {{
                self.position = len;
                if self.html {
                    attr!($key, 0..0)
                } else {
                    return None;
                };
            }};
            ($key:expr, $val:expr) => {
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
        let start_key = match bytes
            .by_ref()
            .skip_while(|&(_, &b)| !is_whitespace(b))
            .find(|&(_, &b)| !is_whitespace(b))
        {
            Some((i, _)) => i,
            None => attr!(self.position..len),
        };

        // key ends with either whitespace or =
        let end_key = match bytes
            .by_ref()
            .find(|&(_, &b)| b == b'=' || is_whitespace(b))
        {
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
            if let Some(start) = self
                .consumed
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
            }
            Some((i, _)) if self.html => {
                let j = bytes
                    .by_ref()
                    .find(|&(_, &b)| is_whitespace(b))
                    .map_or(len, |(j, _)| j);
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

//! Xml Attributes module
//!
//! Provides an iterator over attributes key/value pairs

use errors::{Error, Result};
use escape::{do_unescape, escape};
use reader::{is_whitespace, Reader};
use std::borrow::Cow;
use std::collections::HashMap;
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
    pub(crate) position: usize,
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
#[derive(Clone, PartialEq)]
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
    ///
    /// See also [`unescaped_value_with_custom_entities()`](#method.unescaped_value_with_custom_entities)
    pub fn unescaped_value(&self) -> Result<Cow<[u8]>> {
        self.make_unescaped_value(None)
    }

    /// Returns the unescaped value, using custom entities.
    ///
    /// This is normally the value you are interested in. Escape sequences such as `&gt;` are
    /// replaced with their unescaped equivalents such as `>`.
    /// Additional entities can be provided in `custom_entities`.
    ///
    /// This will allocate if the value contains any escape sequences.
    ///
    /// See also [`unescaped_value()`](#method.unescaped_value)
    ///
    /// # Pre-condition
    ///
    /// The keys and values of `custom_entities`, if any, must be valid UTF-8.
    pub fn unescaped_value_with_custom_entities(
        &self,
        custom_entities: &HashMap<Vec<u8>, Vec<u8>>,
    ) -> Result<Cow<[u8]>> {
        self.make_unescaped_value(Some(custom_entities))
    }

    fn make_unescaped_value(
        &self,
        custom_entities: Option<&HashMap<Vec<u8>, Vec<u8>>>,
    ) -> Result<Cow<[u8]>> {
        do_unescape(&*self.value, custom_entities).map_err(Error::EscapeError)
    }

    /// Decode then unescapes the value
    ///
    /// This allocates a `String` in all cases. For performance reasons it might be a better idea to
    /// instead use one of:
    ///
    /// * [`Reader::decode()`], as it only allocates when the decoding can't be performed otherwise.
    /// * [`unescaped_value()`], as it doesn't allocate when no escape sequences are used.
    ///
    /// [`unescaped_value()`]: #method.unescaped_value
    /// [`Reader::decode()`]: ../../reader/struct.Reader.html#method.decode
    pub fn unescape_and_decode_value<B: BufRead>(&self, reader: &Reader<B>) -> Result<String> {
        self.do_unescape_and_decode_value(reader, None)
    }

    /// Decode then unescapes the value with custom entities
    ///
    /// This allocates a `String` in all cases. For performance reasons it might be a better idea to
    /// instead use one of:
    ///
    /// * [`Reader::decode()`], as it only allocates when the decoding can't be performed otherwise.
    /// * [`unescaped_value()`], as it doesn't allocate when no escape sequences are used.
    ///
    /// [`unescaped_value_with_custom_entities()`]: #method.unescaped_value_with_custom_entities
    /// [`Reader::decode()`]: ../../reader/struct.Reader.html#method.decode
    ///
    /// # Pre-condition
    ///
    /// The keys and values of `custom_entities`, if any, must be valid UTF-8.
    pub fn unescape_and_decode_value_with_custom_entities<B: BufRead>(
        &self,
        reader: &Reader<B>,
        custom_entities: &HashMap<Vec<u8>, Vec<u8>>,
    ) -> Result<String> {
        self.do_unescape_and_decode_value(reader, Some(custom_entities))
    }

    /// The keys and values of `custom_entities`, if any, must be valid UTF-8.
    #[cfg(feature = "encoding")]
    fn do_unescape_and_decode_value<B: BufRead>(
        &self,
        reader: &Reader<B>,
        custom_entities: Option<&HashMap<Vec<u8>, Vec<u8>>>,
    ) -> Result<String> {
        let decoded = reader.decode(&*self.value);
        let unescaped =
            do_unescape(decoded.as_bytes(), custom_entities).map_err(Error::EscapeError)?;
        String::from_utf8(unescaped.into_owned()).map_err(|e| Error::Utf8(e.utf8_error()))
    }

    #[cfg(not(feature = "encoding"))]
    fn do_unescape_and_decode_value<B: BufRead>(
        &self,
        reader: &Reader<B>,
        custom_entities: Option<&HashMap<Vec<u8>, Vec<u8>>>,
    ) -> Result<String> {
        let decoded = reader.decode(&*self.value)?;
        let unescaped =
            do_unescape(decoded.as_bytes(), custom_entities).map_err(Error::EscapeError)?;
        String::from_utf8(unescaped.into_owned()).map_err(|e| Error::Utf8(e.utf8_error()))
    }

    /// helper method to unescape then decode self using the reader encoding
    /// but without BOM (Byte order mark)
    ///
    /// for performance reasons (could avoid allocating a `String`),
    /// it might be wiser to manually use
    /// 1. BytesText::unescaped()
    /// 2. Reader::decode(...)
    #[cfg(feature = "encoding")]
    pub fn unescape_and_decode_without_bom<B: BufRead>(
        &self,
        reader: &mut Reader<B>,
    ) -> Result<String> {
        self.do_unescape_and_decode_without_bom(reader, None)
    }

    /// helper method to unescape then decode self using the reader encoding
    /// but without BOM (Byte order mark)
    ///
    /// for performance reasons (could avoid allocating a `String`),
    /// it might be wiser to manually use
    /// 1. BytesText::unescaped()
    /// 2. Reader::decode(...)
    #[cfg(not(feature = "encoding"))]
    pub fn unescape_and_decode_without_bom<B: BufRead>(
        &self,
        reader: &Reader<B>,
    ) -> Result<String> {
        self.do_unescape_and_decode_without_bom(reader, None)
    }

    /// helper method to unescape then decode self using the reader encoding with custom entities
    /// but without BOM (Byte order mark)
    ///
    /// for performance reasons (could avoid allocating a `String`),
    /// it might be wiser to manually use
    /// 1. BytesText::unescaped()
    /// 2. Reader::decode(...)
    ///
    /// # Pre-condition
    ///
    /// The keys and values of `custom_entities`, if any, must be valid UTF-8.
    #[cfg(feature = "encoding")]
    pub fn unescape_and_decode_without_bom_with_custom_entities<B: BufRead>(
        &self,
        reader: &mut Reader<B>,
        custom_entities: &HashMap<Vec<u8>, Vec<u8>>,
    ) -> Result<String> {
        self.do_unescape_and_decode_without_bom(reader, Some(custom_entities))
    }

    /// helper method to unescape then decode self using the reader encoding with custom entities
    /// but without BOM (Byte order mark)
    ///
    /// for performance reasons (could avoid allocating a `String`),
    /// it might be wiser to manually use
    /// 1. BytesText::unescaped()
    /// 2. Reader::decode(...)
    ///
    /// # Pre-condition
    ///
    /// The keys and values of `custom_entities`, if any, must be valid UTF-8.
    #[cfg(not(feature = "encoding"))]
    pub fn unescape_and_decode_without_bom_with_custom_entities<B: BufRead>(
        &self,
        reader: &Reader<B>,
        custom_entities: &HashMap<Vec<u8>, Vec<u8>>,
    ) -> Result<String> {
        self.do_unescape_and_decode_without_bom(reader, Some(custom_entities))
    }

    #[cfg(feature = "encoding")]
    fn do_unescape_and_decode_without_bom<B: BufRead>(
        &self,
        reader: &mut Reader<B>,
        custom_entities: Option<&HashMap<Vec<u8>, Vec<u8>>>,
    ) -> Result<String> {
        let decoded = reader.decode_without_bom(&*self.value);
        let unescaped =
            do_unescape(decoded.as_bytes(), custom_entities).map_err(Error::EscapeError)?;
        String::from_utf8(unescaped.into_owned()).map_err(|e| Error::Utf8(e.utf8_error()))
    }

    #[cfg(not(feature = "encoding"))]
    fn do_unescape_and_decode_without_bom<B: BufRead>(
        &self,
        reader: &Reader<B>,
        custom_entities: Option<&HashMap<Vec<u8>, Vec<u8>>>,
    ) -> Result<String> {
        let decoded = reader.decode_without_bom(&*self.value)?;
        let unescaped =
            do_unescape(decoded.as_bytes(), custom_entities).map_err(Error::EscapeError)?;
        String::from_utf8(unescaped.into_owned()).map_err(|e| Error::Utf8(e.utf8_error()))
    }
}

impl<'a> std::fmt::Debug for Attribute<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use crate::utils::write_byte_string;

        write!(f, "Attribute {{ key: ")?;
        write_byte_string(f, self.key)?;
        write!(f, ", value: ")?;
        write_byte_string(f, &self.value)?;
        write!(f, " }}")
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
                .find(|r| self.bytes[(*r).clone()] == self.bytes[start_key..end_key])
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

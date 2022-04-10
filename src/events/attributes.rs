//! Xml Attributes module
//!
//! Provides an iterator over attributes key/value pairs

use crate::errors::{Error, Result as XmlResult};
use crate::escape::{do_unescape, escape};
use crate::reader::{is_whitespace, Reader};
use std::fmt::{Debug, Display, Formatter};
use std::{borrow::Cow, collections::HashMap, io::BufRead, ops::Range};

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
    pub fn unescaped_value(&self) -> XmlResult<Cow<[u8]>> {
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
    ) -> XmlResult<Cow<[u8]>> {
        self.make_unescaped_value(Some(custom_entities))
    }

    fn make_unescaped_value(
        &self,
        custom_entities: Option<&HashMap<Vec<u8>, Vec<u8>>>,
    ) -> XmlResult<Cow<[u8]>> {
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
    pub fn unescape_and_decode_value<B: BufRead>(&self, reader: &Reader<B>) -> XmlResult<String> {
        self.do_unescape_and_decode_value(reader, None)
    }

    /// Decode then unescapes the value with custom entities
    ///
    /// This allocates a `String` in all cases. For performance reasons it might be a better idea to
    /// instead use one of:
    ///
    /// * [`Reader::decode()`], as it only allocates when the decoding can't be performed otherwise.
    /// * [`unescaped_value_with_custom_entities()`], as it doesn't allocate when no escape sequences are used.
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
    ) -> XmlResult<String> {
        self.do_unescape_and_decode_value(reader, Some(custom_entities))
    }

    /// The keys and values of `custom_entities`, if any, must be valid UTF-8.
    #[cfg(feature = "encoding")]
    fn do_unescape_and_decode_value<B: BufRead>(
        &self,
        reader: &Reader<B>,
        custom_entities: Option<&HashMap<Vec<u8>, Vec<u8>>>,
    ) -> XmlResult<String> {
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
    ) -> XmlResult<String> {
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
    ) -> XmlResult<String> {
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
    ) -> XmlResult<String> {
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
    ) -> XmlResult<String> {
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
    ) -> XmlResult<String> {
        self.do_unescape_and_decode_without_bom(reader, Some(custom_entities))
    }

    #[cfg(feature = "encoding")]
    fn do_unescape_and_decode_without_bom<B: BufRead>(
        &self,
        reader: &mut Reader<B>,
        custom_entities: Option<&HashMap<Vec<u8>, Vec<u8>>>,
    ) -> XmlResult<String> {
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
    ) -> XmlResult<String> {
        let decoded = reader.decode_without_bom(&*self.value)?;
        let unescaped =
            do_unescape(decoded.as_bytes(), custom_entities).map_err(Error::EscapeError)?;
        String::from_utf8(unescaped.into_owned()).map_err(|e| Error::Utf8(e.utf8_error()))
    }
}

impl<'a> Debug for Attribute<'a> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use crate::utils::{write_byte_string, write_cow_string};

        write!(f, "Attribute {{ key: ")?;
        write_byte_string(f, self.key)?;
        write!(f, ", value: ")?;
        write_cow_string(f, &self.value)?;
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
    /// # use pretty_assertions::assert_eq;
    /// use fast_xml::events::attributes::Attribute;
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
    /// # use pretty_assertions::assert_eq;
    /// use fast_xml::events::attributes::Attribute;
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

////////////////////////////////////////////////////////////////////////////////////////////////////

/// Iterator over XML attributes.
///
/// Yields `Result<Attribute>`. An `Err` will be yielded if an attribute is malformed or duplicated.
/// The duplicate check can be turned off by calling [`with_checks(false)`].
///
/// [`with_checks(false)`]: #method.with_checks
#[derive(Clone, Debug)]
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

impl<'a> Iterator for Attributes<'a> {
    type Item = Result<Attribute<'a>, AttrError>;

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
                    None
                }
            }};
            ($key:expr, $val:expr) => {
                Some(Ok(Attribute {
                    key: &self.bytes[$key],
                    value: Cow::Borrowed(&self.bytes[$val]),
                }))
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
            None => return attr!(self.position..len),
        };

        // key ends with either whitespace or =
        let end_key = match bytes
            .by_ref()
            .find(|&(_, &b)| b == b'=' || is_whitespace(b))
        {
            Some((i, &b'=')) => i,
            Some((i, _)) => {
                // consume until `=` or return if html
                match bytes.by_ref().find(|&(_, &b)| !is_whitespace(b)) {
                    Some((_, &b'=')) => i,
                    Some((j, _)) if self.html => {
                        self.position = j - 1;
                        return attr!(start_key..i, 0..0);
                    }
                    Some((j, _)) => err!(AttrError::ExpectedEq(j)),
                    None if self.html => {
                        self.position = len;
                        return attr!(start_key..len, 0..0);
                    }
                    None => err!(AttrError::ExpectedEq(len)),
                }
            }
            None => return attr!(start_key..len),
        };

        if self.with_checks {
            if let Some(start) = self
                .consumed
                .iter()
                .filter(|r| r.len() == end_key - start_key)
                .find(|r| self.bytes[(*r).clone()] == self.bytes[start_key..end_key])
                .map(|ref r| r.start)
            {
                err!(AttrError::Duplicated(start_key, start));
            }
            self.consumed.push(start_key..end_key);
        }

        // value has quote if not html
        match bytes.by_ref().find(|&(_, &b)| !is_whitespace(b)) {
            Some((i, quote @ &b'\'')) | Some((i, quote @ &b'"')) => {
                match bytes.by_ref().find(|&(_, &b)| b == *quote) {
                    Some((j, _)) => {
                        self.position = j + 1;
                        return attr!(start_key..end_key, i + 1..j);
                    }
                    None => err!(AttrError::UnquotedValue(i)),
                }
            }
            Some((i, _)) if self.html => {
                let j = bytes
                    .by_ref()
                    .find(|&(_, &b)| is_whitespace(b))
                    .map_or(len, |(j, _)| j);
                self.position = j;
                return attr!(start_key..end_key, i..j);
            }
            Some((i, _)) => err!(AttrError::UnquotedValue(i)),
            None => return attr!(start_key..end_key),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// Errors that can be raised during parsing attributes.
///
/// Recovery position in examples shows the position from which parsing of the
/// next attribute will be attempted.
#[derive(Debug, PartialEq)]
pub enum AttrError {
    /// Attribute key was not followed by `=`, position relative to the start of
    /// the owning tag is provided.
    ///
    /// Example of input that raises this error:
    ///
    /// ```xml
    /// <tag key another="attribute"/>
    /// <!--     ^~~ error position, recovery position (8) -->
    /// ```
    ///
    /// This error can be raised only when the iterator is in XML mode.
    ExpectedEq(usize),
    /// Attribute value was not found after `=`, position relative to the start
    /// of the owning tag is provided.
    ///
    /// Example of input that raises this error:
    ///
    /// ```xml
    /// <tag key = />
    /// <!--       ^~~ error position, recovery position (10) -->
    /// ```
    ///
    /// This error can be returned only for the last attribute in the list,
    /// because otherwise any content after `=` will be threated as a value.
    /// The XML
    ///
    /// ```xml
    /// <tag key = another-key = "value"/>
    /// <!--                   ^ ^- recovery position (24) -->
    /// <!--                   '~~ error position (22) -->
    /// ```
    ///
    /// will be treated as `Attribute { key = b"key", value = b"another-key" }`
    /// and or [`Attribute`] is returned, or [`AttrError::UnquotedValue`] is raised,
    /// depending on the parsing mode.
    ExpectedValue(usize),
    /// Attribute value is not quoted, position relative to the start of the
    /// owning tag is provided.
    ///
    /// Example of input that raises this error:
    ///
    /// ```xml
    /// <tag key = value />
    /// <!--       ^    ^~~ recovery position (15) -->
    /// <!--       '~~ error position (10) -->
    /// ```
    ///
    /// This error can be raised only when the iterator is in XML mode.
    UnquotedValue(usize),
    /// Attribute value was not finished with a matching quote, position relative
    /// to the start of owning tag and a quote is provided. That position is always
    /// a last character in the tag content.
    ///
    /// Example of input that raises this error:
    ///
    /// ```xml
    /// <tag key = "value  />
    /// <tag key = 'value  />
    /// <!--               ^~~ error position, recovery position (18) -->
    /// ```
    ///
    /// This error can be returned only for the last attribute in the list,
    /// because all input was consumed during scanning for a quote.
    ExpectedQuote(usize, u8),
    /// An attribute with the same name was already encountered. Two parameters
    /// define (1) the error position relative to the start of the owning tag
    /// for a new attribute and (2) the start position of a previously encountered
    /// attribute with the same name.
    ///
    /// Example of input that raises this error:
    ///
    /// ```xml
    /// <tag key = 'value'  key="value2" attr3='value3' />
    /// <!-- ^              ^            ^~~ recovery position (32) -->
    /// <!-- |              '~~ error position (19) -->
    /// <!-- '~~ previous position (4) -->
    /// ```
    ///
    /// This error is returned only when [`Attributes::with_checks()`] is set
    /// to `true` (that is default behavior).
    Duplicated(usize, usize),
}

impl Display for AttrError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::ExpectedEq(pos) => write!(
                f,
                r#"position {}: attribute key must be directly followed by `=` or space"#,
                pos
            ),
            Self::ExpectedValue(pos) => write!(
                f,
                r#"position {}: `=` must be followed by an attribute value"#,
                pos
            ),
            Self::UnquotedValue(pos) => write!(
                f,
                r#"position {}: attribute value must be enclosed in `"` or `'`"#,
                pos
            ),
            Self::ExpectedQuote(pos, quote) => write!(
                f,
                r#"position {}: missing closing quote `{}` in attribute value"#,
                pos, *quote as char
            ),
            Self::Duplicated(pos1, pos2) => write!(
                f,
                r#"position {}: duplicated attribute, previous declaration at position {}"#,
                pos1, pos2
            ),
        }
    }
}

impl std::error::Error for AttrError {}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// Checks, how parsing of XML-style attributes works. Each attribute should
/// have a value, enclosed in single or double quotes.
#[cfg(test)]
mod xml {
    use super::*;
    use pretty_assertions::assert_eq;

    /// Checked attribute is the single attribute
    mod single {
        use super::*;
        use pretty_assertions::assert_eq;

        /// Attribute have a value enclosed in single quotes
        #[test]
        fn single_quoted() {
            let mut iter = Attributes::new(br#"tag key='value'"#, 3);

            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"key",
                    value: Cow::Borrowed(b"value"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Attribute have a value enclosed in double quotes
        #[test]
        fn double_quoted() {
            let mut iter = Attributes::new(br#"tag key="value""#, 3);

            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"key",
                    value: Cow::Borrowed(b"value"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Attribute have a value, not enclosed in quotes
        #[test]
        fn unquoted() {
            let mut iter = Attributes::new(br#"tag key=value"#, 3);
            //                                 0       ^ = 8

            assert_eq!(iter.next(), Some(Err(AttrError::UnquotedValue(8))));
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Only attribute key is present
        #[test]
        fn key_only() {
            let mut iter = Attributes::new(br#"tag key"#, 3);
            //                                 0      ^ = 7

            assert_eq!(iter.next(), Some(Err(AttrError::ExpectedEq(7))));
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Key is started with an invalid symbol (a single quote in this test).
        /// Because we do not check validity of keys and values during parsing,
        /// that invalid attribute will be returned
        #[test]
        fn key_start_invalid() {
            let mut iter = Attributes::new(br#"tag 'key'='value'"#, 3);

            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"'key'",
                    value: Cow::Borrowed(b"value"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Key contains an invalid symbol (an ampersand in this test).
        /// Because we do not check validity of keys and values during parsing,
        /// that invalid attribute will be returned
        #[test]
        fn key_contains_invalid() {
            let mut iter = Attributes::new(br#"tag key&jey='value'"#, 3);

            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"key&jey",
                    value: Cow::Borrowed(b"value"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Attribute value is missing after `=`
        #[test]
        fn missed_value() {
            let mut iter = Attributes::new(br#"tag key="#, 3);
            //                                 0       ^ = 8

            assert_eq!(iter.next(), Some(Err(AttrError::ExpectedValue(8))));
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }
    }

    /// Checked attribute is the first attribute in the list of many attributes
    mod first {
        use super::*;
        use pretty_assertions::assert_eq;

        /// Attribute have a value enclosed in single quotes
        #[test]
        fn single_quoted() {
            let mut iter = Attributes::new(br#"tag key='value' regular='attribute'"#, 3);

            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"key",
                    value: Cow::Borrowed(b"value"),
                }))
            );
            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"regular",
                    value: Cow::Borrowed(b"attribute"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Attribute have a value enclosed in double quotes
        #[test]
        fn double_quoted() {
            let mut iter = Attributes::new(br#"tag key="value" regular='attribute'"#, 3);

            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"key",
                    value: Cow::Borrowed(b"value"),
                }))
            );
            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"regular",
                    value: Cow::Borrowed(b"attribute"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Attribute have a value, not enclosed in quotes
        #[test]
        fn unquoted() {
            let mut iter = Attributes::new(br#"tag key=value regular='attribute'"#, 3);
            //                                 0       ^ = 8

            assert_eq!(iter.next(), Some(Err(AttrError::UnquotedValue(8))));
            // check error recovery
            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"regular",
                    value: Cow::Borrowed(b"attribute"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Only attribute key is present
        #[test]
        fn key_only() {
            let mut iter = Attributes::new(br#"tag key regular='attribute'"#, 3);
            //                                 0       ^ = 8

            assert_eq!(iter.next(), Some(Err(AttrError::ExpectedEq(8))));
            // check error recovery
            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"regular",
                    value: Cow::Borrowed(b"attribute"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Key is started with an invalid symbol (a single quote in this test).
        /// Because we do not check validity of keys and values during parsing,
        /// that invalid attribute will be returned
        #[test]
        fn key_start_invalid() {
            let mut iter = Attributes::new(br#"tag 'key'='value' regular='attribute'"#, 3);

            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"'key'",
                    value: Cow::Borrowed(b"value"),
                }))
            );
            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"regular",
                    value: Cow::Borrowed(b"attribute"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Key contains an invalid symbol (an ampersand in this test).
        /// Because we do not check validity of keys and values during parsing,
        /// that invalid attribute will be returned
        #[test]
        fn key_contains_invalid() {
            let mut iter = Attributes::new(br#"tag key&jey='value' regular='attribute'"#, 3);

            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"key&jey",
                    value: Cow::Borrowed(b"value"),
                }))
            );
            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"regular",
                    value: Cow::Borrowed(b"attribute"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Attribute value is missing after `=`.
        #[test]
        fn missed_value() {
            let mut iter = Attributes::new(br#"tag key= regular='attribute'"#, 3);
            //                                 0        ^ = 9

            assert_eq!(iter.next(), Some(Err(AttrError::UnquotedValue(9))));
            // Because we do not check validity of keys and values during parsing,
            // "error='recovery'" is considered, as unquoted attribute value and
            // skipped during recovery and iteration finished
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);

            ////////////////////////////////////////////////////////////////////

            let mut iter = Attributes::new(br#"tag key= regular= 'attribute'"#, 3);
            //                                 0        ^ = 9               ^ = 29

            // In that case "regular=" considered as unquoted value
            assert_eq!(iter.next(), Some(Err(AttrError::UnquotedValue(9))));
            // In that case "'attribute'" considered as a key, because we do not check
            // validity of key names
            assert_eq!(iter.next(), Some(Err(AttrError::ExpectedEq(29))));
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);

            ////////////////////////////////////////////////////////////////////

            let mut iter = Attributes::new(br#"tag key= regular ='attribute'"#, 3);
            //                                 0        ^ = 9               ^ = 29

            // In that case "regular" considered as unquoted value
            assert_eq!(iter.next(), Some(Err(AttrError::UnquotedValue(9))));
            // In that case "='attribute'" considered as a key, because we do not check
            // validity of key names
            assert_eq!(iter.next(), Some(Err(AttrError::ExpectedEq(29))));
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);

            ////////////////////////////////////////////////////////////////////

            let mut iter = Attributes::new(br#"tag key= regular = 'attribute'"#, 3);
            //                                 0        ^ = 9     ^ = 19     ^ = 30

            assert_eq!(iter.next(), Some(Err(AttrError::UnquotedValue(9))));
            // In that case second "=" considered as a key, because we do not check
            // validity of key names
            assert_eq!(iter.next(), Some(Err(AttrError::ExpectedEq(19))));
            // In that case "'attribute'" considered as a key, because we do not check
            // validity of key names
            assert_eq!(iter.next(), Some(Err(AttrError::ExpectedEq(30))));
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }
    }

    /// Copy of single, but with additional spaces in markup
    mod sparsed {
        use super::*;
        use pretty_assertions::assert_eq;

        /// Attribute have a value enclosed in single quotes
        #[test]
        fn single_quoted() {
            let mut iter = Attributes::new(br#"tag key = 'value' "#, 3);

            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"key",
                    value: Cow::Borrowed(b"value"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Attribute have a value enclosed in double quotes
        #[test]
        fn double_quoted() {
            let mut iter = Attributes::new(br#"tag key = "value" "#, 3);

            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"key",
                    value: Cow::Borrowed(b"value"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Attribute have a value, not enclosed in quotes
        #[test]
        fn unquoted() {
            let mut iter = Attributes::new(br#"tag key = value "#, 3);
            //                                 0         ^ = 10

            assert_eq!(iter.next(), Some(Err(AttrError::UnquotedValue(10))));
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Only attribute key is present
        #[test]
        fn key_only() {
            let mut iter = Attributes::new(br#"tag key "#, 3);
            //                                 0       ^ = 8

            assert_eq!(iter.next(), Some(Err(AttrError::ExpectedEq(8))));
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Key is started with an invalid symbol (a single quote in this test).
        /// Because we do not check validity of keys and values during parsing,
        /// that invalid attribute will be returned
        #[test]
        fn key_start_invalid() {
            let mut iter = Attributes::new(br#"tag 'key' = 'value' "#, 3);

            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"'key'",
                    value: Cow::Borrowed(b"value"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Key contains an invalid symbol (an ampersand in this test).
        /// Because we do not check validity of keys and values during parsing,
        /// that invalid attribute will be returned
        #[test]
        fn key_contains_invalid() {
            let mut iter = Attributes::new(br#"tag key&jey = 'value' "#, 3);

            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"key&jey",
                    value: Cow::Borrowed(b"value"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Attribute value is missing after `=`
        #[test]
        fn missed_value() {
            let mut iter = Attributes::new(br#"tag key = "#, 3);
            //                                 0         ^ = 10

            assert_eq!(iter.next(), Some(Err(AttrError::ExpectedValue(10))));
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }
    }

    /// Checks that duplicated attributes correctly reported and recovering is
    /// possible after that
    mod duplicated {
        use super::*;

        mod with_check {
            use super::*;
            use pretty_assertions::assert_eq;

            /// Attribute have a value enclosed in single quotes
            #[test]
            fn single_quoted() {
                let mut iter = Attributes::new(br#"tag key='value' key='dup' another=''"#, 3);
                //                                 0   ^ = 4       ^ = 16

                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"key",
                        value: Cow::Borrowed(b"value"),
                    }))
                );
                assert_eq!(iter.next(), Some(Err(AttrError::Duplicated(16, 4))));
                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"another",
                        value: Cow::Borrowed(b""),
                    }))
                );
                assert_eq!(iter.next(), None);
                assert_eq!(iter.next(), None);
            }

            /// Attribute have a value enclosed in double quotes
            #[test]
            fn double_quoted() {
                let mut iter = Attributes::new(br#"tag key='value' key="dup" another=''"#, 3);
                //                                 0   ^ = 4       ^ = 16

                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"key",
                        value: Cow::Borrowed(b"value"),
                    }))
                );
                assert_eq!(iter.next(), Some(Err(AttrError::Duplicated(16, 4))));
                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"another",
                        value: Cow::Borrowed(b""),
                    }))
                );
                assert_eq!(iter.next(), None);
                assert_eq!(iter.next(), None);
            }

            /// Attribute have a value, not enclosed in quotes
            #[test]
            fn unquoted() {
                let mut iter = Attributes::new(br#"tag key='value' key=dup another=''"#, 3);
                //                                 0   ^ = 4       ^ = 16

                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"key",
                        value: Cow::Borrowed(b"value"),
                    }))
                );
                assert_eq!(iter.next(), Some(Err(AttrError::Duplicated(16, 4))));
                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"another",
                        value: Cow::Borrowed(b""),
                    }))
                );
                assert_eq!(iter.next(), None);
                assert_eq!(iter.next(), None);
            }

            /// Only attribute key is present
            #[test]
            fn key_only() {
                let mut iter = Attributes::new(br#"tag key='value' key another=''"#, 3);
                //                                 0                   ^ = 20

                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"key",
                        value: Cow::Borrowed(b"value"),
                    }))
                );
                assert_eq!(iter.next(), Some(Err(AttrError::ExpectedEq(20))));
                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"another",
                        value: Cow::Borrowed(b""),
                    }))
                );
                assert_eq!(iter.next(), None);
                assert_eq!(iter.next(), None);
            }
        }

        /// Check for duplicated names is disabled
        mod without_check {
            use super::*;
            use pretty_assertions::assert_eq;

            /// Attribute have a value enclosed in single quotes
            #[test]
            fn single_quoted() {
                let mut iter = Attributes::new(br#"tag key='value' key='dup' another=''"#, 3);
                iter.with_checks(false);

                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"key",
                        value: Cow::Borrowed(b"value"),
                    }))
                );
                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"key",
                        value: Cow::Borrowed(b"dup"),
                    }))
                );
                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"another",
                        value: Cow::Borrowed(b""),
                    }))
                );
                assert_eq!(iter.next(), None);
                assert_eq!(iter.next(), None);
            }

            /// Attribute have a value enclosed in double quotes
            #[test]
            fn double_quoted() {
                let mut iter = Attributes::new(br#"tag key='value' key="dup" another=''"#, 3);
                iter.with_checks(false);

                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"key",
                        value: Cow::Borrowed(b"value"),
                    }))
                );
                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"key",
                        value: Cow::Borrowed(b"dup"),
                    }))
                );
                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"another",
                        value: Cow::Borrowed(b""),
                    }))
                );
                assert_eq!(iter.next(), None);
                assert_eq!(iter.next(), None);
            }

            /// Attribute have a value, not enclosed in quotes
            #[test]
            fn unquoted() {
                let mut iter = Attributes::new(br#"tag key='value' key=dup another=''"#, 3);
                //                                 0                   ^ = 20
                iter.with_checks(false);

                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"key",
                        value: Cow::Borrowed(b"value"),
                    }))
                );
                assert_eq!(iter.next(), Some(Err(AttrError::UnquotedValue(20))));
                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"another",
                        value: Cow::Borrowed(b""),
                    }))
                );
                assert_eq!(iter.next(), None);
                assert_eq!(iter.next(), None);
            }

            /// Only attribute key is present
            #[test]
            fn key_only() {
                let mut iter = Attributes::new(br#"tag key='value' key another=''"#, 3);
                //                                 0                   ^ = 20
                iter.with_checks(false);

                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"key",
                        value: Cow::Borrowed(b"value"),
                    }))
                );
                assert_eq!(iter.next(), Some(Err(AttrError::ExpectedEq(20))));
                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"another",
                        value: Cow::Borrowed(b""),
                    }))
                );
                assert_eq!(iter.next(), None);
                assert_eq!(iter.next(), None);
            }
        }
    }

    #[test]
    fn mixed_quote() {
        let mut iter = Attributes::new(br#"tag a='a' b = "b" c='cc"cc' d="dd'dd""#, 3);

        assert_eq!(
            iter.next(),
            Some(Ok(Attribute {
                key: b"a",
                value: Cow::Borrowed(b"a"),
            }))
        );
        assert_eq!(
            iter.next(),
            Some(Ok(Attribute {
                key: b"b",
                value: Cow::Borrowed(b"b"),
            }))
        );
        assert_eq!(
            iter.next(),
            Some(Ok(Attribute {
                key: b"c",
                value: Cow::Borrowed(br#"cc"cc"#),
            }))
        );
        assert_eq!(
            iter.next(),
            Some(Ok(Attribute {
                key: b"d",
                value: Cow::Borrowed(b"dd'dd"),
            }))
        );
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None);
    }
}

/// Checks, how parsing of HTML-style attributes works. Each attribute can be
/// in three forms:
/// - XML-like: have a value, enclosed in single or double quotes
/// - have a value, do not enclosed in quotes
/// - without value, key only
#[cfg(test)]
mod html {
    use super::*;
    use pretty_assertions::assert_eq;

    /// Checked attribute is the single attribute
    mod single {
        use super::*;
        use pretty_assertions::assert_eq;

        /// Attribute have a value enclosed in single quotes
        #[test]
        fn single_quoted() {
            let mut iter = Attributes::html(br#"tag key='value'"#, 3);

            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"key",
                    value: Cow::Borrowed(b"value"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Attribute have a value enclosed in double quotes
        #[test]
        fn double_quoted() {
            let mut iter = Attributes::html(br#"tag key="value""#, 3);

            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"key",
                    value: Cow::Borrowed(b"value"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Attribute have a value, not enclosed in quotes
        #[test]
        fn unquoted() {
            let mut iter = Attributes::html(br#"tag key=value"#, 3);

            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"key",
                    value: Cow::Borrowed(b"value"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Only attribute key is present
        #[test]
        fn key_only() {
            let mut iter = Attributes::html(br#"tag key"#, 3);

            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"key",
                    value: Cow::Borrowed(&[]),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Key is started with an invalid symbol (a single quote in this test).
        /// Because we do not check validity of keys and values during parsing,
        /// that invalid attribute will be returned
        #[test]
        fn key_start_invalid() {
            let mut iter = Attributes::html(br#"tag 'key'='value'"#, 3);

            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"'key'",
                    value: Cow::Borrowed(b"value"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Key contains an invalid symbol (an ampersand in this test).
        /// Because we do not check validity of keys and values during parsing,
        /// that invalid attribute will be returned
        #[test]
        fn key_contains_invalid() {
            let mut iter = Attributes::html(br#"tag key&jey='value'"#, 3);

            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"key&jey",
                    value: Cow::Borrowed(b"value"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Attribute value is missing after `=`
        #[test]
        fn missed_value() {
            let mut iter = Attributes::html(br#"tag key="#, 3);
            //                                  0       ^ = 8

            assert_eq!(iter.next(), Some(Err(AttrError::ExpectedValue(8))));
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }
    }

    /// Checked attribute is the first attribute in the list of many attributes
    mod first {
        use super::*;
        use pretty_assertions::assert_eq;

        /// Attribute have a value enclosed in single quotes
        #[test]
        fn single_quoted() {
            let mut iter = Attributes::html(br#"tag key='value' regular='attribute'"#, 3);

            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"key",
                    value: Cow::Borrowed(b"value"),
                }))
            );
            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"regular",
                    value: Cow::Borrowed(b"attribute"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Attribute have a value enclosed in double quotes
        #[test]
        fn double_quoted() {
            let mut iter = Attributes::html(br#"tag key="value" regular='attribute'"#, 3);

            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"key",
                    value: Cow::Borrowed(b"value"),
                }))
            );
            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"regular",
                    value: Cow::Borrowed(b"attribute"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Attribute have a value, not enclosed in quotes
        #[test]
        fn unquoted() {
            let mut iter = Attributes::html(br#"tag key=value regular='attribute'"#, 3);

            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"key",
                    value: Cow::Borrowed(b"value"),
                }))
            );
            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"regular",
                    value: Cow::Borrowed(b"attribute"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Only attribute key is present
        #[test]
        fn key_only() {
            let mut iter = Attributes::html(br#"tag key regular='attribute'"#, 3);

            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"key",
                    value: Cow::Borrowed(&[]),
                }))
            );
            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"regular",
                    value: Cow::Borrowed(b"attribute"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Key is started with an invalid symbol (a single quote in this test).
        /// Because we do not check validity of keys and values during parsing,
        /// that invalid attribute will be returned
        #[test]
        fn key_start_invalid() {
            let mut iter = Attributes::html(br#"tag 'key'='value' regular='attribute'"#, 3);

            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"'key'",
                    value: Cow::Borrowed(b"value"),
                }))
            );
            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"regular",
                    value: Cow::Borrowed(b"attribute"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Key contains an invalid symbol (an ampersand in this test).
        /// Because we do not check validity of keys and values during parsing,
        /// that invalid attribute will be returned
        #[test]
        fn key_contains_invalid() {
            let mut iter = Attributes::html(br#"tag key&jey='value' regular='attribute'"#, 3);

            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"key&jey",
                    value: Cow::Borrowed(b"value"),
                }))
            );
            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"regular",
                    value: Cow::Borrowed(b"attribute"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Attribute value is missing after `=`
        #[test]
        fn missed_value() {
            let mut iter = Attributes::html(br#"tag key= regular='attribute'"#, 3);

            // Because we do not check validity of keys and values during parsing,
            // "regular='attribute'" is considered as unquoted attribute value
            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"key",
                    value: Cow::Borrowed(b"regular='attribute'"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);

            ////////////////////////////////////////////////////////////////////

            let mut iter = Attributes::html(br#"tag key= regular= 'attribute'"#, 3);

            // Because we do not check validity of keys and values during parsing,
            // "regular=" is considered as unquoted attribute value
            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"key",
                    value: Cow::Borrowed(b"regular="),
                }))
            );
            // Because we do not check validity of keys and values during parsing,
            // "'attribute'" is considered as key-only attribute
            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"'attribute'",
                    value: Cow::Borrowed(&[]),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);

            ////////////////////////////////////////////////////////////////////

            let mut iter = Attributes::html(br#"tag key= regular ='attribute'"#, 3);

            // Because we do not check validity of keys and values during parsing,
            // "regular" is considered as unquoted attribute value
            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"key",
                    value: Cow::Borrowed(b"regular"),
                }))
            );
            // Because we do not check validity of keys and values during parsing,
            // "='attribute'" is considered as key-only attribute
            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"='attribute'",
                    value: Cow::Borrowed(&[]),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);

            ////////////////////////////////////////////////////////////////////

            let mut iter = Attributes::html(br#"tag key= regular = 'attribute'"#, 3);
            //                                  0        ^ = 9     ^ = 19     ^ = 30

            // Because we do not check validity of keys and values during parsing,
            // "regular" is considered as unquoted attribute value
            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"key",
                    value: Cow::Borrowed(b"regular"),
                }))
            );
            // Because we do not check validity of keys and values during parsing,
            // "=" is considered as key-only attribute
            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"=",
                    value: Cow::Borrowed(&[]),
                }))
            );
            // Because we do not check validity of keys and values during parsing,
            // "'attribute'" is considered as key-only attribute
            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"'attribute'",
                    value: Cow::Borrowed(&[]),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }
    }

    /// Copy of single, but with additional spaces in markup
    mod sparsed {
        use super::*;
        use pretty_assertions::assert_eq;

        /// Attribute have a value enclosed in single quotes
        #[test]
        fn single_quoted() {
            let mut iter = Attributes::html(br#"tag key = 'value' "#, 3);

            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"key",
                    value: Cow::Borrowed(b"value"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Attribute have a value enclosed in double quotes
        #[test]
        fn double_quoted() {
            let mut iter = Attributes::html(br#"tag key = "value" "#, 3);

            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"key",
                    value: Cow::Borrowed(b"value"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Attribute have a value, not enclosed in quotes
        #[test]
        fn unquoted() {
            let mut iter = Attributes::html(br#"tag key = value "#, 3);

            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"key",
                    value: Cow::Borrowed(b"value"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Only attribute key is present
        #[test]
        fn key_only() {
            let mut iter = Attributes::html(br#"tag key "#, 3);

            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"key",
                    value: Cow::Borrowed(&[]),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Key is started with an invalid symbol (a single quote in this test).
        /// Because we do not check validity of keys and values during parsing,
        /// that invalid attribute will be returned
        #[test]
        fn key_start_invalid() {
            let mut iter = Attributes::html(br#"tag 'key' = 'value' "#, 3);

            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"'key'",
                    value: Cow::Borrowed(b"value"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Key contains an invalid symbol (an ampersand in this test).
        /// Because we do not check validity of keys and values during parsing,
        /// that invalid attribute will be returned
        #[test]
        fn key_contains_invalid() {
            let mut iter = Attributes::html(br#"tag key&jey = 'value' "#, 3);

            assert_eq!(
                iter.next(),
                Some(Ok(Attribute {
                    key: b"key&jey",
                    value: Cow::Borrowed(b"value"),
                }))
            );
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }

        /// Attribute value is missing after `=`
        #[test]
        fn missed_value() {
            let mut iter = Attributes::html(br#"tag key = "#, 3);
            //                                  0         ^ = 10

            assert_eq!(iter.next(), Some(Err(AttrError::ExpectedValue(10))));
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), None);
        }
    }

    /// Checks that duplicated attributes correctly reported and recovering is
    /// possible after that
    mod duplicated {
        use super::*;

        mod with_check {
            use super::*;
            use pretty_assertions::assert_eq;

            /// Attribute have a value enclosed in single quotes
            #[test]
            fn single_quoted() {
                let mut iter = Attributes::html(br#"tag key='value' key='dup' another=''"#, 3);
                //                                  0   ^ = 4       ^ = 16

                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"key",
                        value: Cow::Borrowed(b"value"),
                    }))
                );
                assert_eq!(iter.next(), Some(Err(AttrError::Duplicated(16, 4))));
                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"another",
                        value: Cow::Borrowed(b""),
                    }))
                );
                assert_eq!(iter.next(), None);
                assert_eq!(iter.next(), None);
            }

            /// Attribute have a value enclosed in double quotes
            #[test]
            fn double_quoted() {
                let mut iter = Attributes::html(br#"tag key='value' key="dup" another=''"#, 3);
                //                                  0   ^ = 4       ^ = 16

                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"key",
                        value: Cow::Borrowed(b"value"),
                    }))
                );
                assert_eq!(iter.next(), Some(Err(AttrError::Duplicated(16, 4))));
                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"another",
                        value: Cow::Borrowed(b""),
                    }))
                );
                assert_eq!(iter.next(), None);
                assert_eq!(iter.next(), None);
            }

            /// Attribute have a value, not enclosed in quotes
            #[test]
            fn unquoted() {
                let mut iter = Attributes::html(br#"tag key='value' key=dup another=''"#, 3);
                //                                  0   ^ = 4       ^ = 16

                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"key",
                        value: Cow::Borrowed(b"value"),
                    }))
                );
                assert_eq!(iter.next(), Some(Err(AttrError::Duplicated(16, 4))));
                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"another",
                        value: Cow::Borrowed(b""),
                    }))
                );
                assert_eq!(iter.next(), None);
                assert_eq!(iter.next(), None);
            }

            /// Only attribute key is present
            #[test]
            fn key_only() {
                let mut iter = Attributes::html(br#"tag key='value' key another=''"#, 3);
                //                                  0   ^ = 4       ^ = 16

                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"key",
                        value: Cow::Borrowed(b"value"),
                    }))
                );
                assert_eq!(iter.next(), Some(Err(AttrError::Duplicated(16, 4))));
                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"another",
                        value: Cow::Borrowed(b""),
                    }))
                );
                assert_eq!(iter.next(), None);
                assert_eq!(iter.next(), None);
            }
        }

        /// Check for duplicated names is disabled
        mod without_check {
            use super::*;
            use pretty_assertions::assert_eq;

            /// Attribute have a value enclosed in single quotes
            #[test]
            fn single_quoted() {
                let mut iter = Attributes::html(br#"tag key='value' key='dup' another=''"#, 3);
                iter.with_checks(false);

                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"key",
                        value: Cow::Borrowed(b"value"),
                    }))
                );
                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"key",
                        value: Cow::Borrowed(b"dup"),
                    }))
                );
                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"another",
                        value: Cow::Borrowed(b""),
                    }))
                );
                assert_eq!(iter.next(), None);
                assert_eq!(iter.next(), None);
            }

            /// Attribute have a value enclosed in double quotes
            #[test]
            fn double_quoted() {
                let mut iter = Attributes::html(br#"tag key='value' key="dup" another=''"#, 3);
                iter.with_checks(false);

                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"key",
                        value: Cow::Borrowed(b"value"),
                    }))
                );
                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"key",
                        value: Cow::Borrowed(b"dup"),
                    }))
                );
                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"another",
                        value: Cow::Borrowed(b""),
                    }))
                );
                assert_eq!(iter.next(), None);
                assert_eq!(iter.next(), None);
            }

            /// Attribute have a value, not enclosed in quotes
            #[test]
            fn unquoted() {
                let mut iter = Attributes::html(br#"tag key='value' key=dup another=''"#, 3);
                iter.with_checks(false);

                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"key",
                        value: Cow::Borrowed(b"value"),
                    }))
                );
                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"key",
                        value: Cow::Borrowed(b"dup"),
                    }))
                );
                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"another",
                        value: Cow::Borrowed(b""),
                    }))
                );
                assert_eq!(iter.next(), None);
                assert_eq!(iter.next(), None);
            }

            /// Only attribute key is present
            #[test]
            fn key_only() {
                let mut iter = Attributes::html(br#"tag key='value' key another=''"#, 3);
                iter.with_checks(false);

                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"key",
                        value: Cow::Borrowed(b"value"),
                    }))
                );
                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"key",
                        value: Cow::Borrowed(&[]),
                    }))
                );
                assert_eq!(
                    iter.next(),
                    Some(Ok(Attribute {
                        key: b"another",
                        value: Cow::Borrowed(b""),
                    }))
                );
                assert_eq!(iter.next(), None);
                assert_eq!(iter.next(), None);
            }
        }
    }

    #[test]
    fn mixed_quote() {
        let mut iter = Attributes::html(br#"tag a='a' b = "b" c='cc"cc' d="dd'dd""#, 3);

        assert_eq!(
            iter.next(),
            Some(Ok(Attribute {
                key: b"a",
                value: Cow::Borrowed(b"a"),
            }))
        );
        assert_eq!(
            iter.next(),
            Some(Ok(Attribute {
                key: b"b",
                value: Cow::Borrowed(b"b"),
            }))
        );
        assert_eq!(
            iter.next(),
            Some(Ok(Attribute {
                key: b"c",
                value: Cow::Borrowed(br#"cc"cc"#),
            }))
        );
        assert_eq!(
            iter.next(),
            Some(Ok(Attribute {
                key: b"d",
                value: Cow::Borrowed(b"dd'dd"),
            }))
        );
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None);
    }
}

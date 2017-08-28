//! Xml Attributes module
//!
//! Provides an iterator over attributes key/value pairs
use std::borrow::Cow;
use std::ops::Range;
use std::io::BufRead;
use errors::Result;
use escape::unescape;
use reader::{is_whitespace, Reader};

use memchr;

/// Iterator over attributes key/value pairs
#[derive(Clone)]
pub struct Attributes<'a> {
    /// slice of `Element` corresponding to attributes
    bytes: &'a [u8],
    /// current position of the iterator
    position: usize,
    /// shall the next iterator early exit because there were an error last time
    exit: bool,
    /// if true, checks for duplicate names
    with_checks: bool,
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
            exit: false,
            with_checks: true,
            consumed: Vec::new(),
        }
    }

    /// check if attributes are distincts
    pub fn with_checks(&mut self, val: bool) -> &mut Attributes<'a> {
        self.with_checks = val;
        self
    }

    /// sets `self.exit = true` to terminate the iterator
    fn error<S: Into<String>>(&mut self, msg: S, p: usize) -> Result<Attribute<'a>> {
        self.exit = true;
        Err(::errors::ErrorKind::Attribute(msg.into(), p).into())
    }
}

/// A struct representing a key/value for a xml attribute
///
/// Parses either `key="value"` or `key='value'`
#[derive(Debug, Clone, PartialEq)]
pub struct Attribute<'a> {
    /// the key to uniquely define the attribute
    pub key: &'a [u8],
    /// the value
    pub value: &'a [u8],
}

impl<'a> Attribute<'a> {
    /// unescapes the value
    pub fn unescaped_value(&self) -> Result<Cow<[u8]>> {
        unescape(self.value)
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
    fn from(val: (&'a [u8], &'a [u8])) -> Attribute<'a> {
        Attribute {
            key: val.0,
            value: val.1,
        }
    }
}

impl<'a> From<(&'a str, &'a str)> for Attribute<'a> {
    fn from(val: (&'a str, &'a str)) -> Attribute<'a> {
        Attribute {
            key: val.0.as_bytes(),
            value: val.1.as_bytes(),
        }
    }
}

impl<'a> Iterator for Attributes<'a> {
    type Item = Result<Attribute<'a>>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.exit {
            return None;
        }

        let len = self.bytes.len();
        let p = self.position;
        if len <= p {
            return None;
        }

        // search first space
        let mut start_key = match self.bytes[p..len - 1]
            .iter()
            .position(|&b| is_whitespace(b))
        {
            Some(i) => p + i + 1,
            None => {
                self.position = len;
                return None;
            }
        };

        // now search first non space
        start_key += match self.bytes[start_key..len - 1]
            .iter()
            .position(|&b| !is_whitespace(b))
        {
            Some(i) => i,
            None => {
                self.position = len;
                return None;
            }
        };

        // key end with either whitespace or =
        let end_key = match self.bytes[start_key + 1..len - 1]
            .iter()
            .position(|&b| b == b'=' || is_whitespace(b))
        {
            Some(i) => start_key + 1 + i,
            None => {
                self.position = len;
                return None;
            }
        };

        if self.with_checks {
            if let Some(i) = self.bytes[start_key..end_key]
                .iter()
                .position(|&b| b == b'\'' || b == b'"')
            {
                return Some(
                    self.error("Attribute key cannot contain quote", start_key + i),
                );
            }
            if let Some(r) = self.consumed.iter().cloned().find(|ref r| {
                &self.bytes[(**r).clone()] == &self.bytes[start_key..end_key]
            }) {
                return Some(self.error(
                    format!(
                        "Duplicate attribute at position {} and {}",
                        r.start,
                        start_key
                    ),
                    start_key,
                ));
            }
            self.consumed.push(start_key..end_key);
        }

        // values starts after =
        let start_val = match memchr::memchr(b'=', &self.bytes[end_key..len - 1]) {
            Some(i) => end_key + 1 + i,
            None => {
                self.position = len;
                return None;
            }
        };

        if self.with_checks {
            if let Some(i) = self.bytes[end_key..start_val - 1]
                .iter()
                .position(|&b| !is_whitespace(b))
            {
                return Some(self.error(
                    "Attribute key must be directly followed by = or space",
                    end_key + i,
                ));
            }
        }

        // value starts with a quote
        let (quote, start_val) = match self.bytes[start_val..len - 1]
            .iter()
            .enumerate()
            .filter(|&(_, &b)| !is_whitespace(b))
            .next()
        {
            Some((i, b @ &b'\'')) | Some((i, b @ &b'"')) => (*b, start_val + i + 1),
            Some((i, _)) => {
                return Some(
                    self.error("Attribute value must start with a quote", start_val + i),
                );
            }
            None => {
                self.position = len;
                return None;
            }
        };

        // value ends with the same quote
        let end_val = match memchr::memchr(quote, &self.bytes[start_val..]) {
            Some(i) => start_val + i,
            None => {
                self.position = len;
                return None;
            }
        };

        self.position = end_val + 1;

        Some(Ok(Attribute {
            key: &self.bytes[start_key..end_key],
            value: &self.bytes[start_val..end_val],
        }))
    }
}

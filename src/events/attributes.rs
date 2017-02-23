//! Xml Attributes module
//!
//! Provides an iterator over attributes key/value pairs
use std::borrow::Cow;
use std::ops::Range;
use std::io::BufRead;
use errors::Result;
use escape::unescape;
use reader::Reader;

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
    /// for performance reasons (could avoid allocating a `String`), it might be wiser to manually use
    /// 1. Attributes::unescaped_value()
    /// 2. Reader::decode(...)
    pub fn unescape_and_decode_value<B: BufRead>(&self, reader: &Reader<B>)
        -> Result<String> {
        self.unescaped_value().map(|e| reader.decode(&*e).into_owned())
    }
}

impl<'a> From<(&'a[u8], &'a[u8])> for Attribute<'a> {
    fn from(val:(&'a[u8], &'a[u8])) -> Attribute<'a> {
        Attribute { key: val.0, value: val.1 }
    }
}

impl<'a> From<(&'a str, &'a str)> for Attribute<'a> {
    fn from(val:(&'a str, &'a str)) -> Attribute<'a> {
        Attribute { key: val.0.as_bytes(), value: val.1.as_bytes() }
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

        let mut iter = self.bytes[p..].iter().cloned().enumerate();

        let start_key = {
            let mut found_space = false;
            let start: usize;
            loop {
                match iter.next() {
                    Some((_, b' ')) | Some((_, b'\r')) 
                        | Some((_, b'\n')) | Some((_, b'\t')) => {
                        if !found_space {
                            found_space = true;
                        }
                    }
                    Some((i, _)) => {
                        if found_space {
                            start = i;
                            break;
                        }
                    }
                    None => {
                        self.position = len;
                        return None;
                    }
                }
            }
            start
        };

        let mut has_equal = false;
        let mut end_key = None;
        let mut start_val = None;
        let mut end_val = None;
        let mut quote = 0;
        loop {
            match iter.next() {
                Some((i, b' ')) | Some((i, b'\r')) 
                    | Some((i, b'\n')) | Some((i, b'\t')) => {
                    if end_key.is_none() {
                        end_key = Some(i);
                    }
                }
                Some((i, b'=')) => {
                    if start_val.is_none() {
                        if has_equal {
                            return Some(self.error("Got 2 '=' tokens", p + i));
                        }
                        has_equal = true;
                        if end_key.is_none() {
                            end_key = Some(i);
                        }
                    }
                }
                Some((i, q @ b'"')) |
                Some((i, q @ b'\'')) => {
                    if !has_equal {
                        return Some(self.error("Unexpected quote before '='", p + i));
                    }
                    if start_val.is_none() {
                        start_val = Some(i + 1);
                        quote = q;
                    } else if quote == q && end_val.is_none() {
                        end_val = Some(i);
                        break;
                    }
                }
                None => {
                    self.position = len;
                    return None;
                }
                Some((_, _)) => (),
            }
        }
        self.position += end_val.unwrap() + 1;

        let r = (p + start_key)..(p + end_key.unwrap());
        if self.with_checks {
            let name = &self.bytes[r.clone()];
            if let Some(ref r2) = self.consumed
                .iter()
                .cloned()
                .find(|r2| &self.bytes[r2.clone()] == name) {
                    return Some(self.error(format!("Duplicate attribute at position {} and {}", 
                                    r2.start, r.start), r.start));
            }
            self.consumed.push(r.clone());
        }

        Some(Ok(Attribute {
            key: &self.bytes[r],
            value: &self.bytes[(p + start_val.unwrap())..(p + end_val.unwrap())],
        }))
    }
}

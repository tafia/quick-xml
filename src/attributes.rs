//! Xml Attributes module
//!
//! Provides an iterator over attributes key/value pairs
use std::borrow::Cow;
use std::ops::Range;
use error::{Error, ResultPos};
use escape::unescape;

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
    #[inline]
    pub fn new(buf: &'a [u8], pos: usize) -> Attributes<'a> {
        Attributes {
            bytes: buf,
            position: pos,
            exit: false,
            with_checks: true,
            consumed: Vec::new(),
        }
    }

    /// gets unescaped variant
    ///
    /// all escaped characters ('&...;') in attribute values are replaced
    /// with their corresponding character
    #[inline]
    pub fn unescaped(self) -> UnescapedAttributes<'a> {
        UnescapedAttributes { inner: self }
    }

    /// check if attributes are distincts
    #[inline]
    pub fn with_checks(mut self, val: bool) -> Attributes<'a> {
        self.with_checks = val;
        self
    }

    /// return Err((e, p))
    /// sets `self.exit = true` to terminate the iterator
    #[inline]
    fn error(&mut self, e: Error, p: usize) -> ResultPos<(&'a [u8], &'a [u8])> {
        self.exit = true;
        Err((e, p))
    }
}

impl<'a> Iterator for Attributes<'a> {
    type Item = ResultPos<(&'a [u8], &'a [u8])>;
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
                    if has_equal {
                        return Some(self.error(Error::Malformed(
                                    "Got 2 '=' tokens".to_string()), p + i));
                    }
                    has_equal = true;
                    if end_key.is_none() {
                        end_key = Some(i);
                    }
                }
                Some((i, q @ b'"')) |
                Some((i, q @ b'\'')) => {
                    if !has_equal {
                        return Some(self.error(Error::Malformed(
                                    "Unexpected quote before '='".to_string()), p + i));
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
                    return Some(self.error(Error::Malformed(
                            format!("Duplicate attribute at position {} and {}", 
                                    r2.start, r.start)), r.start));
            }
            self.consumed.push(r.clone());
        }

        Some(Ok((&self.bytes[r], 
                 &self.bytes[(p + start_val.unwrap())..(p + end_val.unwrap())])))
    }
}

/// Escaped attributes
///
/// Iterate over all attributes and unescapes attribute values
pub struct UnescapedAttributes<'a> {
    inner: Attributes<'a>,
}

impl<'a> Iterator for UnescapedAttributes<'a> {
    type Item = ResultPos<(&'a [u8], Cow<'a, [u8]>)>;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|a| a.and_then(|(k, v)| unescape(v).map(|v| (k, v))))
    }
}

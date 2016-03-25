//! Xml Attributes module
//!
//! Provides an iterator over attributes key/value pairs
use error::{Error, Result};

/// Iterator over attributes key/value pairs
pub struct Attributes<'a> {
    bytes: &'a [u8],
    position: usize,
    was_error: bool,
}

impl<'a> Attributes<'a> {
    /// creates a new attribute from a buffer
    /// 
    /// pos represents current position of the iterator 
    /// (starts after start element name)
    pub fn new(buf: &'a [u8], pos: usize) -> Attributes<'a> {
        Attributes {
            bytes: buf,
            position: pos,
            was_error: false,
        }
    }
}

impl<'a> Iterator for Attributes<'a> {
    type Item = Result<(&'a [u8], &'a [u8])>;
    fn next(&mut self) -> Option<Self::Item> {
        
        if self.was_error { return None; }

        let len = self.bytes.len();
        let p = self.position;
        if len <= p { return None; }

        let mut iter = self.bytes[p..].iter().cloned().enumerate();

        let start_key = {
            let mut found_space = false;
            let start: usize;
            loop {
                match iter.next() {
                    Some((_, b' '))
                        | Some((_, b'\r')) 
                        | Some((_, b'\n'))
                        | Some((_, b'\t')) => if !found_space { found_space = true; },
                    Some((i, _)) => if found_space { 
                        start = i;
                        break;
                    },
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
                Some((i, b' '))
                    | Some((i, b'\r')) 
                    | Some((i, b'\n'))
                    | Some((i, b'\t')) => {
                    if end_key.is_none() { end_key = Some(i); }
                },
                Some((i, b'=')) => {
                    if has_equal {
                        self.was_error = true;
                        return Some(Err(Error::Malformed("Got 2 '=' tokens".to_owned())));
                    }
                    has_equal = true;
                    if end_key.is_none() {
                        end_key = Some(i);
                    }
                },
                Some((i, q @ b'"')) | Some((i, q @ b'\'')) => {
                    if !has_equal {
                        self.was_error = true;
                        return Some(Err(Error::Malformed("Unexpected quote before '='".to_owned())));
                    }
                    if start_val.is_none() {
                        start_val = Some(i + 1);
                        quote = q;
                    } else if quote == q && end_val.is_none() {
                        end_val = Some(i);
                        break;
                    }
                },
                None => {
                    self.position = len;
                    return None;
                }
                Some((_, _)) => (),
            }
        }
        self.position += end_val.unwrap() + 1;

        Some(Ok((&self.bytes[(p + start_key)..(p + end_key.unwrap())],
           &self.bytes[(p + start_val.unwrap())..(p + end_val.unwrap())])))
    }
}

//! Quick XmlReader reader which performs **very** well.

#[macro_use]
extern crate log;

pub mod error;

#[cfg(test)]
mod test;

use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::iter::Iterator;
use std::path::Path;

use error::{Error, Result};

enum TagState {
    Opened,
    Closed,
}

pub struct XmlReader<B: BufRead> {
    /// reader
    reader: B,
    /// if was error, exit next
    exit: bool,
    next_close: bool,
    opened: Vec<Element>,
    tag_state: TagState,
    trim_text: bool,
}

impl<B: BufRead> XmlReader<B> {

    /// Creates a XmlReader from a generic BufReader
    pub fn from_reader(reader: B) -> XmlReader<B> {
        XmlReader {
            reader: reader,
            exit: false,
            next_close: false,
            opened: Vec::new(),
            tag_state: TagState::Closed,
            trim_text: false,
        }
    }

    pub fn trim_text(mut self, val: bool) -> XmlReader<B> {
        self.trim_text = val;
        self
    }
}

impl XmlReader<BufReader<File>> {
    /// Creates a csv from a file path
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<XmlReader<BufReader<File>>>
    {
        let reader = BufReader::new(try!(File::open(path)));
        Ok(XmlReader::from_reader(reader))
    }
}

impl<'a> XmlReader<&'a [u8]> {
    /// Creates a CSV reader for an in memory string buffer.
    pub fn from_str(s: &'a str) -> XmlReader<&'a [u8]> {
        XmlReader::from_reader(s.as_bytes())
    }
}

#[derive(Debug, Clone)]
pub struct Element {
    buf: Vec<u8>,
    start: usize,
    end: usize,
    name_end: usize,
}

impl Element {

    fn new(buf: Vec<u8>, start: usize, end: usize, name_end: usize) -> Element {
        Element {
            buf: buf,
            start: start,
            end: end,
            name_end: name_end,
        }
    }
    
    /// name part of element
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[self.start..self.name_end]
    }

    /// whole element seen as str, without parsing if there are blanks etc ...
    pub fn as_str(&self) -> Result<&str> {
        ::std::str::from_utf8(self.as_bytes()).map_err(|e| Error::from(e))
    }

    pub fn attributes<'a>(&'a self) -> Attributes<'a> {
        Attributes {
            bytes: &self.buf[self.start..self.end],
            position: self.name_end,
        }
    }
}

#[derive(Debug)]
pub enum Event {
    Start(Element),
    End(Element),
    Text(Element),
    Comment(Element),
    CData(Element),
    Header(Element),
}

impl Event {
    pub fn element(&self) -> &Element {
        match self {
            &Event::Start(ref e) |
            &Event::End(ref e) |
            &Event::Text(ref e) |
            &Event::Comment(ref e) |
            &Event::CData(ref e) |
            &Event::Header(ref e) => e,
        }
    }
}

/// Iterator on csv returning rows
impl<B: BufRead> Iterator for XmlReader<B> {

    type Item = Result<Event>;

    fn next(&mut self) -> Option<Result<Event>> {
        if self.exit { return None; }
        if self.next_close {
            self.next_close = false;
            return Some(Ok(Event::End(self.opened.pop().unwrap())));
        }
        let mut buf = Vec::new();
        match self.tag_state {
            TagState::Opened => {
                self.tag_state = TagState::Closed;
                match read_until(&mut self.reader, b'>', &mut buf) {
                    Ok(0) => None,
                    Ok(_n) => {
                        let len = buf.len();
                        if &buf[..1] == b"/" {
                            Some(Ok(Event::End(Element::new(buf, 1, len, len))))
                        } else if len >= 3 && &buf[..3] == b"!--" {
                            if len < 5 || &buf[(len - 2)..] != b"--" {
                                self.exit = true;
                                Some(Err(Error::Malformed("Expecting '--', found '>'")))
                            } else {
                                Some(Ok(Event::Comment(Element::new(buf, 3, len - 2, len - 2))))
                            }
                        } else if len >= 8 && &buf[..8] == b"![CDATA[" {
                            loop {
                                let len = buf.len();
                                if len >= 10 && &buf[(len - 2)..] == b"]]" {
                                    return Some(Ok(Event::CData(Element::new(buf, 8, len - 2, len - 2))))
                                }
                                buf.push(b'>');
                                match read_until(&mut self.reader, b'>', &mut buf) {
                                    Ok(0) => {
                                        self.exit = true;
                                        return Some(Err(Error::Malformed("Unescaped CDATA tag")));
                                    },
                                    Err(e) => {
                                        self.exit = true;
                                        return Some(Err(Error::from(e)));
                                    },
                                    _ => (),
                                }
                            }
                        } else if &buf[..1] == b"?" && &buf[(len - 2)..] == b"?" {
                            Some(Ok(Event::Header(Element::new(buf, 1, len - 1, len - 1))))
                        } else {
                            if &buf[(len - 1)..] == b"/" {
                                self.next_close = true;
                                let element = Element::new(buf, 0, len - 1, len - 1);
                                self.opened.push(element.clone());
                                Some(Ok(Event::Start(element)))
                            } else {
                                // TODO: do this directly when reading bufreader ...
                                let name_end = buf.iter().position(|&b| is_whitespace(b)).unwrap_or(len);
                                Some(Ok(Event::Start(Element::new(buf, 0, len, name_end))))
                            }
                        }
                    },
                    Err(e) => {
                        self.exit = true;
                        Some(Err(Error::from(e)))
                    },
                }
            },
            TagState::Closed => {
                self.tag_state = TagState::Opened;
                match read_until(&mut self.reader, b'<', &mut buf) {
                    Ok(0) => None,
                    Ok(_n) => {
                        let (start, len) = if self.trim_text {
                            // trim start
                            match buf.iter().position(|&b| !is_whitespace(b)) {
                                Some(start) => (start, buf.len() - buf.iter().rev()
                                                .position(|&b| !is_whitespace(b)).unwrap_or(0)),
                                None => return self.next()
                            }
                        } else {
                            (0, buf.len())
                        };
                        Some(Ok(Event::Text(Element::new(buf, start, len, len))))
                    },
                    Err(e) => {
                        self.exit = true;
                        Some(Err(Error::from(e)))
                    },
                }
            }
        }
    }
}

#[inline(always)]
fn is_whitespace(b: u8) -> bool {
    match b {
        b' ' | b'\r' | b'\n' | b'\t' => true,
        _ => false,
    }
}

pub struct Attributes<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Iterator for Attributes<'a> {
    type Item = Result<(&'a[u8], &'a str)>;
    fn next(&mut self) -> Option<Self::Item> {
        
        let len = self.bytes.len();
        let p = self.position;
        let mut iter = self.bytes[p..].iter().cloned().enumerate();

        let start_key = {
            let mut found_space = false;
            let p: usize;
            loop {
                match iter.next() {
                    Some((_, b' '))
                        | Some((_, b'\r')) 
                        | Some((_, b'\n'))
                        | Some((_, b'\t')) => if !found_space { found_space = true; },
                    Some((i, _)) => if found_space { 
                        p = i;
                        break;
                    },
                    None => {
                        self.position = len;
                        return None;
                    }
                }
            }
            p
        };

        let mut has_equal = false;
        let mut end_key = None;
        let mut start_val = None;
        let mut end_val = None;
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
                        debug!("has_equal x2 !");
                        return None; // TODO: return error instead
                    }
                    has_equal = true;
                    if end_key.is_none() {
                        end_key = Some(i);
                    }
                },
                Some((i, b'"')) => {
                    if !has_equal {
                        return Some(Err(Error::Malformed("Unexpected quote before '='")));
                    }
                    if start_val.is_none() {
                        start_val = Some(i + 1);
                    } else if end_val.is_none() {
                        end_val = Some(i);
                        break;
                    }
                },
                Some((_, _)) => (),
                None => {
                    self.position = len;
                    return None;
                }
            }
        }
        self.position = end_val.unwrap() + 1;

        match ::std::str::from_utf8(&self.bytes[(p + start_val.unwrap())..(p + end_val.unwrap())]) {
            Ok(s) => Some(Ok((&self.bytes[(p + start_key)..(p + end_key.unwrap())], s))),
            Err(e) => Some(Err(Error::from(e))),
        }
    }
}

#[inline(always)]
/// read_until slighly modified from rust std library
///
/// only change is that we do not write the matching character
fn read_until<R: BufRead>(r: &mut R, byte: u8, buf: &mut Vec<u8>) -> Result<usize> {
    let mut read = 0;
    let mut done = false;
    while !done {
        let used = {
            let available = match r.fill_buf() {
                Ok(n) if n.is_empty() => return Ok(read),
                Ok(n) => n,
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(Error::from(e)),
            };
            
            let mut bytes = available.iter().enumerate();

            let used: usize;
            loop {
                match bytes.next() {
                    Some((i, &b)) => {
                        if b == byte {
                            buf.extend_from_slice(&available[..i]);
                            done = true;
                            used = i + 1;
                            break;
                        }
                    },
                    None => {
                        buf.extend_from_slice(available);
                        used = available.len();
                        break;
                    },
                }
            }
            used
        };
        r.consume(used);
        read += used;
    }
    Ok(read)
}


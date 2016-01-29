//! Quick XmlReader reader which performs **very** well.

#[macro_use]
extern crate log;

pub mod error;
pub mod attributes;

#[cfg(test)]
mod test;

use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::iter::Iterator;
use std::path::Path;

use error::{Error, Result};
use attributes::Attributes;

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
    with_check: bool,
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
            with_check: true,
        }
    }

    pub fn trim_text(mut self, val: bool) -> XmlReader<B> {
        self.trim_text = val;
        self
    }

    pub fn with_check(mut self, val: bool) -> XmlReader<B> {
        self.with_check = val;
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
                        match buf[0] {
                            b'/' => {
                                if self.with_check && &buf[1..] != self.opened.pop().unwrap().as_bytes() {
                                    self.exit = true;
                                    return Some(Err(Error::Malformed(
                                            "End event doesn't match last opened element")));
                                }
                                return Some(Ok(Event::End(Element::new(buf, 1, len, len))))
                            },
                            b'?' => {
                                if len > 1 && buf[len - 1] == b'?' {
                                    return Some(Ok(Event::Header(Element::new(buf, 1, len - 1, len - 1))));
                                } else {
                                    self.exit = true;
                                    return Some(Err(Error::Malformed("Unescaped Header event")));
                                }
                            },
                            b'!' => {
                                if len >= 3 && &buf[1..3] == b"--" {
                                    if len < 5 || &buf[(len - 2)..] != b"--" {
                                        self.exit = true;
                                        return Some(Err(Error::Malformed("Unescaped Comment event")));
                                    } else {
                                        return Some(Ok(Event::Comment(Element::new(buf, 3, len - 2, len - 2))));
                                    }
                                } else if len >= 8 && &buf[1..8] == b"[CDATA[" {
                                    loop {
                                        let len = buf.len();
                                        if len >= 10 && &buf[(len - 2)..] == b"]]" {
                                            return Some(Ok(Event::CData(Element::new(buf, 8, len - 2, len - 2))));
                                        }
                                        buf.push(b'>');
                                        match read_until(&mut self.reader, b'>', &mut buf) {
                                            Ok(0) => {
                                                self.exit = true;
                                                return Some(Err(Error::Malformed("Unescaped CDATA event")));
                                            },
                                            Err(e) => {
                                                self.exit = true;
                                                return Some(Err(Error::from(e)));
                                            },
                                            _ => (),
                                        }
                                    }
                                }
                            },
                            _ => (),
                        }

                        // default case regular start or start/end
                        if buf[len - 1] == b'/' {
                            self.next_close = true;
                            let element = Element::new(buf, 0, len - 1, len - 1);
                            self.opened.push(element.clone());
                            Some(Ok(Event::Start(element)))
                        } else {
                            // TODO: do this directly when reading bufreader ...
                            let name_end = buf.iter().position(|&b| is_whitespace(b)).unwrap_or(len);
                            let element = Element::new(buf, 0, len, name_end);
                            if self.with_check {
                                self.opened.push(element.clone());
                            }
                            Some(Ok(Event::Start(element)))
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
    
    /// name part of element (without eventual attributes)
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[self.start..self.name_end]
    }

    /// whole element seen as str, without parsing if there are blanks etc ...
    pub fn as_str(&self) -> Result<&str> {
        ::std::str::from_utf8(self.as_bytes()).map_err(|e| Error::from(e))
    }

    /// get attributes iterator
    pub fn attributes<'a>(&'a self) -> Attributes<'a> {
        Attributes::new(&self.buf[self.start..self.end], self.name_end)
    }

    /// consumes entire self (including attributes) and returns string
    ///
    /// useful when we need to get Text event value (which don't have attributes)
    pub fn into_string(self) -> Result<String> {
        match ::std::string::String::from_utf8(self.buf) {
            Ok(s) => Ok(s),
            Err(e) => Err(Error::Utf8(e.utf8_error())),
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

#[inline(always)]
fn is_whitespace(b: u8) -> bool {
    match b {
        b' ' | b'\r' | b'\n' | b'\t' => true,
        _ => false,
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


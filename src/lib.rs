//! Quick XmlReader reader which performs **very** well.
//!
//! # Example
//!
//! ```
//! use quick_xml::{XmlReader, Event};
//! 
//! let xml = r#"<tag1 att1 = \"test\">
//!                 <tag2><!--Test comment-->Test</b>
//!                 <tag2>
//!                     Test 2
//!                 </tag2>
//!             </tag1>"#;
//! let reader = XmlReader::from_str(xml).trim_text(true);
//! let mut count = 0;
//! let mut txt = Vec::new();
//! for r in reader {
//!     match r {
//!         Ok(Event::Start(ref e)) => {
//!             match e.as_bytes() {
//!                 b"a" => println!("attributes values: {:?}", 
//!                                  e.attributes().map(|a| a.unwrap().1).collect::<Vec<_>>()),
//!                 b"b" => count += 1,
//!                 _ => (),
//!             }
//!         },
//!         Ok(Event::Text(e)) => txt.push(e.into_string()),
//!         Err(e) => println!("{:?}", e),
//!         _ => (),
//!     }
//! }
//! ```

#[macro_use]
extern crate log;

pub mod error;
pub mod attributes;

#[cfg(test)]
mod test;

use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::iter::Iterator;
use std::path::Path;
use std::fmt;

use error::{Error, Result};
use attributes::Attributes;

enum TagState {
    Opened,
    Closed,
}

/// Xml reader
///
/// Consumes a `BufRead` and streams xml Event
pub struct XmlReader<B: BufRead> {
    /// reader
    reader: B,
    /// if was error, exit next
    exit: bool,
    /// true when last Start element was a <.. />
    next_close: bool,
    /// all currently Started elements which didn't have a matching End element yet
    opened: Vec<Element>,
    /// current state Open/Close
    tag_state: TagState,
    /// trims Text events, skip the element if text is empty
    trim_text: bool,
    /// check if End nodes match last Start node
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

    /// Change trim_text default behaviour (false per default)
    ///
    /// When set to true, all Text events are trimed. If they are empty, no event if pushed
    pub fn trim_text(mut self, val: bool) -> XmlReader<B> {
        self.trim_text = val;
        self
    }

    /// Change default with_check (true per default)
    ///
    /// When set to true, it won't check if End node match last Start node.
    /// If the xml is known to be sane (already processed etc ...) this saves extra time
    pub fn with_check(mut self, val: bool) -> XmlReader<B> {
        self.with_check = val;
        self
    }

    /// private function to read until '<' is found
    fn read_until_open(&mut self) -> Option<Result<Event>> {
        self.tag_state = TagState::Opened;
        let mut buf = Vec::new();
        match read_until(&mut self.reader, b'<', &mut buf) {
            Ok(0) => None,
            Ok(_n) => {
                let (start, len) = if self.trim_text {
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

    fn read_until_close(&mut self) -> Option<Result<Event>> {
        self.tag_state = TagState::Closed;
        let mut buf = Vec::new();
        match read_until(&mut self.reader, b'>', &mut buf) {
            Ok(0) => None,
            Ok(_n) => {
                let len = buf.len();
                match buf[0] {
                    b'/' => {
                        if self.with_check {
                            let e = self.opened.pop().unwrap();
                            if &buf[1..] != e.as_bytes() {
                                self.exit = true;
                                return Some(Err(Error::Malformed(format!(
                                        "End event {:?} doesn't match last opened element {:?}, opened: {:?}", 
                                        Element::new(buf, 1, len, len), e, self.opened))));
                            }
                        }
                        return Some(Ok(Event::End(Element::new(buf, 1, len, len))))
                    },
                    b'?' => {
                        if len > 1 && buf[len - 1] == b'?' {
                            return Some(Ok(Event::Header(Element::new(buf, 1, len - 1, len - 1))));
                        } else {
                            self.exit = true;
                            return Some(Err(Error::Malformed("Unescaped Header event".to_owned())));
                        }
                    },
                    b'!' => {
                        if len >= 3 && &buf[1..3] == b"--" {
                            loop {
                                let len = buf.len();
                                if len >= 5 && &buf[(len - 2)..] == b"--" {
                                    return Some(Ok(Event::Comment(Element::new(buf, 3, len - 2, len - 2))));
                                }
                                buf.push(b'>');
                                match read_until(&mut self.reader, b'>', &mut buf) {
                                    Ok(0) => {
                                        self.exit = true;
                                        return Some(Err(Error::Malformed("Unescaped Comment event".to_owned())));
                                    },
                                    Err(e) => {
                                        self.exit = true;
                                        return Some(Err(Error::from(e)));
                                    },
                                    _ => (),
                                }
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
                                        return Some(Err(Error::Malformed("Unescaped CDATA event".to_owned())));
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
                // TODO: do this directly when reading bufreader ...
                let name_end = buf.iter().position(|&b| is_whitespace(b)).unwrap_or(len);
                if buf[len - 1] == b'/' {
                    self.next_close = true;
                    let element = Element::new(buf, 0, len - 1, 
                                               if name_end < len { name_end } else { len - 1 });
                    self.opened.push(element.clone());
                    Some(Ok(Event::Start(element)))
                } else {
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
    }
}

impl XmlReader<BufReader<File>> {
    /// Creates a xml reader from a file path
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<XmlReader<BufReader<File>>>
    {
        let reader = BufReader::new(try!(File::open(path)));
        Ok(XmlReader::from_reader(reader))
    }
}

impl<'a> XmlReader<&'a [u8]> {
    /// Creates a xml reader for an in memory string buffer.
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
            let e = self.opened.pop().unwrap();
            return Some(Ok(Event::End(e)));
        }
        match self.tag_state {
            TagState::Opened => self.read_until_close(),
            TagState::Closed => self.read_until_open(),
        }
    }

}

#[derive(Clone)]
/// Wrapper around Vec<u8> representing the content of an event (aka node)
///
/// The purpose of not returning a String directly is to postpone calculations (utf8 conversion)
/// to the last moment: byte checks are enough in most cases
pub struct Element {
    buf: Vec<u8>,
    start: usize,
    end: usize,
    name_end: usize,
}

impl Element {

    /// private function to create a new element
    fn new(buf: Vec<u8>, start: usize, end: usize, name_end: usize) -> Element {
        Element {
            buf: buf,
            start: start,
            end: end,
            name_end: name_end,
        }
    }
    
    /// name as &[u8] (without eventual attributes)
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[self.start..self.name_end]
    }

    /// name as str, (without eventual attributes)
    pub fn as_str(&self) -> Result<&str> {
        ::std::str::from_utf8(self.as_bytes()).map_err(Error::Utf8)
    }

    /// get attributes iterator
    pub fn attributes(&self) -> Attributes {
        Attributes::new(&self.buf[self.start..self.end], self.name_end)
    }

    /// consumes entire self (including eventual attributes!) and returns `String`
    ///
    /// useful when we need to get Text event value (which don't have attributes)
    pub fn into_string(self) -> Result<String> {
        ::std::string::String::from_utf8(self.buf).map_err(|e| Error::Utf8(e.utf8_error()))
    }
}

impl fmt::Debug for Element {
    fn fmt(&self, f: &mut fmt::Formatter) -> ::std::result::Result<(), fmt::Error> {
        write!(f, "Element {{ buf: {:?}, name_end: {}, end: {} }}", 
               self.as_str(), self.name_end, self.end)
    }
}

/// Event to interprete node as they are parsed
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

    /// returns inner Element for the event
    pub fn element(&self) -> &Element {
        match *self {
            Event::Start(ref e) |
            Event::End(ref e) |
            Event::Text(ref e) |
            Event::Comment(ref e) |
            Event::CData(ref e) |
            Event::Header(ref e) => e,
        }
    }
}

fn is_whitespace(b: u8) -> bool {
    match b {
        b' ' | b'\r' | b'\n' | b'\t' => true,
        _ => false,
    }
}

/// read_until slighly modified from rust std library
///
/// only change is that we do not write the matching character
#[inline(always)]
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

/// Xml writer
///
/// Consumes a `Write` and writes xml Events
pub struct XmlWriter<W: Write> {
    /// underlying writer
    writer: W
}

impl<W: Write> XmlWriter<W> {

    /// Creates a XmlWriter from a generic Write
    pub fn new(inner: W) -> XmlWriter<W> {
        XmlWriter {
            writer: inner
        }
    }

    /// Consumes this Xml Writer, returning the underlying writer.
    pub fn into_inner(self) -> W { self.writer }

    pub fn write(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Start(ref e) => self.write_start_tag(e),
            Event::End(ref e) => self.write_wrapped_str(b"</", e, b">"),
            Event::Text(ref e) => self.write_bytes(e.as_bytes()),
            Event::Comment(ref e) => self.write_wrapped_str(b"<!--", e, b"-->"),
            Event::CData(ref e) => self.write_wrapped_str(b"<![CDATA[", e, b"]]>"),
            Event::Header(ref e) => self.write_wrapped_str(b"<?", e, b"?>"),
        }
    }

	#[inline]
    fn write_bytes(&mut self, value: &[u8]) -> Result<()> {
        try!(self.writer.write(value));
        Ok(())
    }

    fn write_start_tag(&mut self, element: &Element) -> Result<()> {
        try!(self.write_bytes(b"<"));
        try!(self.write_bytes(element.as_bytes()));
        for attr in element.attributes() {
            let attr = try!(attr);
            try!(self.write_bytes(b" "));
            try!(self.write_bytes(attr.0));
            try!(self.write_bytes(b"=\""));
            try!(self.write_bytes(attr.1.as_bytes()));
            try!(self.write_bytes(b"\""));
        }
        self.write_bytes(b">")
    }

    fn write_wrapped_str(&mut self, before: &[u8], element: &Element, after: &[u8]) -> Result<()> {
        try!(self.write_bytes(before));
        try!(self.write_bytes(element.as_bytes()));
        self.write_bytes(after)
    }

}

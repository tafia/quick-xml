//! Quick XmlReader reader which performs **very** well.
//!
//! # Example
//!
//! ```
//! use quick_xml::{XmlReader, Event};
//! 
//! let xml = r#"<tag1 att1 = "test">
//!                 <tag2><!--Test comment-->Test</tag2>
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
//!                 b"tag1" => println!("attributes values: {:?}", 
//!                                  e.attributes().map(|a| a.unwrap().1).collect::<Vec<_>>()),
//!                 b"tag2" => count += 1,
//!                 _ => (),
//!             }
//!         },
//!         Ok(Event::Text(e)) => txt.push(e.into_string()),
//!         Err(e) => panic!("{:?}", e),
//!         _ => (),
//!     }
//! }
//! ```
//!
//! # Example of transforming XML
//!
//! ```
//! use quick_xml::{Element, Event, XmlReader, XmlWriter};
//! use quick_xml::Event::*;
//! use std::io::Cursor;
//! use std::iter;
//! 
//! let xml = r#"<this_tag k1="v1" k2="v2"><child>text</child></this_tag>"#;
//! let reader = XmlReader::from_str(xml).trim_text(true);
//! let mut writer = XmlWriter::new(Cursor::new(Vec::new()));
//! for r in reader {
//!     match r {
//!         Ok(Event::Start(ref e)) if e.as_bytes() == b"this_tag" => {
//!             // collect existing attributes
//!             let mut attrs = e.attributes().map(|attr| attr.unwrap()).collect::<Vec<_>>();
//!
//!             // adds a new my-key="some value" attribute
//!             attrs.push((b"my-key", "some value"));
//!
//!             // writes the event to the writer
//!             assert!(writer.write(Start(Element::new("my_elem", attrs.into_iter()))).is_ok());
//!         },
//!         Ok(Event::End(ref e)) if e.as_bytes() == b"this_tag" => {
//!             assert!(writer.write(End(Element::new("my_elem", iter::empty::<(&str, &str)>()))).is_ok());
//!         },
//!         Ok(e) => assert!(writer.write(e).is_ok()),
//!         Err(e) => panic!("{:?}", e),
//!     }
//! }
//!
//! let result = writer.into_inner().into_inner();
//! let expected = r#"<my_elem k1="v1" k2="v2" my-key="some value"><child>text</child></my_elem>"#;
//! assert_eq!(result, expected.as_bytes());
//! ```

#![deny(missing_docs)]

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
use std::str::from_utf8;

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

    /// Reads until end element is found
    ///
    /// Manages nested cases where parent and child elements have the same name
    pub fn read_to_end<K: AsRef<[u8]>>(&mut self, end: K) -> Result<()> {
        let mut depth = 0;
        let end = end.as_ref();
        loop {
            match self.next() {
                Some(Ok(Event::End(ref e))) if e.as_bytes() == end => {
                    if depth == 0 { return Ok(()); }
                    depth -= 1;
                },
                Some(Ok(Event::Start(ref e))) if e.as_bytes() == end => depth += 1,
                Some(Err(e)) => return Err(e),
                None => {
                    warn!("EOF instead of {:?}", from_utf8(end));
                    return Err(Error::Unexpected(format!("Reached EOF, expecting {:?} end tag",
                                                         from_utf8(end))));
                },
                _ => (),
            }
        }
    }

    /// Reads next event, if `Event::Text` or `Event::End`, 
    /// then returns a `String`, else returns an error
    pub fn read_text<K: AsRef<[u8]>>(&mut self, end: K) -> Result<String> {
        match self.next() {
            Some(Ok(Event::Text(e))) => {
                self.read_to_end(end).and_then(|_| e.into_string())
            },
            Some(Ok(Event::End(ref e))) if e.as_bytes() == end.as_ref() => Ok("".to_owned()),
            Some(Err(e)) => Err(e),
            None => Err(Error::Unexpected("Reached EOF while reading text".to_owned())),
            Some(Ok(_)) => {
                Err(Error::Unexpected("Cannot read text, expecting Event::Text".to_owned()))
            },
        }
    }

    /// Loop over elements and apply a `f` closure on start elements
    ///
    /// Ends when `end` `Event::End` is found
    /// This helper method is particularly useful for nested searches
    ///
    /// # Example:
    /// ```
    /// # use quick_xml::{XmlReader, Event};
    /// let mut r = XmlReader::from_str("<a><b>test</b>\
    ///     <b>test 2</b><c/><b>test 3</b></a>").trim_text(true);
    /// let mut tests = Vec::new();
    /// r.map_starts::<_, &str>(None, |r, e| match e.as_bytes() {
    ///     b"a" => r.map_starts(Some("a"), |r, e| match e.as_bytes() {
    ///         b"b" => r.read_text("b").map(|t| tests.push(t)),
    ///         name => r.read_to_end(name)
    ///     }),
    ///     name => r.read_to_end(name),
    /// }).unwrap();
    /// ```
    pub fn map_starts<F, K: AsRef<[u8]>>(&mut self, end: Option<K>, mut f: F) -> Result<()>
        where F: FnMut(&mut XmlReader<B>, &Element) -> Result<()> 
    {
        let end = end.as_ref();
        match end {
            Some(end) => {
                let end = end.as_ref();
                loop {
                    match self.next() {
                        Some(Ok(Event::End(ref e))) if e.as_bytes() == end => return Ok(()),
                        Some(Ok(Event::Start(ref e))) => try!(f(self, e)),
                        Some(Err(e)) => return Err(e),
                        None => return Err(Error::Unexpected(format!("Unexpected end of {:?}",
                                                                     from_utf8(end)))),
                        _ => (),
                    }
                }
            },
            None => loop {
                match self.next() {
                    Some(Ok(Event::Start(ref e))) => try!(f(self, e)),
                    None => return Ok(()),
                    Some(Err(e)) => return Err(e),
                    _ => (),
                }
            },
        }
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
                Some(Ok(Event::Text(Element::from_buffer(buf, start, len, len))))
            },
            Err(e) => {
                self.exit = true;
                Some(Err(Error::from(e)))
            },
        }
    }

    /// private function to read until '>' is found
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
                                        Element::from_buffer(buf, 1, len, len), e, self.opened))));
                            }
                        }
                        return Some(Ok(Event::End(Element::from_buffer(buf, 1, len, len))))
                    },
                    b'?' => {
                        if len > 1 && buf[len - 1] == b'?' {
                            return Some(Ok(Event::Header(Element::from_buffer(buf, 1, len - 1, len - 1))));
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
                                    return Some(Ok(Event::Comment(Element::from_buffer(buf, 3, len - 2, len - 2))));
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
                                    return Some(Ok(Event::CData(Element::from_buffer(buf, 8, len - 2, len - 2))));
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
                    let element = Element::from_buffer(buf, 0, len - 1, 
                                                       if name_end < len { name_end } else { len - 1 });
                    self.opened.push(element.clone());
                    Some(Ok(Event::Start(element)))
                } else {
                    let element = Element::from_buffer(buf, 0, len, name_end);
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

    /// Creates a new Element from the given name and attributes.
    /// attributes are represented as an iterator over (key, value) tuples.
    /// Key and value can be anything that implements the AsRef<[u8]> trait,
    /// like byte slices and strings.
    pub fn new<'a, K, V, I>(name: &str, attributes: I) -> Element 
        where K: AsRef<[u8]>, V: AsRef<[u8]>, I: Iterator<Item = (K, V)>
    {
        let mut bytes = Vec::from(name.as_bytes());
        let name_end = bytes.len();
        for attr in attributes {
            bytes.push(b' ');
            bytes.extend_from_slice(attr.0.as_ref());
            bytes.extend_from_slice(b"=\"");
            bytes.extend_from_slice(attr.1.as_ref());
            bytes.push(b'"');
        }
        let end = bytes.len();
        Element {
            buf: bytes,
            start: 0,
            end: end,
            name_end: name_end
        }
    }

    /// private function to create a new element from a buffer.
    fn from_buffer(buf: Vec<u8>, start: usize, end: usize, name_end: usize) -> Element {
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
        from_utf8(self.as_bytes()).map_err(Error::Utf8)
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
    /// <...> eventually with attributes 
    Start(Element),
    /// </...>
    End(Element),
    /// Data between Start and End element
    Text(Element),
    /// <!-- ... -->
    Comment(Element),
    /// <![CDATA[...]]>
    CData(Element),
    /// <?...?>
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

    /// Writes the given event to the underlying writer.
    pub fn write(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Start(e) => self.write_start_tag(e),
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

    fn write_start_tag(&mut self, element: Element) -> Result<()> {
        try!(self.write_bytes(b"<"));
        try!(self.write_bytes(&try!(element.into_string()).into_bytes()));
        self.write_bytes(b">")
    }

    fn write_wrapped_str(&mut self, before: &[u8], element: &Element, after: &[u8]) -> Result<()> {
        try!(self.write_bytes(before));
        try!(self.write_bytes(element.as_bytes()));
        self.write_bytes(after)
    }

}

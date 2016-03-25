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
//!             match e.name() {
//!                 b"tag1" => println!("attributes values: {:?}", 
//!                                  e.attributes().map(|a| a.unwrap().1).collect::<Vec<_>>()),
//!                 b"tag2" => count += 1,
//!                 _ => (),
//!             }
//!         },
//!         Ok(Event::Text(e)) => txt.push(e.into_string()),
//!         Err((e, pos)) => panic!("{:?} at position {}", e, pos),
//!         _ => (),
//!     }
//! }
//! ```
//!
//! # Example of transforming XML
//!
//! ```
//! use quick_xml::{AsStr, Element, Event, XmlReader, XmlWriter};
//! use quick_xml::Event::*;
//! use std::io::Cursor;
//! use std::iter;
//! 
//! let xml = r#"<this_tag k1="v1" k2="v2"><child>text</child></this_tag>"#;
//! let reader = XmlReader::from_str(xml).trim_text(true);
//! let mut writer = XmlWriter::new(Cursor::new(Vec::new()));
//! for r in reader {
//!     match r {
//!         Ok(Event::Start(ref e)) if e.name() == b"this_tag" => {
//!             // collect existing attributes
//!             let mut attrs = e.attributes().map(|attr| attr.unwrap()).collect::<Vec<_>>();
//!
//!             // copy existing attributes, adds a new my-key="some value" attribute
//!             let mut elem = Element::new("my_elem").with_attributes(attrs);
//!             elem.push_attribute(b"my-key", "some value");
//!
//!             // writes the event to the writer
//!             assert!(writer.write(Start(elem)).is_ok());
//!         },
//!         Ok(Event::End(ref e)) if e.name() == b"this_tag" => {
//!             assert!(writer.write(End(Element::new("my_elem"))).is_ok());
//!         },
//!         Ok(e) => assert!(writer.write(e).is_ok()),
//!         Err((e, pos)) => panic!("{:?} at position {}", e, pos),
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
use std::iter::IntoIterator;
use std::path::Path;
use std::fmt;
use std::str::from_utf8;

use error::{Error, Result, ResultPos};
use attributes::Attributes;

enum TagState {
    Opened,
    Closed,
}

/// A trait to support on-demand conversion from UTF-8
pub trait AsStr {
    /// Converts this to an &str
    fn as_str(&self) -> Result<&str>;
}

/// Implements AsStr for a byte slice
impl AsStr for [u8] {
    fn as_str(&self) -> Result<&str> {
        from_utf8(self).map_err(Error::Utf8)
    }
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
    /// current position, useful for debuging errors
    position: usize,
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
            position: 0,
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
    pub fn read_to_end<K: AsRef<[u8]>>(&mut self, end: K) -> ResultPos<()> {
        let mut depth = 0;
        let end = end.as_ref();
        loop {
            match self.next() {
                Some(Ok(Event::End(ref e))) if e.name() == end => {
                    if depth == 0 { return Ok(()); }
                    depth -= 1;
                },
                Some(Ok(Event::Start(ref e))) if e.name() == end => depth += 1,
                Some(Err(e)) => return Err(e),
                None => {
                    warn!("EOF instead of {:?}", from_utf8(end));
                    return Err((Error::Unexpected(
                                format!("Reached EOF, expecting {:?} end tag",
                                        from_utf8(end))), self.position));
                },
                _ => (),
            }
        }
    }

    /// Reads next event, if `Event::Text` or `Event::End`, 
    /// then returns a `String`, else returns an error
    pub fn read_text<K: AsRef<[u8]>>(&mut self, end: K) -> ResultPos<String> {
        match self.next() {
            Some(Ok(Event::Text(e))) => self.read_to_end(end)
                .and_then(|_| e.into_string().map_err(|e| (e, self.position))),
            Some(Ok(Event::End(ref e))) if e.name() == end.as_ref() => Ok("".to_owned()),
            Some(Err(e)) => Err(e),
            None => Err((Error::Unexpected("Reached EOF while reading text".to_owned()), self.position)),
            _ => Err((Error::Unexpected("Cannot read text, expecting Event::Text".to_owned()), self.position)),
        }
    }

    /// Gets the current BufRead position
    /// Useful when debugging errors
    pub fn position(&self) -> usize {
        self.position
    }

    /// private function to read until '<' is found
    fn read_until_open(&mut self) -> Option<Result<Event>> {
        self.tag_state = TagState::Opened;
        let mut buf = Vec::new();
        match read_until(&mut self.reader, b'<', &mut buf) {
            Ok(0) => None,
            Ok(n) => {
                self.position += n;
                let (start, len) = if self.trim_text {
                    match buf.iter().position(|&b| !is_whitespace(b)) {
                        Some(start) => (start, buf.len() - buf.iter().rev()
                                        .position(|&b| !is_whitespace(b)).unwrap_or(0)),
                        None => return self.next().map(|n| n.map_err(|(e, _)| e))
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
            Ok(n) => {
                self.position += n;
                let len = buf.len();
                match buf[0] {
                    b'/' => {
                        if self.with_check {
                            let e = match self.opened.pop() {
                                Some(e) => e,
                                None => return Some(Err(Error::Malformed(format!(
                                        "Cannot close {:?} element, there is no opened element",
                                        buf[1..].as_str())))),
                            };
                            if &buf[1..] != e.name() {
                                self.exit = true;
                                return Some(Err(Error::Malformed(format!(
                                        "End event {:?} doesn't match last opened element {:?}, opened: {:?}", 
                                        Element::from_buffer(buf, 1, len, len), e, self.opened))));
                            }
                        }
                        return Some(Ok(Event::End(Element::from_buffer(buf, 1, len, len))))
                    },
                    b'?' => {
                        if len > 5 && buf[len - 1] == b'?' {
                            if &buf[1..4] == b"xml" && is_whitespace(buf[4]) {
                                return Some(Ok(Event::Decl(XmlDecl { 
                                    element: Element::from_buffer(buf, 1, len - 1, 3)
                                })));
                            } else {
                                return Some(Err(Error::Malformed(
                                            "Xml declaration must start with '?xml '".to_owned())));
                            }
                        } else {
                            self.exit = true;
                            return Some(Err(Error::Malformed("Unescaped XmlDecl event".to_owned())));
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
                    let end = if name_end < len { name_end } else { len - 1 };
                    let element = Element::from_buffer(buf, 0, len - 1, end);
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

    type Item = ResultPos<Event>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.exit { return None; }
        if self.next_close {
            self.next_close = false;
            let e = self.opened.pop().unwrap();
            return Some(Ok(Event::End(e)));
        }
        match self.tag_state {
            TagState::Opened => self.read_until_close(),
            TagState::Closed => self.read_until_open(),
        }.map(|n| n.map_err(|e| (e, self.position)))
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

    /// Creates a new Element from the given name.
    /// name is a reference that can be converted to a byte slice, such as &[u8] or &str
    pub fn new<A>(name: A) -> Element where A: AsRef<[u8]> {
        let bytes = Vec::from(name.as_ref());
        let end = bytes.len();
        Element {
            buf: bytes,
            start: 0,
            end: end,
            name_end: end
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

    /// Consumes self and adds attributes to this element from an iterator over (key, value) tuples.
    /// Key and value can be anything that implements the AsRef<[u8]> trait,
    /// like byte slices and strings.
    pub fn with_attributes<K, V, I>(mut self, attributes: I) -> Self
        where K: AsRef<[u8]>, V: AsRef<[u8]>, I: IntoIterator<Item = (K, V)>
    {
        self.extend_attributes(attributes);
        self
    }

    /// name as &[u8] (without eventual attributes)
    pub fn name(&self) -> &[u8] {
        &self.buf[self.start..self.name_end]
    }

    /// whole content as &[u8] (including eventual attributes)
    pub fn content(&self) -> &[u8] {
        &self.buf[self.start..self.end]
    }

    /// get attributes iterator
    pub fn attributes(&self) -> Attributes {
        Attributes::new(self.content(), self.name_end)
    }

    /// extend the attributes of this element from an iterator over (key, value) tuples.
    /// Key and value can be anything that implements the AsRef<[u8]> trait,
    /// like byte slices and strings.
    pub fn extend_attributes<K, V, I>(&mut self, attributes: I) -> &mut Element
        where K: AsRef<[u8]>, V: AsRef<[u8]>, I: IntoIterator<Item = (K, V)>
    {
        for attr in attributes {
            self.push_attribute(attr.0, attr.1);
        }
        self
    }

    /// consumes entire self (including eventual attributes!) and returns `String`
    ///
    /// useful when we need to get Text event value (which don't have attributes)
    pub fn into_string(self) -> Result<String> {
        ::std::string::String::from_utf8(self.buf).map_err(|e| Error::Utf8(e.utf8_error()))
    }

    /// Adds an attribute to this element from the given key and value.
    /// Key and value can be anything that implements the AsRef<[u8]> trait,
    /// like byte slices and strings.
    pub fn push_attribute<K, V>(&mut self, key: K, value: V)
        where K: AsRef<[u8]>, V: AsRef<[u8]>
    {
        let bytes = &mut self.buf;
        bytes.push(b' ');
        bytes.extend_from_slice(key.as_ref());
        bytes.extend_from_slice(b"=\"");
        bytes.extend_from_slice(value.as_ref());
        bytes.push(b'"');
        self.end = bytes.len();
    }
}

impl fmt::Debug for Element {
    fn fmt(&self, f: &mut fmt::Formatter) -> ::std::result::Result<(), fmt::Error> {
        write!(f, "Element {{ buf: {:?}, name_end: {}, end: {} }}", 
               self.content().as_str(), self.name_end, self.end)
    }
}

/// Wrapper around Element to parse XmlDecl
///
/// Postpone element parsing only when needed
#[derive(Debug)]
pub struct XmlDecl {
    element: Element,
}

impl XmlDecl {

    /// Gets xml version, including quotes (' or ")
    pub fn version(&self) -> Result<&str> {
        match self.element.attributes().next() {
            Some(Err(e)) => Err(e),
            Some(Ok((b"version", v))) => v.as_str(),
            Some(Ok((k, _))) => Err(Error::Malformed(format!(
                        "XmlDecl must start with 'version' attribute, found {:?}", k.as_str()))),
            None => Err(Error::Malformed(
                    "XmlDecl must start with 'version' attribute, found none".to_owned())),
        }
    }

    /// Gets xml encoding, including quotes (' or ")
    pub fn encoding(&self) -> Option<Result<&str>> {
        for a in self.element.attributes() {
            match a {
                Err(e) => return Some(Err(e)),
                Ok((b"encoding", v)) => return Some(v.as_str()),
                _ => (),
            }
        }
        None
    }

    /// Gets xml standalone, including quotes (' or ")
    pub fn standalone(&self) -> Option<Result<&str>> {
        for a in self.element.attributes() {
            match a {
                Err(e) => return Some(Err(e)),
                Ok((b"standalone", v)) => return Some(v.as_str()),
                _ => (),
            }
        }
        None
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
    /// <?xml ...?>
    Decl(XmlDecl),
}

impl Event {

    /// returns inner Element for the event
    pub fn element(&self) -> &Element {
        match *self {
            Event::Start(ref e) |
            Event::End(ref e) |
            Event::Text(ref e) |
            Event::Comment(ref e) |
            Event::CData(ref e) => e,
            Event::Decl(ref e) => &e.element,
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
                Err(e) => return Err(Error::Io(e)),
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
            Event::Start(ref e) => self.write_wrapped_str(b"<", e, b">"),
            Event::End(ref e) => self.write_wrapped_str(b"</", e, b">"),
            Event::Text(ref e) => self.write_bytes(e.content()),
            Event::Comment(ref e) => self.write_wrapped_str(b"<!--", e, b"-->"),
            Event::CData(ref e) => self.write_wrapped_str(b"<![CDATA[", e, b"]]>"),
            Event::Decl(ref e) => self.write_wrapped_str(b"<?", &e.element, b"?>"),
        }
    }

    #[inline]
    fn write_bytes(&mut self, value: &[u8]) -> Result<()> {
        try!(self.writer.write(value));
        Ok(())
    }

    fn write_wrapped_str(&mut self, before: &[u8], element: &Element, after: &[u8])
        -> Result<()> 
    {
        try!(self.write_bytes(before));
        try!(self.write_bytes(&element.content()));
        self.write_bytes(after)
    }

}

//! High performance xml reader/writer.
//!
//! ## Reader
//!
//! Depending on your needs, you can use:
//!
//! - `XmlReader`: for best performance
//! - `XmlnsReader`: if you need to resolve namespaces (around 50% slower than `XmlReader`)
//!
//! ## Writer
//!
//! `XmlWriter`: to write xmls. Can be nested with readers if you want to transform xmls
//! 
//! ## Examples
//! 
//! ### Reader
//! 
//! ```rust
//! use quick_xml::{XmlReader, Event};
//! 
//! let xml = r#"<tag1 att1 = "test">
//!                 <tag2><!--Test comment-->Test</tag2>
//!                 <tag2>
//!                     Test 2
//!                 </tag2>
//!             </tag1>"#;
//! let reader = XmlReader::from(xml).trim_text(true);
//! // if you want to use namespaces, you just need to convert the `XmlReader`
//! // to an `XmlnsReader`:
//! // let reader_ns = reader.namespaced();
//! let mut count = 0;
//! let mut txt = Vec::new();
//! for r in reader {
//! // namespaced: the `for` loop moves the reader
//! // => use `while let` so you can have access to `reader_ns.resolve` for attributes
//! // while let Some(r) = reader.next() {
//!     match r {
//!         Ok(Event::Start(ref e)) => {
//!         // for namespaced:
//!         // Ok((ref namespace_value, Event::Start(ref e)))
//!             match e.name() {
//!                 b"tag1" => println!("attributes values: {:?}", 
//!                                  e.attributes().map(|a| a.unwrap().1)
//!                                  // namespaced: use `reader_ns.resolve`
//!                                  // e.attributes().map(|a| a.map(|(k, _)| reader_ns.resolve(k))) ...
//!                                  .collect::<Vec<_>>()),
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
//! ### Writer
//! 
//! ```rust
//! use quick_xml::{AsStr, Element, Event, XmlReader, XmlWriter};
//! use quick_xml::Event::*;
//! use std::io::Cursor;
//! use std::iter;
//! 
//! let xml = r#"<this_tag k1="v1" k2="v2"><child>text</child></this_tag>"#;
//! let reader = XmlReader::from(xml).trim_text(true);
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
pub mod reader;
pub mod writer;
mod escape;

#[cfg(test)]
mod test;

use std::iter::IntoIterator;
use std::ops::Range;
use std::fmt;
use std::str::from_utf8;
use std::borrow::Cow;

use error::{Error, Result, ResultPos};
use reader::attributes::{Attributes, UnescapedAttributes};
use escape::unescape;

pub use writer::XmlWriter;
pub use reader::XmlReader;

/// A trait to support on-demand conversion from UTF-8
pub trait AsStr {
    /// Converts this to an `&str`
    fn as_str(&self) -> Result<&str>;
}

/// Implements `AsStr` for a byte slice
impl AsStr for [u8] {
    fn as_str(&self) -> Result<&str> {
        from_utf8(self).map_err(Error::Utf8)
    }
}

/// General content of an event (aka node)
///
/// Element is a wrapper over the bytes representing the node:
///
/// E.g. given a node `<name att1="a", att2="b">`, the corresponding `Event` will be
///
/// ```ignore
/// Event::Start(Element {
///     buf:    b"name att1=\"a\", att2=\"b\"",
///     start:  0,
///     end:    b"name att1=\"a\", att2=\"b\"".len(),
///     name_end: b"name".len()
/// })
/// ```
///
/// For performance reasons, most of the time, no character searches but
/// `b'<'` and `b'>'` are performed:
///
/// - no attribute parsing: use lazy `Attributes` iterator only when needed
/// - no namespace awareness as it requires parsing all `Start` element attributes
/// - no utf8 conversion: prefer searching statically binary comparisons
/// then use the `as_str` or `into_string` methods
#[derive(Clone)]
pub struct Element {
    /// content of the element, before any utf8 conversion
    buf: Vec<u8>,
    /// content range, excluding text defining Event type
    content: Range<usize>,
    /// element name range
    name: Range<usize>,
}

impl Element {
    /// Creates a new Element from the given name.
    /// name is a reference that can be converted to a byte slice,
    /// such as &[u8] or &str
    pub fn new<A>(name: A) -> Element
        where A: AsRef<[u8]>
    {
        let bytes = Vec::from(name.as_ref());
        let end = bytes.len();
        Element::from_buffer(bytes, 0, end, end)
    }

    /// private function to create a new element from a buffer.
    #[inline]
    fn from_buffer(buf: Vec<u8>, start: usize, end: usize, name_end: usize)
        -> Element
    {
        Element {
            buf: buf,
            content: Range { start: start, end: end },
            name: Range { start: start, end: name_end },
        }
    }

    /// Consumes self and adds attributes to this element from an iterator
    /// over (key, value) tuples.
    /// Key and value can be anything that implements the AsRef<[u8]> trait,
    /// like byte slices and strings.
    pub fn with_attributes<K, V, I>(mut self, attributes: I) -> Self
        where K: AsRef<[u8]>,
              V: AsRef<[u8]>,
              I: IntoIterator<Item = (K, V)>
    {
        self.extend_attributes(attributes);
        self
    }

    /// name as &[u8] (without eventual attributes)
    pub fn name(&self) -> &[u8] {
        &self.buf[self.name.clone()]
    }

    /// whole content as &[u8] (including eventual attributes)
    pub fn content(&self) -> &[u8] {
        &self.buf[self.content.clone()]
    }

    /// gets escaped content
    ///
    /// Searches for '&' into content and try to escape the coded character if possible
    /// returns Malformed error with index within element if '&' is not followed by ';'
    pub fn unescaped_content(&self) -> ResultPos<Cow<[u8]>> {
        unescape(self.content())
    }

    /// gets attributes iterator
    pub fn attributes(&self) -> Attributes {
        Attributes::new(self.content(), self.name.end)
    }

    /// gets attributes iterator whose attribute values are unescaped ('&...;' replaced
    /// by their corresponding character)
    pub fn unescaped_attributes(&self) -> UnescapedAttributes {
        self.attributes().unescaped()
    }

    /// extend the attributes of this element from an iterator over (key, value) tuples.
    /// Key and value can be anything that implements the AsRef<[u8]> trait,
    /// like byte slices and strings.
    pub fn extend_attributes<K, V, I>(&mut self, attributes: I) -> &mut Element
        where K: AsRef<[u8]>,
              V: AsRef<[u8]>,
              I: IntoIterator<Item = (K, V)>
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
        ::std::string::String::from_utf8(self.buf)
            .map_err(|e| Error::Utf8(e.utf8_error()))
    }
    
    /// consumes entire self (including eventual attributes!) and returns `String`
    ///
    /// useful when we need to get Text event value (which don't have attributes)
    /// and unescape XML entities
    pub fn into_unescaped_string(self) -> Result<String> {
        ::std::string::String::from_utf8(
            try!(self.unescaped_content().map_err(|(e, _)| e)).into_owned())
            .map_err(|e| Error::Utf8(e.utf8_error()))
    }

    /// Adds an attribute to this element from the given key and value.
    /// Key and value can be anything that implements the AsRef<[u8]> trait,
    /// like byte slices and strings.
    pub fn push_attribute<K, V>(&mut self, key: K, value: V)
        where K: AsRef<[u8]>,
              V: AsRef<[u8]>
    {
        let bytes = &mut self.buf;
        bytes.push(b' ');
        bytes.extend_from_slice(key.as_ref());
        bytes.extend_from_slice(b"=\"");
        bytes.extend_from_slice(value.as_ref());
        bytes.push(b'"');
        self.content.end = bytes.len();
    }
}

impl fmt::Debug for Element {
    fn fmt(&self, f: &mut fmt::Formatter) -> ::std::result::Result<(), fmt::Error> {
        write!(f,
               "Element {{ buf: {:?}, name_end: {}, end: {} }}",
               self.content().as_str(),
               self.name.end,
               self.content.end)
    }
}

/// Wrapper around `Element` to parse/write `XmlDecl`
///
/// Postpone element parsing only when needed.
///
/// [W3C XML 1.1 Prolog and Document Type Delcaration](http://w3.org/TR/xml11/#sec-prolog-dtd)
#[derive(Clone, Debug)]
pub struct XmlDecl {
    element: Element,
}

impl XmlDecl {

    /// Gets xml version, including quotes (' or ")
    pub fn version(&self) -> ResultPos<&[u8]> {
        match self.element.attributes().next() {
            Some(Err(e)) => Err(e),
            Some(Ok((b"version", v))) => Ok(v),
            Some(Ok((k, _))) => {
                let m = format!("XmlDecl must start with 'version' attribute, found {:?}",
                                k.as_str());
                Err((Error::Malformed(m), 0))
            }
            None => {
                let m = "XmlDecl must start with 'version' attribute, found none".to_string();
                Err((Error::Malformed(m), 0))
            }
        }
    }

    /// Gets xml encoding, including quotes (' or ")
    pub fn encoding(&self) -> Option<ResultPos<&[u8]>> {
        for a in self.element.attributes() {
            match a {
                Err(e) => return Some(Err(e)),
                Ok((b"encoding", v)) => return Some(Ok(v)),
                _ => (),
            }
        }
        None
    }

    /// Gets xml standalone, including quotes (' or ")
    pub fn standalone(&self) -> Option<ResultPos<&[u8]>> {
        for a in self.element.attributes() {
            match a {
                Err(e) => return Some(Err(e)),
                Ok((b"standalone", v)) => return Some(Ok(v)),
                _ => (),
            }
        }
        None
    }

    /// Constructs a new `XmlDecl` from the (mandatory) _version_ (should be `1.0` or `1.1`),
    /// the optional _encoding_ (e.g., `UTF-8`) and the optional _standalone_ (`yes` or `no`)
    /// attribute.
    ///
    /// Does not escape any of its inputs. Always uses double quotes to wrap the attribute values.
    /// The caller is responsible for escaping attribute values. Shouldn't usually be relevant since
    /// the double quote character is not allowed in any of the attribute values.
    pub fn new(version: &[u8], encoding: Option<&[u8]>, standalone: Option<&[u8]>) -> XmlDecl {
        // Compute length of the buffer based on supplied attributes
        // ' encoding=""'   => 12
        let encoding_attr_len = if let Some(xs) = encoding { 12 + xs.len() } else { 0 };
        // ' standalone=""' => 14
        let standalone_attr_len = if let Some(xs) = standalone { 14 + xs.len() } else { 0 };
        // 'xml version=""' => 14
        let mut buf = Vec::with_capacity(14 + encoding_attr_len + standalone_attr_len);

        buf.extend_from_slice(b"xml version=\"");
        buf.extend_from_slice(version);

        if let Some(encoding_val) = encoding {
            buf.extend_from_slice(b"\" encoding=\"");
            buf.extend_from_slice(encoding_val);
        }

        if let Some(standalone_val) = standalone {
            buf.extend_from_slice(b"\" standalone=\"");
            buf.extend_from_slice(standalone_val);
        }
        buf.push(b'"');

        let buf_len = buf.len();
        XmlDecl { element: Element::from_buffer(buf, 0, buf_len, 3) }
    }
}

/// Event to interprete node as they are parsed
#[derive(Clone, Debug)]
pub enum Event {
    /// Start tag (with attributes) <...>
    Start(Element),
    /// End tag </...>
    End(Element),
    /// Empty element tag (with attributes) <.../>
    Empty(Element),
    /// Data between Start and End element
    Text(Element),
    /// Comment <!-- ... -->
    Comment(Element),
    /// CData <![CDATA[...]]>
    CData(Element),
    /// Xml declaration <?xml ...?>
    Decl(XmlDecl),
    /// Processing instruction <?...?>
    PI(Element),
    /// Doctype <!DOCTYPE...>
    DocType(Element),
}

impl Event {
    /// returns inner Element for the event
    pub fn element(&self) -> &Element {
        match *self {
            Event::Start(ref e) |
            Event::End(ref e) |
            Event::Empty(ref e) |
            Event::Text(ref e) |
            Event::Comment(ref e) |
            Event::CData(ref e) |
            Event::PI(ref e) |
            Event::DocType(ref e) => e,
            Event::Decl(ref e) => &e.element,
        }
    }
}

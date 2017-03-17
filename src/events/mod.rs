//! A module to handle `Event` enumerator

pub mod attributes;

use std::borrow::Cow;
use std::str::from_utf8;
use std::ops::Deref;
use encoding_rs::Encoding;
use std::io::BufRead;

use escape::unescape;
use self::attributes::{Attributes, Attribute};
use errors::Result;
use reader::Reader;

/// A struct to manage `Event::Start` events
///
/// Provides in particular an iterator over attributes
#[derive(Clone, Debug)]
pub struct BytesStart<'a> {
    /// content of the element, before any utf8 conversion
    buf: Cow<'a, [u8]>,
    /// end of the element name, the name starts at that the start of `buf`
    name_len: usize,
}

impl<'a> BytesStart<'a> {
    /// Creates a new `BytesStart` from the given name.
    #[inline]
    pub fn borrowed(content: &'a [u8], name_len: usize) -> BytesStart<'a> {
        BytesStart {
            buf: Cow::Borrowed(content),
            name_len: name_len,
        }
    }

    /// Creates a new `BytesStart` from the given name. Owns its content
    #[inline]
    pub fn owned(content: Vec<u8>, name_len: usize) -> BytesStart<'static> {
        BytesStart {
            buf: Cow::Owned(content),
            name_len: name_len,
        }
    }

    /// Converts the event into an Owned event
    pub fn into_owned(self) -> BytesStart<'static> {
        BytesStart {
            buf: Cow::Owned(self.buf.into_owned()),
            name_len: self.name_len,
        }
    }

    /// Consumes self and adds attributes to this element from an iterator
    /// over (key, value) tuples.
    /// Key and value can be anything that implements the AsRef<[u8]> trait,
    /// like byte slices and strings.
    pub fn with_attributes<'b, I>(mut self, attributes: I) -> Self
        where I: IntoIterator,
              I::Item: Into<Attribute<'b>>
    {
        self.extend_attributes(attributes);
        self
    }

    /// name as &[u8] (without eventual attributes)
    pub fn name(&self) -> &[u8] {
        &self.buf[..self.name_len]
    }

    /// local name (excluding namespace) as &[u8] (without evental attributes)
    /// Note: only events produced by `read_namespaced_event()` will have needed
    /// namespace information to resolve the local name.
    #[inline]
    pub fn local_name<B: BufRead>(&self, rdr: &Reader<B>) -> &[u8] {
        rdr.resolve_namespace(self.name()).1
    }

    /// gets unescaped content
    ///
    /// Searches for '&' into content and try to escape the coded character if possible
    /// returns Malformed error with index within element if '&' is not followed by ';'
    pub fn unescaped(&self) -> Result<Cow<[u8]>> {
        unescape(&*self.buf)
    }

    /// gets attributes iterator
    pub fn attributes(&self) -> Attributes {
        Attributes::new(&self, self.name_len)
    }

    /// extend the attributes of this element from an iterator over (key, value) tuples.
    /// Key and value can be anything that implements the AsRef<[u8]> trait,
    /// like byte slices and strings.
    pub fn extend_attributes<'b, I>(&mut self, attributes: I) -> &mut BytesStart<'a>
        where I: IntoIterator,
              I::Item: Into<Attribute<'b>>
    {
        for attr in attributes {
            self.push_attribute(attr);
        }
        self
    }

    /// helper method to unescape then decode self using the reader encoding (utf8 per default)
    ///
    /// for performance reasons (could avoid allocating a `String`), it might be wiser to manually use
    /// 1. BytesStart::unescaped()
    /// 2. Reader::decode(...)
    pub fn unescape_and_decode<B: BufRead>(&self, reader: &Reader<B>) -> Result<String> {
        self.unescaped().map(|e| reader.decode(&*e).into_owned())
    }

    /// Adds an attribute to this element from the given key and value.
    /// Key and value can be anything that implements the AsRef<[u8]> trait,
    /// like byte slices and strings.
    pub fn push_attribute<'b, A: Into<Attribute<'b>>>(&mut self, attr: A) {
        let a = attr.into();
        let bytes = self.buf.to_mut();
        bytes.push(b' ');
        bytes.extend_from_slice(a.key);
        bytes.extend_from_slice(b"=\"");
        bytes.extend_from_slice(a.value);
        bytes.push(b'"');
    }
}

/// Wrapper around `BytesElement` to parse/write `XmlDecl`
///
/// Postpone element parsing only when needed.
///
/// [W3C XML 1.1 Prolog and Document Type Delcaration](http://w3.org/TR/xml11/#sec-prolog-dtd)
#[derive(Clone, Debug)]
pub struct BytesDecl<'a> {
    element: BytesStart<'a>,
}

impl<'a> BytesDecl<'a> {
    /// Creates a `BytesDecl` from a `BytesStart`
    pub fn from_start(start: BytesStart<'a>) -> BytesDecl<'a> {
        BytesDecl { element: start }
    }

    /// Gets xml version, including quotes (' or ")
    pub fn version(&self) -> Result<&[u8]> {
        match self.element.attributes().next() {
            Some(Err(e)) => Err(e),
            Some(Ok(Attribute { key: b"version", value: v })) => Ok(v),
            Some(Ok(a)) => {
                Err(format!("XmlDecl must start with 'version' attribute, found {:?}",
                            from_utf8(a.key))
                    .into())
            }
            None => Err("XmlDecl must start with 'version' attribute, found none".into()),
        }
    }

    /// Gets xml encoding, including quotes (' or ")
    pub fn encoding(&self) -> Option<Result<&[u8]>> {
        for a in self.element.attributes() {
            match a {
                Err(e) => return Some(Err(e)),
                Ok(Attribute { key: b"encoding", value: v }) => return Some(Ok(v)),
                _ => (),
            }
        }
        None
    }

    /// Gets xml standalone, including quotes (' or ")
    pub fn standalone(&self) -> Option<Result<&[u8]>> {
        for a in self.element.attributes() {
            match a {
                Err(e) => return Some(Err(e)),
                Ok(Attribute { key: b"standalone", value: v }) => return Some(Ok(v)),
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
    pub fn new(version: &[u8],
               encoding: Option<&[u8]>,
               standalone: Option<&[u8]>)
               -> BytesDecl<'static> {
        // Compute length of the buffer based on supplied attributes
        // ' encoding=""'   => 12
        let encoding_attr_len = if let Some(xs) = encoding {
            12 + xs.len()
        } else {
            0
        };
        // ' standalone=""' => 14
        let standalone_attr_len = if let Some(xs) = standalone {
            14 + xs.len()
        } else {
            0
        };
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

        BytesDecl { element: BytesStart::owned(buf, 3) }
    }

    /// Gets the decoder struct
    pub fn encoder(&self) -> Option<&'static Encoding> {
        self.encoding()
            .and_then(|e| e.ok())
            .and_then(|e| Encoding::for_label(e))
    }
}

/// A struct to manage `Event::End` events
#[derive(Clone, Debug)]
pub struct BytesEnd<'a> {
    name: Cow<'a, [u8]>,
}

impl<'a> BytesEnd<'a> {
    /// Creates a new `BytesEnd` borrowing a slice
    #[inline]
    pub fn borrowed(name: &'a [u8]) -> BytesEnd<'a> {
        BytesEnd { name: Cow::Borrowed(name) }
    }

    /// Creates a new `BytesEnd` owning its name
    #[inline]
    pub fn owned(name: Vec<u8>) -> BytesEnd<'static> {
        BytesEnd { name: Cow::Owned(name) }
    }

    /// Gets `BytesEnd` event name
    #[inline]
    pub fn name(&self) -> &[u8] {
        &*self.name
    }

    /// local name (excluding namespace) as &[u8] (without evental attributes)
    /// Note: only events produced by `read_namespaced_event()` will have needed
    /// namespace information to resolve the local name.
    #[inline]
    pub fn local_name<B: BufRead>(&self, rdr: &Reader<B>) -> &[u8] {
        rdr.resolve_namespace(self.name()).1
    }
}

/// A struct to manage `Event::End` events
#[derive(Clone, Debug)]
pub struct BytesText<'a> {
    content: Cow<'a, [u8]>,
}

impl<'a> BytesText<'a> {
    /// Creates a new `BytesEnd` borrowing a slice
    #[inline]
    pub fn borrowed(content: &'a [u8]) -> BytesText<'a> {
        BytesText { content: Cow::Borrowed(content) }
    }

    /// Creates a new `BytesEnd` owning its name
    #[inline]
    pub fn owned(content: Vec<u8>) -> BytesText<'static> {
        BytesText { content: Cow::Owned(content) }
    }

    /// gets escaped content
    ///
    /// Searches for '&' into content and try to escape the coded character if possible
    /// returns Malformed error with index within element if '&' is not followed by ';'
    pub fn unescaped(&self) -> Result<Cow<[u8]>> {
        unescape(&self)
    }

    /// helper method to unescape then decode self using the reader encoding (utf8 per default)
    ///
    /// for performance reasons (could avoid allocating a `String`), it might be wiser to manually use
    /// 1. BytesText::unescaped()
    /// 2. Reader::decode(...)
    pub fn unescape_and_decode<B: BufRead>(&self, reader: &Reader<B>) -> Result<String> {
        self.unescaped().map(|e| reader.decode(&*e).into_owned())
    }
}

/// Event to interprete node as they are parsed
#[derive(Clone, Debug)]
pub enum Event<'a> {
    /// Start tag (with attributes) <...>
    Start(BytesStart<'a>),
    /// End tag </...>
    End(BytesEnd<'a>),
    /// Empty element tag (with attributes) <.../>
    Empty(BytesStart<'a>),
    /// Data between Start and End element
    Text(BytesText<'a>),
    /// Comment <!-- ... -->
    Comment(BytesText<'a>),
    /// CData <![CDATA[...]]>
    CData(BytesText<'a>),
    /// Xml declaration <?xml ...?>
    Decl(BytesDecl<'a>),
    /// Processing instruction <?...?>
    PI(BytesText<'a>),
    /// Doctype <!DOCTYPE...>
    DocType(BytesText<'a>),
    /// Eof of file event
    Eof,
}

impl<'a> Deref for BytesStart<'a> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &*self.buf
    }
}

impl<'a> Deref for BytesDecl<'a> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &*self.element
    }
}

impl<'a> Deref for BytesEnd<'a> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &*self.name
    }
}

impl<'a> Deref for BytesText<'a> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &*self.content
    }
}

impl<'a> Deref for Event<'a> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        match *self {
            Event::Start(ref e) => &*e,
            Event::End(ref e) => &*e,
            Event::Text(ref e) => &*e,
            Event::Empty(ref e) => &*e,
            Event::Decl(ref e) => &*e,
            Event::PI(ref e) => &*e,
            Event::CData(ref e) => &*e,
            Event::Comment(ref e) => &*e,
            Event::DocType(ref e) => &*e,
            Event::Eof => &[],
        }
    }
}

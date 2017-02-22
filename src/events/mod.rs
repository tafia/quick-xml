//! A module to handle `BytesEvent` enumerator

pub mod attributes;

use std::borrow::Cow;
use std::str::from_utf8;
use std::ops::Deref;

use escape::unescape;
use self::attributes::{Attributes, UnescapedAttributes};
use error::{Result, ResultPos, Error};

/// A struct to manage `BytesEvent::Start` events
///
/// Provides in particular an iterator over attributes
#[derive(Clone, Debug)]
pub struct BytesStart<'a> {
    /// content of the element, before any utf8 conversion
    buf: Cow<'a, [u8]>,
    /// end of the element name, the name starts at that the start of `buf`
    name_len: usize
}

impl<'a> BytesStart<'a> {

    /// Creates a new `BytesStart` from the given name.
    #[inline]
    pub fn borrowed(content: &'a[u8], name_len: usize) -> BytesStart<'a> {
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
    pub fn with_attributes<K, V, I>(&mut self, attributes: I) -> &mut Self
        where K: AsRef<[u8]>,
              V: AsRef<[u8]>,
              I: IntoIterator<Item = (K, V)>
    {
        self.extend_attributes(attributes);
        self
    }

    /// name as &[u8] (without eventual attributes)
    pub fn name(&self) -> &[u8] {
        &self.buf[..self.name_len]
    }

    /// gets unescaped content
    ///
    /// Searches for '&' into content and try to escape the coded character if possible
    /// returns Malformed error with index within element if '&' is not followed by ';'
    pub fn unescaped(&self) -> ResultPos<Cow<[u8]>> {
        unescape(&self)
    }

    /// gets attributes iterator
    pub fn attributes(&self) -> Attributes {
        Attributes::new(&self, self.name_len)
    }

    /// gets attributes iterator whose attribute values are unescaped ('&...;' replaced
    /// by their corresponding character)
    pub fn unescaped_attributes(&self) -> UnescapedAttributes {
        self.attributes().unescaped()
    }

    /// extend the attributes of this element from an iterator over (key, value) tuples.
    /// Key and value can be anything that implements the AsRef<[u8]> trait,
    /// like byte slices and strings.
    pub fn extend_attributes<K, V, I>(&mut self, attributes: I) -> &mut BytesStart<'a>
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
        ::std::string::String::from_utf8(self.buf.into_owned())
            .map_err(|e| Error::Utf8(e.utf8_error()))
    }
    
    /// consumes entire self (including eventual attributes!) and returns `String`
    ///
    /// useful when we need to get Text event value (which don't have attributes)
    /// and unescape XML entities
    pub fn into_unescaped_string(self) -> Result<String> {
        ::std::string::String::from_utf8(
            try!(self.unescaped().map_err(|(e, _)| e)).into_owned())
            .map_err(|e| Error::Utf8(e.utf8_error()))
    }

    /// Adds an attribute to this element from the given key and value.
    /// Key and value can be anything that implements the AsRef<[u8]> trait,
    /// like byte slices and strings.
    pub fn push_attribute<K, V>(&mut self, key: K, value: V)
        where K: AsRef<[u8]>,
              V: AsRef<[u8]>
    {
        let bytes = self.buf.to_mut();
        bytes.push(b' ');
        bytes.extend_from_slice(key.as_ref());
        bytes.extend_from_slice(b"=\"");
        bytes.extend_from_slice(value.as_ref());
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
    pub fn new(version: &[u8], encoding: Option<&[u8]>, standalone: Option<&[u8]>) -> BytesDecl<'static> {
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

        BytesDecl { element: BytesStart::owned(buf, 3) }
    }
}

/// A struct to manage `BytesEvent::End` events
#[derive(Clone, Debug)]
pub struct BytesEnd<'a> {
    name: Cow<'a, [u8]>
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
}

/// A struct to manage `BytesEvent::End` events
#[derive(Clone, Debug)]
pub struct BytesText<'a> {
    content: Cow<'a, [u8]>
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
    pub fn unescaped(&self) -> ResultPos<Cow<[u8]>> {
        unescape(&self)
    }

    /// consumes entire self (including eventual attributes!) and returns `String`
    ///
    /// useful when we need to get Text event value (which don't have attributes)
    pub fn into_string(self) -> Result<String> {
        ::std::string::String::from_utf8(self.content.into_owned())
            .map_err(|e| Error::Utf8(e.utf8_error()))
    }
    
    /// consumes entire self (including eventual attributes!) and returns `String`
    ///
    /// useful when we need to get Text event value (which don't have attributes)
    /// and unescape XML entities
    pub fn into_unescaped_string(self) -> Result<String> {
        ::std::string::String::from_utf8(
            try!(self.unescaped().map_err(|(e, _)| e)).into_owned())
            .map_err(|e| Error::Utf8(e.utf8_error()))
    }
}

/// BytesEvent to interprete node as they are parsed
#[derive(Clone, Debug)]
pub enum BytesEvent<'a> {
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

impl<'a> Deref for BytesEvent<'a> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        match *self {
            BytesEvent::Start(ref e) => &*e,
            BytesEvent::End(ref e) => &*e,
            BytesEvent::Text(ref e) => &*e,
            BytesEvent::Empty(ref e) => &*e,
            BytesEvent::Decl(ref e) => &*e,
            BytesEvent::PI(ref e) => &*e,
            BytesEvent::CData(ref e) => &*e,
            BytesEvent::Comment(ref e) => &*e,
            BytesEvent::DocType(ref e) => &*e,
            BytesEvent::Eof => &[],
        }
    }
}

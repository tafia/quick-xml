//! Defines zero-copy XML events used throughout this library.
//!
//! A XML event often represents part of a XML element.
//! They occur both during reading and writing and are
//! usually used with the stream-oriented API.
//!
//! For example, the XML element
//! ```xml
//! <name attr="value">Inner text</name>
//! ```
//! consists of the three events `Start`, `Text` and `End`.
//! They can also represent other parts in an XML document like the
//! XML declaration. Each Event usually contains further information,
//! like the tag name, the attribute or the inner text.
//!
//! See [`Event`] for a list of all possible events.
//!
//! # Reading
//! When reading a XML stream, the events are emitted by
//! [`Reader::read_event_into`]. You must listen
//! for the different types of events you are interested in.
//!
//! See [`Reader`] for further information.
//!
//! # Writing
//! When writing the XML document, you must create the XML element
//! by constructing the events it consists of and pass them to the writer
//! sequentially.
//!
//! See [`Writer`] for further information.
//!
//! [`Writer`]: crate::writer::Writer
//! [`Event`]: crate::events::Event

pub mod attributes;

#[cfg(feature = "encoding")]
use encoding_rs::Encoding;
use std::borrow::Cow;
use std::fmt::{self, Debug, Formatter};
use std::ops::Deref;
use std::str::from_utf8;

use crate::errors::{Error, Result};
use crate::escape::{escape, partial_escape, unescape_with};
use crate::name::{LocalName, QName};
use crate::utils::write_cow_string;
use attributes::{Attribute, Attributes};

/// Text that appeared before an XML declaration, a start element or a comment.
///
/// In well-formed XML it could contain a Byte-Order-Mark (BOM). If this event
/// contains something else except BOM, the XML should be considered ill-formed.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BytesStartText<'a> {
    content: BytesText<'a>,
}

impl<'a> BytesStartText<'a> {
    /// Converts the event into an owned event.
    pub fn into_owned(self) -> BytesStartText<'static> {
        BytesStartText {
            content: self.content.into_owned(),
        }
    }

    /// Extracts the inner `Cow` from the `BytesStartText` event container.
    #[inline]
    pub fn into_inner(self) -> Cow<'a, [u8]> {
        self.content.into_inner()
    }

    /// Converts the event into a borrowed event.
    #[inline]
    pub fn borrow(&self) -> BytesStartText {
        BytesStartText {
            content: self.content.borrow(),
        }
    }

    // TODO: clean this up
    // /// Decodes bytes of event, stripping byte order mark (BOM) if it is presented
    // /// in the event.
    // ///
    // /// This method does not unescapes content, because no escape sequences can
    // /// appeared in the BOM or in the text before the first tag.
    // pub fn decode_with_bom_removal(&self, decoder: Decoder) -> Result<String> {
    //     //TODO: Fix lifetime issue - it should be possible to borrow string
    //     let decoded = decoder.decode_with_bom_removal(&*self)?;

    //     Ok(decoded.to_string())
    // }
}

impl<'a> Deref for BytesStartText<'a> {
    type Target = BytesText<'a>;

    fn deref(&self) -> &Self::Target {
        &self.content
    }
}

impl<'a> From<BytesText<'a>> for BytesStartText<'a> {
    fn from(content: BytesText<'a>) -> Self {
        Self { content }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// Opening tag data (`Event::Start`), with optional attributes.
///
/// `<name attr="value">`.
///
/// The name can be accessed using the [`name`] or [`local_name`] methods.
/// An iterator over the attributes is returned by the [`attributes`] method.
///
/// [`name`]: Self::name
/// [`local_name`]: Self::local_name
/// [`attributes`]: Self::attributes
#[derive(Clone, Eq, PartialEq)]
pub struct BytesStart<'a> {
    /// content of the element, before any utf8 conversion
    pub(crate) buf: Cow<'a, [u8]>,
    /// end of the element name, the name starts at that the start of `buf`
    pub(crate) name_len: usize,
}

impl<'a> BytesStart<'a> {
    /// Creates a new `BytesStart` from the given content (name + attributes).
    ///
    /// # Warning
    ///
    /// `&content[..name_len]` is not checked to be a valid name
    #[inline]
    pub fn borrowed(content: &'a [u8], name_len: usize) -> Self {
        BytesStart {
            buf: Cow::Borrowed(content),
            name_len,
        }
    }

    /// Creates a new `BytesStart` from the given name.
    ///
    /// # Warning
    ///
    /// `&content` is not checked to be a valid name
    #[inline]
    pub fn borrowed_name(name: &'a [u8]) -> BytesStart<'a> {
        Self::borrowed(name, name.len())
    }

    /// Creates a new `BytesStart` from the given content (name + attributes)
    ///
    /// Owns its contents.
    #[inline]
    pub fn owned<C: Into<Vec<u8>>>(content: C, name_len: usize) -> BytesStart<'static> {
        BytesStart {
            buf: Cow::Owned(content.into()),
            name_len,
        }
    }

    /// Creates a new `BytesStart` from the given name
    ///
    /// Owns its contents.
    #[inline]
    pub fn owned_name<C: Into<Vec<u8>>>(name: C) -> BytesStart<'static> {
        let content = name.into();
        BytesStart {
            name_len: content.len(),
            buf: Cow::Owned(content),
        }
    }

    /// Converts the event into an owned event.
    pub fn into_owned(self) -> BytesStart<'static> {
        Self::owned(self.buf.into_owned(), self.name_len)
    }

    /// Converts the event into an owned event without taking ownership of Event
    pub fn to_owned(&self) -> BytesStart<'static> {
        Self::owned(self.buf.to_owned(), self.name_len)
    }

    /// Converts the event into a borrowed event. Most useful when paired with [`to_end`].
    ///
    /// # Example
    ///
    /// ```rust
    /// # use quick_xml::{Error, Writer};
    /// use quick_xml::events::{BytesStart, Event};
    ///
    /// struct SomeStruct<'a> {
    ///     attrs: BytesStart<'a>,
    ///     // ...
    /// }
    /// # impl<'a> SomeStruct<'a> {
    /// # fn example(&self) -> Result<(), Error> {
    /// # let mut writer = Writer::new(Vec::new());
    ///
    /// writer.write_event(Event::Start(self.attrs.borrow()))?;
    /// // ...
    /// writer.write_event(Event::End(self.attrs.to_end()))?;
    /// # Ok(())
    /// # }}
    /// ```
    ///
    /// [`to_end`]: Self::to_end
    pub fn borrow(&self) -> BytesStart {
        BytesStart::borrowed(&self.buf, self.name_len)
    }

    /// Creates new paired close tag
    pub fn to_end(&self) -> BytesEnd {
        BytesEnd::borrowed(self.name().into_inner())
    }

    /// Gets the undecoded raw tag name, as present in the input stream.
    #[inline]
    pub fn name(&self) -> QName {
        QName(&self.buf[..self.name_len])
    }

    /// Gets the undecoded raw local tag name (excluding namespace) as present
    /// in the input stream.
    ///
    /// All content up to and including the first `:` character is removed from the tag name.
    #[inline]
    pub fn local_name(&self) -> LocalName {
        self.name().into()
    }

    /// Edit the name of the BytesStart in-place
    ///
    /// # Warning
    ///
    /// `name` is not checked to be a valid name
    pub fn set_name(&mut self, name: &[u8]) -> &mut BytesStart<'a> {
        let bytes = self.buf.to_mut();
        bytes.splice(..self.name_len, name.iter().cloned());
        self.name_len = name.len();
        self
    }
}

/// Attribute-related methods
impl<'a> BytesStart<'a> {
    /// Consumes `self` and yield a new `BytesStart` with additional attributes from an iterator.
    ///
    /// The yielded items must be convertible to [`Attribute`] using `Into`.
    pub fn with_attributes<'b, I>(mut self, attributes: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Attribute<'b>>,
    {
        self.extend_attributes(attributes);
        self
    }

    /// Add additional attributes to this tag using an iterator.
    ///
    /// The yielded items must be convertible to [`Attribute`] using `Into`.
    pub fn extend_attributes<'b, I>(&mut self, attributes: I) -> &mut BytesStart<'a>
    where
        I: IntoIterator,
        I::Item: Into<Attribute<'b>>,
    {
        for attr in attributes {
            self.push_attribute(attr);
        }
        self
    }

    /// Adds an attribute to this element.
    pub fn push_attribute<'b, A>(&mut self, attr: A)
    where
        A: Into<Attribute<'b>>,
    {
        let a = attr.into();
        let bytes = self.buf.to_mut();
        bytes.push(b' ');
        bytes.extend_from_slice(a.key.as_ref());
        bytes.extend_from_slice(b"=\"");
        bytes.extend_from_slice(&*a.value);
        bytes.push(b'"');
    }

    /// Remove all attributes from the ByteStart
    pub fn clear_attributes(&mut self) -> &mut BytesStart<'a> {
        self.buf.to_mut().truncate(self.name_len);
        self
    }

    /// Returns an iterator over the attributes of this tag.
    pub fn attributes(&self) -> Attributes {
        Attributes::new(&self.buf, self.name_len)
    }

    /// Returns an iterator over the HTML-like attributes of this tag (no mandatory quotes or `=`).
    pub fn html_attributes(&self) -> Attributes {
        Attributes::html(self, self.name_len)
    }

    /// Gets the undecoded raw string with the attributes of this tag as a `&[u8]`,
    /// including the whitespace after the tag name if there is any.
    #[inline]
    pub fn attributes_raw(&self) -> &[u8] {
        &self.buf[self.name_len..]
    }

    /// Try to get an attribute
    pub fn try_get_attribute<N: AsRef<[u8]> + Sized>(
        &'a self,
        attr_name: N,
    ) -> Result<Option<Attribute<'a>>> {
        for a in self.attributes() {
            let a = a?;
            if a.key.as_ref() == attr_name.as_ref() {
                return Ok(Some(a));
            }
        }
        Ok(None)
    }
}

impl<'a> Debug for BytesStart<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "BytesStart {{ buf: ")?;
        write_cow_string(f, &self.buf)?;
        write!(f, ", name_len: {} }}", self.name_len)
    }
}

impl<'a> Deref for BytesStart<'a> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &*self.buf
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// An XML declaration (`Event::Decl`).
///
/// [W3C XML 1.1 Prolog and Document Type Declaration](http://w3.org/TR/xml11/#sec-prolog-dtd)
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BytesDecl<'a> {
    content: BytesStart<'a>,
}

impl<'a> BytesDecl<'a> {
    /// Creates a `BytesDecl` from a `BytesStart`
    pub fn from_start(start: BytesStart<'a>) -> Self {
        Self { content: start }
    }

    /// Gets xml version, excluding quotes (`'` or `"`).
    ///
    /// According to the [grammar], the version *must* be the first thing in the declaration.
    /// This method tries to extract the first thing in the declaration and return it.
    /// In case of multiple attributes value of the first one is returned.
    ///
    /// If version is missed in the declaration, or the first thing is not a version,
    /// [`Error::XmlDeclWithoutVersion`] will be returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::borrow::Cow;
    /// use quick_xml::Error;
    /// use quick_xml::events::{BytesDecl, BytesStart};
    ///
    /// // <?xml version='1.1'?>
    /// let decl = BytesDecl::from_start(BytesStart::borrowed(b" version='1.1'", 0));
    /// assert_eq!(
    ///     decl.version().unwrap(),
    ///     Cow::Borrowed(b"1.1".as_ref())
    /// );
    ///
    /// // <?xml version='1.0' version='1.1'?>
    /// let decl = BytesDecl::from_start(BytesStart::borrowed(b" version='1.0' version='1.1'", 0));
    /// assert_eq!(
    ///     decl.version().unwrap(),
    ///     Cow::Borrowed(b"1.0".as_ref())
    /// );
    ///
    /// // <?xml encoding='utf-8'?>
    /// let decl = BytesDecl::from_start(BytesStart::borrowed(b" encoding='utf-8'", 0));
    /// match decl.version() {
    ///     Err(Error::XmlDeclWithoutVersion(Some(key))) => assert_eq!(key, "encoding".to_string()),
    ///     _ => assert!(false),
    /// }
    ///
    /// // <?xml encoding='utf-8' version='1.1'?>
    /// let decl = BytesDecl::from_start(BytesStart::borrowed(b" encoding='utf-8' version='1.1'", 0));
    /// match decl.version() {
    ///     Err(Error::XmlDeclWithoutVersion(Some(key))) => assert_eq!(key, "encoding".to_string()),
    ///     _ => assert!(false),
    /// }
    ///
    /// // <?xml?>
    /// let decl = BytesDecl::from_start(BytesStart::borrowed(b"", 0));
    /// match decl.version() {
    ///     Err(Error::XmlDeclWithoutVersion(None)) => {},
    ///     _ => assert!(false),
    /// }
    /// ```
    ///
    /// [grammar]: https://www.w3.org/TR/xml11/#NT-XMLDecl
    pub fn version(&self) -> Result<Cow<[u8]>> {
        // The version *must* be the first thing in the declaration.
        match self.content.attributes().with_checks(false).next() {
            Some(Ok(a)) if a.key.as_ref() == b"version" => Ok(a.value),
            // first attribute was not "version"
            Some(Ok(a)) => {
                let found = from_utf8(a.key.as_ref())?.to_string();
                Err(Error::XmlDeclWithoutVersion(Some(found)))
            }
            // error parsing attributes
            Some(Err(e)) => Err(e.into()),
            // no attributes
            None => Err(Error::XmlDeclWithoutVersion(None)),
        }
    }

    /// Gets xml encoding, excluding quotes (`'` or `"`).
    ///
    /// Although according to the [grammar] encoding must appear before `"standalone"`
    /// and after `"version"`, this method does not check that. The first occurrence
    /// of the attribute will be returned even if there are several. Also, method does
    /// not restrict symbols that can forming the encoding, so the returned encoding
    /// name may not correspond to the grammar.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::borrow::Cow;
    /// use quick_xml::Error;
    /// use quick_xml::events::{BytesDecl, BytesStart};
    ///
    /// // <?xml version='1.1'?>
    /// let decl = BytesDecl::from_start(BytesStart::borrowed(b" version='1.1'", 0));
    /// assert!(decl.encoding().is_none());
    ///
    /// // <?xml encoding='utf-8'?>
    /// let decl = BytesDecl::from_start(BytesStart::borrowed(b" encoding='utf-8'", 0));
    /// match decl.encoding() {
    ///     Some(Ok(Cow::Borrowed(encoding))) => assert_eq!(encoding, b"utf-8"),
    ///     _ => assert!(false),
    /// }
    ///
    /// // <?xml encoding='something_WRONG' encoding='utf-8'?>
    /// let decl = BytesDecl::from_start(BytesStart::borrowed(b" encoding='something_WRONG' encoding='utf-8'", 0));
    /// match decl.encoding() {
    ///     Some(Ok(Cow::Borrowed(encoding))) => assert_eq!(encoding, b"something_WRONG"),
    ///     _ => assert!(false),
    /// }
    /// ```
    ///
    /// [grammar]: https://www.w3.org/TR/xml11/#NT-XMLDecl
    pub fn encoding(&self) -> Option<Result<Cow<[u8]>>> {
        self.content
            .try_get_attribute("encoding")
            .map(|a| a.map(|a| a.value))
            .transpose()
    }

    /// Gets xml standalone, excluding quotes (`'` or `"`).
    ///
    /// Although according to the [grammar] standalone flag must appear after `"version"`
    /// and `"encoding"`, this method does not check that. The first occurrence of the
    /// attribute will be returned even if there are several. Also, method does not
    /// restrict symbols that can forming the value, so the returned flag name may not
    /// correspond to the grammar.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::borrow::Cow;
    /// use quick_xml::Error;
    /// use quick_xml::events::{BytesDecl, BytesStart};
    ///
    /// // <?xml version='1.1'?>
    /// let decl = BytesDecl::from_start(BytesStart::borrowed(b" version='1.1'", 0));
    /// assert!(decl.standalone().is_none());
    ///
    /// // <?xml standalone='yes'?>
    /// let decl = BytesDecl::from_start(BytesStart::borrowed(b" standalone='yes'", 0));
    /// match decl.standalone() {
    ///     Some(Ok(Cow::Borrowed(encoding))) => assert_eq!(encoding, b"yes"),
    ///     _ => assert!(false),
    /// }
    ///
    /// // <?xml standalone='something_WRONG' encoding='utf-8'?>
    /// let decl = BytesDecl::from_start(BytesStart::borrowed(b" standalone='something_WRONG' encoding='utf-8'", 0));
    /// match decl.standalone() {
    ///     Some(Ok(Cow::Borrowed(flag))) => assert_eq!(flag, b"something_WRONG"),
    ///     _ => assert!(false),
    /// }
    /// ```
    ///
    /// [grammar]: https://www.w3.org/TR/xml11/#NT-XMLDecl
    pub fn standalone(&self) -> Option<Result<Cow<[u8]>>> {
        self.content
            .try_get_attribute("standalone")
            .map(|a| a.map(|a| a.value))
            .transpose()
    }

    /// Constructs a new `XmlDecl` from the (mandatory) _version_ (should be `1.0` or `1.1`),
    /// the optional _encoding_ (e.g., `UTF-8`) and the optional _standalone_ (`yes` or `no`)
    /// attribute.
    ///
    /// Does not escape any of its inputs. Always uses double quotes to wrap the attribute values.
    /// The caller is responsible for escaping attribute values. Shouldn't usually be relevant since
    /// the double quote character is not allowed in any of the attribute values.
    pub fn new(
        version: &[u8],
        encoding: Option<&[u8]>,
        standalone: Option<&[u8]>,
    ) -> BytesDecl<'static> {
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

        BytesDecl {
            content: BytesStart::owned(buf, 3),
        }
    }

    /// Gets the decoder struct
    #[cfg(feature = "encoding")]
    pub fn encoder(&self) -> Option<&'static Encoding> {
        self.encoding()
            .and_then(|e| e.ok())
            .and_then(|e| Encoding::for_label(&*e))
    }

    /// Converts the event into an owned event.
    pub fn into_owned(self) -> BytesDecl<'static> {
        BytesDecl {
            content: self.content.into_owned(),
        }
    }

    /// Converts the event into a borrowed event.
    #[inline]
    pub fn borrow(&self) -> BytesDecl {
        BytesDecl {
            content: self.content.borrow(),
        }
    }
}

impl<'a> Deref for BytesDecl<'a> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &*self.content
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// A struct to manage `Event::End` events
#[derive(Clone, Eq, PartialEq)]
pub struct BytesEnd<'a> {
    name: Cow<'a, [u8]>,
}

impl<'a> BytesEnd<'a> {
    /// Creates a new `BytesEnd` borrowing a slice
    #[inline]
    pub fn borrowed(name: &'a [u8]) -> BytesEnd<'a> {
        BytesEnd {
            name: Cow::Borrowed(name),
        }
    }

    /// Creates a new `BytesEnd` owning its name
    #[inline]
    pub fn owned(name: Vec<u8>) -> BytesEnd<'static> {
        BytesEnd {
            name: Cow::Owned(name),
        }
    }

    /// Converts the event into an owned event.
    pub fn into_owned(self) -> BytesEnd<'static> {
        BytesEnd {
            name: Cow::Owned(self.name.into_owned()),
        }
    }

    /// Converts the event into a borrowed event.
    #[inline]
    pub fn borrow(&self) -> BytesEnd {
        BytesEnd {
            name: Cow::Borrowed(&self.name),
        }
    }

    /// Gets the undecoded raw tag name, as present in the input stream.
    #[inline]
    pub fn name(&self) -> QName {
        QName(&*self.name)
    }

    /// Gets the undecoded raw local tag name (excluding namespace) as present
    /// in the input stream.
    ///
    /// All content up to and including the first `:` character is removed from the tag name.
    #[inline]
    pub fn local_name(&self) -> LocalName {
        self.name().into()
    }
}

impl<'a> Debug for BytesEnd<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "BytesEnd {{ name: ")?;
        write_cow_string(f, &self.name)?;
        write!(f, " }}")
    }
}

impl<'a> Deref for BytesEnd<'a> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &*self.name
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// Data from various events (most notably, `Event::Text`) that stored in XML
/// in escaped form. Internally data is stored in escaped form
#[derive(Clone, Eq, PartialEq)]
pub struct BytesText<'a> {
    /// Escaped then encoded content of the event. Content is encoded in the XML
    /// document encoding when event comes from the reader and should be in the
    /// document encoding when event passed to the writer
    content: Cow<'a, [u8]>,
}

impl<'a> BytesText<'a> {
    /// Creates a new `BytesText` from an escaped byte sequence.
    #[inline]
    pub fn from_escaped<C: Into<Cow<'a, [u8]>>>(content: C) -> Self {
        Self {
            content: content.into(),
        }
    }

    /// Creates a new `BytesText` from a byte sequence. The byte sequence is
    /// expected not to be escaped.
    #[inline]
    pub fn from_plain(content: &'a [u8]) -> Self {
        Self {
            content: escape(content),
        }
    }

    /// Creates a new `BytesText` from an escaped string.
    #[inline]
    pub fn from_escaped_str<C: Into<Cow<'a, str>>>(content: C) -> Self {
        Self::from_escaped(match content.into() {
            Cow::Owned(o) => Cow::Owned(o.into_bytes()),
            Cow::Borrowed(b) => Cow::Borrowed(b.as_bytes()),
        })
    }

    /// Creates a new `BytesText` from a string. The string is expected not to
    /// be escaped.
    #[inline]
    pub fn from_plain_str(content: &'a str) -> Self {
        Self::from_plain(content.as_bytes())
    }

    /// Ensures that all data is owned to extend the object's lifetime if
    /// necessary.
    #[inline]
    pub fn into_owned(self) -> BytesText<'static> {
        BytesText {
            content: self.content.into_owned().into(),
        }
    }

    /// Extracts the inner `Cow` from the `BytesText` event container.
    #[inline]
    pub fn into_inner(self) -> Cow<'a, [u8]> {
        self.content
    }

    /// Converts the event into a borrowed event.
    #[inline]
    pub fn borrow(&self) -> BytesText {
        BytesText {
            content: Cow::Borrowed(&self.content),
        }
    }

    /// Returns the unescaped content of the event.
    ///
    /// Searches for '&' into content and try to escape the coded character if possible
    /// returns Malformed error with index within element if '&' is not followed by ';'
    ///
    /// See also [`unescape_with()`](Self::unescape_with)
    pub fn unescape(&self) -> Result<Cow<str>> {
        self.unescape_with(|_| None)
    }

    /// Returns the unescaped content of the event with a custom entity resolver.
    ///
    /// Searches for '&' into content and try to escape the coded character if possible
    /// returns Malformed error with index within element if '&' is not followed by ';'
    /// A fallback resolver for additional custom entities can be provided via `resolve_entity`.
    ///
    /// See also [`unescape()`](Self::unescape)
    pub fn unescape_with<'entity>(
        &self,
        resolve_entity: impl Fn(&str) -> Option<&'entity str>,
    ) -> Result<Cow<str>> {
        Ok(unescape_with(from_utf8(self)?, resolve_entity)?) // TODO(dalley): this is temporary
    }

    /// Gets escaped content.
    pub fn escape(&self) -> &str {
        from_utf8(self).unwrap() // TODO(dalley): this is temporary
    }
}

impl<'a> Debug for BytesText<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "BytesText {{ content: ")?;
        write_cow_string(f, &self.content)?;
        write!(f, " }}")
    }
}

impl<'a> Deref for BytesText<'a> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &*self.content
    }
}

impl<'a> From<BytesStartText<'a>> for BytesText<'a> {
    fn from(content: BytesStartText<'a>) -> Self {
        content.content
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// CDATA content contains unescaped data from the reader. If you want to write them as a text,
/// [convert](Self::escape) it to [`BytesText`]
#[derive(Clone, Eq, PartialEq)]
pub struct BytesCData<'a> {
    content: Cow<'a, [u8]>,
}

impl<'a> BytesCData<'a> {
    /// Creates a new `BytesCData` from a byte sequence.
    #[inline]
    pub fn new<C: Into<Cow<'a, [u8]>>>(content: C) -> Self {
        Self {
            content: content.into(),
        }
    }

    /// Creates a new `BytesCData` from a string
    #[inline]
    pub fn from_str(content: &'a str) -> Self {
        Self::new(content.as_bytes())
    }

    /// Ensures that all data is owned to extend the object's lifetime if
    /// necessary.
    #[inline]
    pub fn into_owned(self) -> BytesCData<'static> {
        BytesCData {
            content: self.content.into_owned().into(),
        }
    }

    /// Extracts the inner `Cow` from the `BytesCData` event container.
    #[inline]
    pub fn into_inner(self) -> Cow<'a, [u8]> {
        self.content
    }

    /// Converts the event into a borrowed event.
    #[inline]
    pub fn borrow(&self) -> BytesCData {
        BytesCData {
            content: Cow::Borrowed(&self.content),
        }
    }

    /// Converts this CDATA content to an escaped version, that can be written
    /// as an usual text in XML.
    ///
    /// This function performs following replacements:
    ///
    /// | Character | Replacement
    /// |-----------|------------
    /// | `<`       | `&lt;`
    /// | `>`       | `&gt;`
    /// | `&`       | `&amp;`
    /// | `'`       | `&apos;`
    /// | `"`       | `&quot;`
    pub fn escape(self) -> BytesText<'a> {
        let content = std::str::from_utf8(&self.content).unwrap(); // TODO(dalley): this is temporary
        BytesText::from_escaped(escape(&content)).into_owned()
    }

    /// Converts this CDATA content to an escaped version, that can be written
    /// as an usual text in XML.
    ///
    /// In XML text content, it is allowed (though not recommended) to leave
    /// the quote special characters `"` and `'` unescaped.
    ///
    /// This function performs following replacements:
    ///
    /// | Character | Replacement
    /// |-----------|------------
    /// | `<`       | `&lt;`
    /// | `>`       | `&gt;`
    /// | `&`       | `&amp;`
    pub fn partial_escape(self) -> BytesText<'a> {
        let content = std::str::from_utf8(&self.content).unwrap(); // TODO(dalley): this is temporary
        BytesText::from_escaped(partial_escape(&content)).into_owned()
    }
}

impl<'a> Debug for BytesCData<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "BytesCData {{ content: ")?;
        write_cow_string(f, &self.content)?;
        write!(f, " }}")
    }
}

impl<'a> Deref for BytesCData<'a> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &*self.content
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// Event emitted by [`Reader::read_event_into`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Event<'a> {
    /// Text that appeared before the first opening tag or an [XML declaration].
    /// [According to the XML standard][std], no text allowed before the XML
    /// declaration. However, if there is a BOM in the stream, some data may be
    /// present.
    ///
    /// When this event is generated, it is the very first event emitted by the
    /// [`Reader`], and there can be the only one such event.
    ///
    /// The [`Writer`] writes content of this event "as is" without encoding or
    /// escaping. If you write it, it should be written first and only one time
    /// (but writer does not enforce that).
    ///
    /// # Examples
    ///
    /// ```
    /// # use pretty_assertions::assert_eq;
    /// use std::borrow::Cow;
    /// use quick_xml::Reader;
    /// use quick_xml::events::Event;
    ///
    /// // XML in UTF-8 with BOM
    /// let xml = b"\xEF\xBB\xBF<?xml version='1.0'?>";
    /// let mut reader = Reader::from_bytes(xml);
    /// let mut events_processed = 0;
    /// loop {
    ///     match reader.read_event() {
    ///         Ok(Event::StartText(e)) => {
    ///             assert_eq!(events_processed, 0);
    ///             // Content contains BOM
    ///             assert_eq!(e.into_inner(), Cow::Borrowed(b"\xEF\xBB\xBF"));
    ///         }
    ///         Ok(Event::Decl(_)) => {
    ///             assert_eq!(events_processed, 1);
    ///         }
    ///         Ok(Event::Eof) => {
    ///             assert_eq!(events_processed, 2);
    ///             break;
    ///         }
    ///         e => panic!("Unexpected event {:?}", e),
    ///     }
    ///     events_processed += 1;
    /// }
    /// ```
    ///
    /// [XML declaration]: Event::Decl
    /// [std]: https://www.w3.org/TR/xml11/#NT-document
    /// [`Writer`]: crate::writer::Writer
    StartText(BytesStartText<'a>),
    /// Start tag (with attributes) `<tag attr="value">`.
    Start(BytesStart<'a>),
    /// End tag `</tag>`.
    End(BytesEnd<'a>),
    /// Empty element tag (with attributes) `<tag attr="value" />`.
    Empty(BytesStart<'a>),
    /// Character data between `Start` and `End` element.
    Text(BytesText<'a>),
    /// Comment `<!-- ... -->`.
    Comment(BytesText<'a>),
    /// CData `<![CDATA[...]]>`.
    CData(BytesCData<'a>),
    /// XML declaration `<?xml ...?>`.
    Decl(BytesDecl<'a>),
    /// Processing instruction `<?...?>`.
    PI(BytesText<'a>),
    /// Doctype `<!DOCTYPE ...>`.
    DocType(BytesText<'a>),
    /// End of XML document.
    Eof,
}

impl<'a> Event<'a> {
    /// Converts the event to an owned version, untied to the lifetime of
    /// buffer used when reading but incurring a new, separate allocation.
    pub fn into_owned(self) -> Event<'static> {
        match self {
            Event::StartText(e) => Event::StartText(e.into_owned()),
            Event::Start(e) => Event::Start(e.into_owned()),
            Event::End(e) => Event::End(e.into_owned()),
            Event::Empty(e) => Event::Empty(e.into_owned()),
            Event::Text(e) => Event::Text(e.into_owned()),
            Event::Comment(e) => Event::Comment(e.into_owned()),
            Event::CData(e) => Event::CData(e.into_owned()),
            Event::Decl(e) => Event::Decl(e.into_owned()),
            Event::PI(e) => Event::PI(e.into_owned()),
            Event::DocType(e) => Event::DocType(e.into_owned()),
            Event::Eof => Event::Eof,
        }
    }

    /// Converts the event into a borrowed event.
    #[inline]
    pub fn borrow(&self) -> Event {
        match self {
            Event::StartText(e) => Event::StartText(e.borrow()),
            Event::Start(e) => Event::Start(e.borrow()),
            Event::End(e) => Event::End(e.borrow()),
            Event::Empty(e) => Event::Empty(e.borrow()),
            Event::Text(e) => Event::Text(e.borrow()),
            Event::Comment(e) => Event::Comment(e.borrow()),
            Event::CData(e) => Event::CData(e.borrow()),
            Event::Decl(e) => Event::Decl(e.borrow()),
            Event::PI(e) => Event::PI(e.borrow()),
            Event::DocType(e) => Event::DocType(e.borrow()),
            Event::Eof => Event::Eof,
        }
    }
}

impl<'a> Deref for Event<'a> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        match *self {
            Event::StartText(ref e) => &*e,
            Event::Start(ref e) | Event::Empty(ref e) => &*e,
            Event::End(ref e) => &*e,
            Event::Text(ref e) => &*e,
            Event::Decl(ref e) => &*e,
            Event::PI(ref e) => &*e,
            Event::CData(ref e) => &*e,
            Event::Comment(ref e) => &*e,
            Event::DocType(ref e) => &*e,
            Event::Eof => &[],
        }
    }
}

impl<'a> AsRef<Event<'a>> for Event<'a> {
    fn as_ref(&self) -> &Event<'a> {
        self
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn bytestart_create() {
        let b = BytesStart::owned_name("test");
        assert_eq!(b.len(), 4);
        assert_eq!(b.name(), QName(b"test"));
    }

    #[test]
    fn bytestart_set_name() {
        let mut b = BytesStart::owned_name("test");
        assert_eq!(b.len(), 4);
        assert_eq!(b.name(), QName(b"test"));
        assert_eq!(b.attributes_raw(), b"");
        b.push_attribute(("x", "a"));
        assert_eq!(b.len(), 10);
        assert_eq!(b.attributes_raw(), b" x=\"a\"");
        b.set_name(b"g");
        assert_eq!(b.len(), 7);
        assert_eq!(b.name(), QName(b"g"));
    }

    #[test]
    fn bytestart_clear_attributes() {
        let mut b = BytesStart::owned_name("test");
        b.push_attribute(("x", "y\"z"));
        b.push_attribute(("x", "y\"z"));
        b.clear_attributes();
        assert!(b.attributes().next().is_none());
        assert_eq!(b.len(), 4);
        assert_eq!(b.name(), QName(b"test"));
    }
}

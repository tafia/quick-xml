//! Serde `Deserializer` module
//!
//! # Examples
//!
//! Here is a simple example parsing [crates.io](https://crates.io/) source code.
//!
//! ```
//! // Cargo.toml
//! // [dependencies]
//! // serde = { version = "1.0", features = [ "derive" ] }
//! // quick-xml = { version = "0.22", features = [ "serialize" ] }
//! # use pretty_assertions::assert_eq;
//! use serde::Deserialize;
//! use quick_xml::de::{from_str, DeError};
//!
//! #[derive(Debug, Deserialize, PartialEq)]
//! struct Link {
//!     rel: String,
//!     href: String,
//!     sizes: Option<String>,
//! }
//!
//! #[derive(Debug, Deserialize, PartialEq)]
//! #[serde(rename_all = "lowercase")]
//! enum Lang {
//!     En,
//!     Fr,
//!     De,
//! }
//!
//! #[derive(Debug, Deserialize, PartialEq)]
//! struct Head {
//!     title: String,
//!     #[serde(rename = "link", default)]
//!     links: Vec<Link>,
//! }
//!
//! #[derive(Debug, Deserialize, PartialEq)]
//! struct Script {
//!     src: String,
//!     integrity: String,
//! }
//!
//! #[derive(Debug, Deserialize, PartialEq)]
//! struct Body {
//!     #[serde(rename = "script", default)]
//!     scripts: Vec<Script>,
//! }
//!
//! #[derive(Debug, Deserialize, PartialEq)]
//! struct Html {
//!     lang: Option<String>,
//!     head: Head,
//!     body: Body,
//! }
//!
//! fn crates_io() -> Result<Html, DeError> {
//!     let xml = "<!DOCTYPE html>
//!         <html lang=\"en\">
//!           <head>
//!             <meta charset=\"utf-8\">
//!             <meta http-equiv=\"X-UA-Compatible\" content=\"IE=edge\">
//!             <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">
//!
//!             <title>crates.io: Rust Package Registry</title>
//!
//!
//!         <meta name=\"cargo/config/environment\" content=\"%7B%22modulePrefix%22%3A%22cargo%22%2C%22environment%22%3A%22production%22%2C%22rootURL%22%3A%22%2F%22%2C%22locationType%22%3A%22router-scroll%22%2C%22historySupportMiddleware%22%3Atrue%2C%22EmberENV%22%3A%7B%22FEATURES%22%3A%7B%7D%2C%22EXTEND_PROTOTYPES%22%3A%7B%22Date%22%3Afalse%7D%7D%2C%22APP%22%3A%7B%22name%22%3A%22cargo%22%2C%22version%22%3A%22b7796c9%22%7D%2C%22fastboot%22%3A%7B%22hostWhitelist%22%3A%5B%22crates.io%22%2C%7B%7D%2C%7B%7D%5D%7D%2C%22ember-cli-app-version%22%3A%7B%22version%22%3A%22b7796c9%22%7D%2C%22ember-cli-mirage%22%3A%7B%22usingProxy%22%3Afalse%2C%22useDefaultPassthroughs%22%3Atrue%7D%2C%22exportApplicationGlobal%22%3Afalse%7D\" />
//!         <!-- EMBER_CLI_FASTBOOT_TITLE --><!-- EMBER_CLI_FASTBOOT_HEAD -->
//!         <link rel=\"manifest\" href=\"/manifest.webmanifest\">
//!         <link rel=\"apple-touch-icon\" href=\"/cargo-835dd6a18132048a52ac569f2615b59d.png\" sizes=\"227x227\">
//!         <meta name=\"theme-color\" content=\"#f9f7ec\">
//!         <meta name=\"apple-mobile-web-app-capable\" content=\"yes\">
//!         <meta name=\"apple-mobile-web-app-title\" content=\"crates.io: Rust Package Registry\">
//!         <meta name=\"apple-mobile-web-app-status-bar-style\" content=\"default\">
//!
//!             <link rel=\"stylesheet\" href=\"/assets/vendor-8d023d47762d5431764f589a6012123e.css\" integrity=\"sha256-EoB7fsYkdS7BZba47+C/9D7yxwPZojsE4pO7RIuUXdE= sha512-/SzGQGR0yj5AG6YPehZB3b6MjpnuNCTOGREQTStETobVRrpYPZKneJwcL/14B8ufcvobJGFDvnTKdcDDxbh6/A==\" >
//!             <link rel=\"stylesheet\" href=\"/assets/cargo-cedb8082b232ce89dd449d869fb54b98.css\" integrity=\"sha256-S9K9jZr6nSyYicYad3JdiTKrvsstXZrvYqmLUX9i3tc= sha512-CDGjy3xeyiqBgUMa+GelihW394pqAARXwsU+HIiOotlnp1sLBVgO6v2ZszL0arwKU8CpvL9wHyLYBIdfX92YbQ==\" >
//!
//!
//!             <link rel=\"shortcut icon\" href=\"/favicon.ico\" type=\"image/x-icon\">
//!             <link rel=\"icon\" href=\"/cargo-835dd6a18132048a52ac569f2615b59d.png\" type=\"image/png\">
//!             <link rel=\"search\" href=\"/opensearch.xml\" type=\"application/opensearchdescription+xml\" title=\"Cargo\">
//!           </head>
//!           <body>
//!             <!-- EMBER_CLI_FASTBOOT_BODY -->
//!             <noscript>
//!                 <div id=\"main\">
//!                     <div class='noscript'>
//!                         This site requires JavaScript to be enabled.
//!                     </div>
//!                 </div>
//!             </noscript>
//!
//!             <script src=\"/assets/vendor-bfe89101b20262535de5a5ccdc276965.js\" integrity=\"sha256-U12Xuwhz1bhJXWyFW/hRr+Wa8B6FFDheTowik5VLkbw= sha512-J/cUUuUN55TrdG8P6Zk3/slI0nTgzYb8pOQlrXfaLgzr9aEumr9D1EzmFyLy1nrhaDGpRN1T8EQrU21Jl81pJQ==\" ></script>
//!             <script src=\"/assets/cargo-4023b68501b7b3e17b2bb31f50f5eeea.js\" integrity=\"sha256-9atimKc1KC6HMJF/B07lP3Cjtgr2tmET8Vau0Re5mVI= sha512-XJyBDQU4wtA1aPyPXaFzTE5Wh/mYJwkKHqZ/Fn4p/ezgdKzSCFu6FYn81raBCnCBNsihfhrkb88uF6H5VraHMA==\" ></script>
//!
//!
//!           </body>
//!         </html>
//! }";
//!     let html: Html = from_str(xml)?;
//!     assert_eq!(&html.head.title, "crates.io: Rust Package Registr");
//!     Ok(html)
//! }
//! ```

mod escape;
mod map;
mod seq;
mod var;

pub use crate::errors::serialize::DeError;
use crate::{
    errors::Error,
    events::{BytesCData, BytesEnd, BytesStart, BytesText, Event},
    reader::Decoder,
    Reader,
};
use serde::de::{self, Deserialize, DeserializeOwned, Visitor};
use serde::serde_if_integer128;
use std::borrow::Cow;
use std::io::BufRead;

pub(crate) const INNER_VALUE: &str = "$value";
pub(crate) const UNFLATTEN_PREFIX: &str = "$unflatten=";
pub(crate) const PRIMITIVE_PREFIX: &str = "$primitive=";

/// Simplified event which contains only these variants that used by deserializer
#[derive(Debug, PartialEq)]
pub enum DeEvent<'a> {
    /// Start tag (with attributes) `<tag attr="value">`.
    Start(BytesStart<'a>),
    /// End tag `</tag>`.
    End(BytesEnd<'a>),
    /// Escaped character data between `Start` and `End` element.
    Text(BytesText<'a>),
    /// Unescaped character data between `Start` and `End` element,
    /// stored in `<![CDATA[...]]>`.
    CData(BytesCData<'a>),
    /// End of XML document.
    Eof,
}

/// An xml deserializer
pub struct Deserializer<'de, R>
where
    R: XmlRead<'de>,
{
    reader: R,
    peek: Option<DeEvent<'de>>,
    /// Special sing that deserialized struct have a field with the special
    /// name (see constant `INNER_VALUE`). That field should be deserialized
    /// from the text content of the XML node:
    ///
    /// ```xml
    /// <tag>value for INNER_VALUE field<tag>
    /// ```
    has_value_field: bool,
}

/// Deserialize an instance of type `T` from a string of XML text.
pub fn from_str<'de, T>(s: &'de str) -> Result<T, DeError>
where
    T: Deserialize<'de>,
{
    from_slice(s.as_bytes())
}

/// Deserialize an instance of type `T` from bytes of XML text.
#[deprecated = "Use `from_slice` instead"]
pub fn from_bytes<'de, T>(s: &'de [u8]) -> Result<T, DeError>
where
    T: Deserialize<'de>,
{
    from_slice(s)
}

/// Deserialize an instance of type `T` from bytes of XML text.
pub fn from_slice<'de, T>(s: &'de [u8]) -> Result<T, DeError>
where
    T: Deserialize<'de>,
{
    let mut de = Deserializer::from_slice(s);
    T::deserialize(&mut de)
}

/// Deserialize from a reader. This method will do internal copies of data
/// readed from `reader`. If you want have a `&[u8]` or `&str` input and want
/// to borrow as much as possible, use [`from_slice`] or [`from_str`]
pub fn from_reader<R, T>(reader: R) -> Result<T, DeError>
where
    R: BufRead,
    T: DeserializeOwned,
{
    let mut de = Deserializer::from_reader(reader);
    T::deserialize(&mut de)
}

// TODO: According to the https://www.w3.org/TR/xmlschema-2/#boolean,
// valid boolean representations are only "true", "false", "1", and "0"
fn deserialize_bool<'de, V>(value: &[u8], decoder: Decoder, visitor: V) -> Result<V::Value, DeError>
where
    V: Visitor<'de>,
{
    #[cfg(feature = "encoding")]
    {
        let value = decoder.decode(value);
        // No need to unescape because valid boolean representations cannot be escaped
        match value.as_ref() {
            "true" | "1" | "True" | "TRUE" | "t" | "Yes" | "YES" | "yes" | "y" => {
                visitor.visit_bool(true)
            }
            "false" | "0" | "False" | "FALSE" | "f" | "No" | "NO" | "no" | "n" => {
                visitor.visit_bool(false)
            }
            _ => Err(DeError::InvalidBoolean(value.into())),
        }
    }

    #[cfg(not(feature = "encoding"))]
    {
        // No need to unescape because valid boolean representations cannot be escaped
        match value {
            b"true" | b"1" | b"True" | b"TRUE" | b"t" | b"Yes" | b"YES" | b"yes" | b"y" => {
                visitor.visit_bool(true)
            }
            b"false" | b"0" | b"False" | b"FALSE" | b"f" | b"No" | b"NO" | b"no" | b"n" => {
                visitor.visit_bool(false)
            }
            e => Err(DeError::InvalidBoolean(decoder.decode(e)?.into())),
        }
    }
}

impl<'de, R> Deserializer<'de, R>
where
    R: XmlRead<'de>,
{
    /// Create an XML deserializer from one of the possible quick_xml input sources.
    ///
    /// Typically it is more convenient to use one of these methods instead:
    ///
    ///  - [`Deserializer::from_str`]
    ///  - [`Deserializer::from_slice`]
    ///  - [`Deserializer::from_reader`]
    pub fn new(reader: R) -> Self {
        Deserializer {
            reader,
            peek: None,
            has_value_field: false,
        }
    }

    /// Get a new deserializer from a regular BufRead
    #[deprecated = "Use `Deserializer::new` instead"]
    pub fn from_borrowing_reader(reader: R) -> Self {
        Self::new(reader)
    }

    fn peek(&mut self) -> Result<&DeEvent<'de>, DeError> {
        if self.peek.is_none() {
            self.peek = Some(self.reader.next()?);
        }
        match self.peek.as_ref() {
            Some(v) => Ok(v),
            // SAFETY: a `None` variant for `self.peek` would have been replaced
            // by a `Some` variant in the code above.
            // TODO: Can be replaced with `unsafe { std::hint::unreachable_unchecked() }`
            // if unsafe code will be allowed
            None => unreachable!(),
        }
    }

    fn next(&mut self) -> Result<DeEvent<'de>, DeError> {
        if let Some(e) = self.peek.take() {
            return Ok(e);
        }
        self.reader.next()
    }

    fn next_start(&mut self) -> Result<Option<BytesStart<'de>>, DeError> {
        loop {
            let e = self.next()?;
            match e {
                DeEvent::Start(e) => return Ok(Some(e)),
                DeEvent::End(_) => return Err(DeError::End),
                DeEvent::Eof => return Ok(None),
                _ => (), // ignore texts
            }
        }
    }

    /// Consumes a one XML element or an XML tree, returns associated text or
    /// an empty string.
    ///
    /// # Handling events
    ///
    /// The table below shows how events is handled by this method:
    ///
    /// |Event             |XML                        |Handling
    /// |------------------|---------------------------|----------------------------------------
    /// |[`DeEvent::Start`]|`<tag>...</tag>`           |Result determined by the second table
    /// |[`DeEvent::End`]  |`</any-tag>`               |Emits [`DeError::End`]
    /// |[`DeEvent::Text`] |`text content`             |Unescapes `text content` and returns it
    /// |[`DeEvent::CData`]|`<![CDATA[cdata content]]>`|Returns `cdata content` unchanged
    /// |[`DeEvent::Eof`]  |                           |Emits [`DeError::Eof`]
    ///
    /// Second event, consumed if [`DeEvent::Start`] was received:
    ///
    /// |Event             |XML                        |Handling
    /// |------------------|---------------------------|----------------------------------------------------------------------------------
    /// |[`DeEvent::Start`]|`<any-tag>...</any-tag>`   |Emits [`DeError::Start`]
    /// |[`DeEvent::End`]  |`</tag>`                   |Returns an empty slice, if close tag matched the open one
    /// |[`DeEvent::End`]  |`</any-tag>`               |Emits [`DeError::End`]
    /// |[`DeEvent::Text`] |`text content`             |Unescapes `text content` and returns it, consumes events up to `</tag>`
    /// |[`DeEvent::CData`]|`<![CDATA[cdata content]]>`|Returns `cdata content` unchanged, consumes events up to `</tag>`
    /// |[`DeEvent::Eof`]  |                           |Emits [`DeError::Eof`]
    fn next_text(&mut self, unescape: bool) -> Result<BytesCData<'de>, DeError> {
        match self.next()? {
            DeEvent::Text(e) if unescape => e.unescape().map_err(|e| DeError::Xml(e.into())),
            DeEvent::Text(e) => Ok(BytesCData::new(e.into_inner())),
            DeEvent::CData(e) => Ok(e),
            DeEvent::Start(e) => {
                // allow one nested level
                let inner = self.next()?;
                let t = match inner {
                    DeEvent::Text(t) if unescape => t.unescape()?,
                    DeEvent::Text(t) => BytesCData::new(t.into_inner()),
                    DeEvent::CData(t) => t,
                    DeEvent::Start(_) => return Err(DeError::Start),
                    // We can get End event in case of `<tag></tag>` or `<tag/>` input
                    // Return empty text in that case
                    DeEvent::End(end) if end.name() == e.name() => {
                        return Ok(BytesCData::new(&[] as &[u8]));
                    }
                    DeEvent::End(_) => return Err(DeError::End),
                    DeEvent::Eof => return Err(DeError::Eof),
                };
                self.read_to_end(e.name())?;
                Ok(t)
            }
            DeEvent::End(_) => Err(DeError::End),
            DeEvent::Eof => Err(DeError::Eof),
        }
    }

    fn read_to_end(&mut self, name: &[u8]) -> Result<(), DeError> {
        // First one might be in self.peek
        match self.next()? {
            DeEvent::Start(e) => self.reader.read_to_end(e.name())?,
            DeEvent::End(e) if e.name() == name => return Ok(()),
            _ => (),
        }
        self.reader.read_to_end(name)
    }
}

impl<'de> Deserializer<'de, SliceReader<'de>> {
    /// Create new deserializer that will borrow data from the specified string
    pub fn from_str(s: &'de str) -> Self {
        Self::from_slice(s.as_bytes())
    }

    /// Create new deserializer that will borrow data from the specified byte array
    pub fn from_slice(bytes: &'de [u8]) -> Self {
        let mut reader = Reader::from_bytes(bytes);
        reader
            .expand_empty_elements(true)
            .check_end_names(true)
            .trim_text(true);
        Self::new(SliceReader { reader })
    }
}

impl<'de, R> Deserializer<'de, IoReader<R>>
where
    R: BufRead,
{
    /// Create new deserializer that will copy data from the specified reader
    /// into internal buffer. If you already have a string or a byte array, use
    /// [`Self::from_str`] or [`Self::from_slice`] instead, because they will
    /// borrow instead of copy, whenever possible
    pub fn from_reader(reader: R) -> Self {
        let mut reader = Reader::from_reader(reader);
        reader
            .expand_empty_elements(true)
            .check_end_names(true)
            .trim_text(true);

        Self::new(IoReader {
            reader,
            buf: Vec::new(),
        })
    }
}

macro_rules! deserialize_type {
    ($deserialize:ident => $visit:ident) => {
        fn $deserialize<V>(self, visitor: V) -> Result<V::Value, DeError>
        where
            V: Visitor<'de>,
        {
            // No need to unescape because valid integer representations cannot be escaped
            let text = self.next_text(false)?;
            let string = text.decode(self.reader.decoder())?;
            visitor.$visit(string.parse()?)
        }
    };
}

impl<'de, 'a, R> de::Deserializer<'de> for &'a mut Deserializer<'de, R>
where
    R: XmlRead<'de>,
{
    type Error = DeError;

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        // Try to go to the next `<tag ...>...</tag>` or `<tag .../>`
        if let Some(e) = self.next_start()? {
            let name = e.name().to_vec();
            self.has_value_field = fields.contains(&INNER_VALUE);
            let map = map::MapAccess::new(self, e, fields)?;
            let value = visitor.visit_map(map)?;
            self.has_value_field = false;
            self.read_to_end(&name)?;
            Ok(value)
        } else {
            Err(DeError::Start)
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        // No need to unescape because valid integer representations cannot be escaped
        let text = self.next_text(false)?;

        deserialize_bool(text.as_ref(), self.reader.decoder(), visitor)
    }

    /// Representation of owned strings the same as [non-owned](#method.deserialize_str).
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        let text = self.next_text(true)?;
        let string = text.decode(self.reader.decoder())?;
        match string {
            Cow::Borrowed(string) => visitor.visit_borrowed_str(string),
            Cow::Owned(string) => visitor.visit_string(string),
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        // No need to unescape because bytes gives access to the raw XML input
        let text = self.next_text(false)?;
        visitor.visit_bytes(&text)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        // No need to unescape because bytes gives access to the raw XML input
        let text = self.next_text(false)?;
        let value = text.into_inner().into_owned();
        visitor.visit_byte_buf(value)
    }

    /// Unit represented in XML as a `xs:element` or text/CDATA content.
    /// Any content inside `xs:element` is ignored and skipped.
    ///
    /// Produces unit struct from any of following inputs:
    /// - any `<tag ...>...</tag>`
    /// - any `<tag .../>`
    /// - any text content
    /// - any CDATA content
    ///
    /// # Events handling
    ///
    /// |Event             |XML                        |Handling
    /// |------------------|---------------------------|-------------------------------------------
    /// |[`DeEvent::Start`]|`<tag>...</tag>`           |Calls `visitor.visit_unit()`, consumes all events up to corresponding `End` event
    /// |[`DeEvent::End`]  |`</tag>`                   |Emits [`DeError::End`]
    /// |[`DeEvent::Text`] |`text content`             |Calls `visitor.visit_unit()`. Text content is ignored
    /// |[`DeEvent::CData`]|`<![CDATA[cdata content]]>`|Calls `visitor.visit_unit()`. CDATA content is ignored
    /// |[`DeEvent::Eof`]  |                           |Emits [`DeError::Eof`]
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        match self.next()? {
            DeEvent::Start(s) => {
                self.read_to_end(s.name())?;
                visitor.visit_unit()
            }
            DeEvent::Text(_) | DeEvent::CData(_) => visitor.visit_unit(),
            DeEvent::End(_) => Err(DeError::End),
            DeEvent::Eof => Err(DeError::Eof),
        }
    }

    /// Representation of the names units the same as [unnamed units](#method.deserialize_unit)
    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(1, visitor)
    }

    /// Representation of tuples the same as [sequences](#method.deserialize_seq).
    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    /// Representation of named tuples the same as [unnamed tuples](#method.deserialize_tuple).
    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        let value = visitor.visit_enum(var::EnumAccess::new(self))?;
        Ok(value)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(seq::SeqAccess::new(self)?)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        self.deserialize_struct("", &[], visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        match self.peek()? {
            DeEvent::Text(t) if t.is_empty() => visitor.visit_none(),
            DeEvent::CData(t) if t.is_empty() => visitor.visit_none(),
            DeEvent::Eof => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    /// Always call `visitor.visit_unit()` because returned value ignored in any case.
    ///
    /// This method consumes any single [event][DeEvent] except the [`Start`][DeEvent::Start]
    /// event, in which case all events up to corresponding [`End`][DeEvent::End] event will
    /// be consumed.
    ///
    /// This method returns error if current event is [`End`][DeEvent::End] or [`Eof`][DeEvent::Eof]
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        match self.next()? {
            DeEvent::Start(e) => self.read_to_end(e.name())?,
            DeEvent::End(_) => return Err(DeError::End),
            DeEvent::Eof => return Err(DeError::Eof),
            _ => (),
        }
        visitor.visit_unit()
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        match self.peek()? {
            DeEvent::Start(_) => self.deserialize_map(visitor),
            // Redirect to deserialize_unit in order to consume an event and return an appropriate error
            DeEvent::End(_) | DeEvent::Eof => self.deserialize_unit(visitor),
            _ => self.deserialize_string(visitor),
        }
    }

    deserialize_type!(deserialize_i8 => visit_i8);
    deserialize_type!(deserialize_i16 => visit_i16);
    deserialize_type!(deserialize_i32 => visit_i32);
    deserialize_type!(deserialize_i64 => visit_i64);
    deserialize_type!(deserialize_u8 => visit_u8);
    deserialize_type!(deserialize_u16 => visit_u16);
    deserialize_type!(deserialize_u32 => visit_u32);
    deserialize_type!(deserialize_u64 => visit_u64);
    deserialize_type!(deserialize_f32 => visit_f32);
    deserialize_type!(deserialize_f64 => visit_f64);

    serde_if_integer128! {
        deserialize_type!(deserialize_i128 => visit_i128);
        deserialize_type!(deserialize_u128 => visit_u128);
    }
}

/// Trait used by the deserializer for iterating over input. This is manually
/// "specialized" for iterating over `&[u8]`.
///
/// You do not need to implement this trait, it is needed to abstract from
/// [borrowing](SliceReader) and [copying](IoReader) data sources and reuse code in
/// deserializer
pub trait XmlRead<'i> {
    /// Return an input-borrowing event.
    fn next(&mut self) -> Result<DeEvent<'i>, DeError>;

    /// Skips until end element is found. Unlike `next()` it will not allocate
    /// when it cannot satisfy the lifetime.
    fn read_to_end(&mut self, name: &[u8]) -> Result<(), DeError>;

    /// A copy of the reader's decoder used to decode strings.
    fn decoder(&self) -> Decoder;
}

/// XML input source that reads from a std::io input stream.
///
/// You cannot create it, it is created automatically when you call
/// [`Deserializer::from_reader`]
pub struct IoReader<R: BufRead> {
    reader: Reader<R>,
    buf: Vec<u8>,
}

impl<'i, R: BufRead> XmlRead<'i> for IoReader<R> {
    fn next(&mut self) -> Result<DeEvent<'static>, DeError> {
        let event = loop {
            let e = self.reader.read_event(&mut self.buf)?;
            match e {
                Event::Start(e) => break Ok(DeEvent::Start(e.into_owned())),
                Event::End(e) => break Ok(DeEvent::End(e.into_owned())),
                Event::Text(e) => break Ok(DeEvent::Text(e.into_owned())),
                Event::CData(e) => break Ok(DeEvent::CData(e.into_owned())),
                Event::Eof => break Ok(DeEvent::Eof),

                _ => self.buf.clear(),
            }
        };

        self.buf.clear();

        event
    }

    fn read_to_end(&mut self, name: &[u8]) -> Result<(), DeError> {
        match self.reader.read_to_end(name, &mut self.buf) {
            Err(Error::UnexpectedEof(_)) => Err(DeError::Eof),
            other => Ok(other?),
        }
    }

    fn decoder(&self) -> Decoder {
        self.reader.decoder()
    }
}

/// XML input source that reads from a slice of bytes and can borrow from it.
///
/// You cannot create it, it is created automatically when you call
/// [`Deserializer::from_str`] or [`Deserializer::from_slice`]
pub struct SliceReader<'de> {
    reader: Reader<&'de [u8]>,
}

impl<'de> XmlRead<'de> for SliceReader<'de> {
    fn next(&mut self) -> Result<DeEvent<'de>, DeError> {
        loop {
            let e = self.reader.read_event_unbuffered()?;
            match e {
                Event::Start(e) => break Ok(DeEvent::Start(e)),
                Event::End(e) => break Ok(DeEvent::End(e)),
                Event::Text(e) => break Ok(DeEvent::Text(e)),
                Event::CData(e) => break Ok(DeEvent::CData(e)),
                Event::Eof => break Ok(DeEvent::Eof),

                _ => (),
            }
        }
    }

    fn read_to_end(&mut self, name: &[u8]) -> Result<(), DeError> {
        match self.reader.read_to_end_unbuffered(name) {
            Err(Error::UnexpectedEof(_)) => Err(DeError::Eof),
            other => Ok(other?),
        }
    }

    fn decoder(&self) -> Decoder {
        self.reader.decoder()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn read_to_end() {
        use crate::de::DeEvent::*;

        let mut de = Deserializer::from_slice(
            br#"
            <root>
                <tag a="1"><tag>text</tag>content</tag>
                <tag a="2"><![CDATA[cdata content]]></tag>
                <self-closed/>
            </root>
            "#,
        );

        assert_eq!(
            de.next().unwrap(),
            Start(BytesStart::borrowed_name(b"root"))
        );

        assert_eq!(
            de.next().unwrap(),
            Start(BytesStart::borrowed(br#"tag a="1""#, 3))
        );
        assert_eq!(de.read_to_end(b"tag").unwrap(), ());

        assert_eq!(
            de.next().unwrap(),
            Start(BytesStart::borrowed(br#"tag a="2""#, 3))
        );
        assert_eq!(
            de.next().unwrap(),
            CData(BytesCData::from_str("cdata content"))
        );
        assert_eq!(de.next().unwrap(), End(BytesEnd::borrowed(b"tag")));

        assert_eq!(
            de.next().unwrap(),
            Start(BytesStart::borrowed(b"self-closed", 11))
        );
        assert_eq!(de.read_to_end(b"self-closed").unwrap(), ());

        assert_eq!(de.next().unwrap(), End(BytesEnd::borrowed(b"root")));
        assert_eq!(de.next().unwrap(), Eof);
    }

    #[test]
    fn borrowing_reader_parity() {
        let s = r##"
            <item name="hello" source="world.rs">Some text</item>
            <item2/>
            <item3 value="world" />
    	"##
        .as_bytes();

        let mut reader1 = IoReader {
            reader: Reader::from_reader(s),
            buf: Vec::new(),
        };
        let mut reader2 = SliceReader {
            reader: Reader::from_bytes(s),
        };

        loop {
            let event1 = reader1.next().unwrap();
            let event2 = reader2.next().unwrap();

            if let (DeEvent::Eof, DeEvent::Eof) = (&event1, &event2) {
                break;
            }

            assert_eq!(event1, event2);
        }
    }

    #[test]
    fn borrowing_reader_events() {
        let s = r##"
            <item name="hello" source="world.rs">Some text</item>
            <item2></item2>
            <item3/>
            <item4 value="world" />
        "##
        .as_bytes();

        let mut reader = SliceReader {
            reader: Reader::from_bytes(s),
        };

        reader
            .reader
            .trim_text(true)
            .expand_empty_elements(true)
            .check_end_names(true);

        let mut events = Vec::new();

        loop {
            let event = reader.next().unwrap();
            if let DeEvent::Eof = event {
                break;
            }
            events.push(event);
        }

        use crate::de::DeEvent::*;

        assert_eq!(
            events,
            vec![
                Start(BytesStart::borrowed(
                    br#"item name="hello" source="world.rs""#,
                    4
                )),
                Text(BytesText::from_escaped(b"Some text".as_ref())),
                End(BytesEnd::borrowed(b"item")),
                Start(BytesStart::borrowed(b"item2", 5)),
                End(BytesEnd::borrowed(b"item2")),
                Start(BytesStart::borrowed(b"item3", 5)),
                End(BytesEnd::borrowed(b"item3")),
                Start(BytesStart::borrowed(br#"item4 value="world" "#, 5)),
                End(BytesEnd::borrowed(b"item4")),
            ]
        )
    }

    #[test]
    fn borrowing_read_to_end() {
        let s = " <item /> ";
        let mut reader = SliceReader {
            reader: Reader::from_str(s),
        };

        reader
            .reader
            .trim_text(true)
            .expand_empty_elements(true)
            .check_end_names(true);

        assert_eq!(
            reader.next().unwrap(),
            DeEvent::Start(BytesStart::borrowed(b"item ", 4))
        );
        reader.read_to_end(b"item").unwrap();
        assert_eq!(reader.next().unwrap(), DeEvent::Eof);
    }

    /// Ensures, that [`Deserializer::next_text()`] never can get an `End` event,
    /// because parser reports error early
    #[test]
    fn next_text() {
        match from_str::<String>(r#"</root>"#) {
            Err(DeError::Xml(Error::EndEventMismatch { expected, found })) => {
                assert_eq!(expected, "");
                assert_eq!(found, "root");
            }
            x => panic!(
                r#"Expected `Err(Xml(EndEventMismatch("", "root")))`, but found {:?}"#,
                x
            ),
        }

        let s: String = from_str(r#"<root></root>"#).unwrap();
        assert_eq!(s, "");

        match from_str::<String>(r#"<root></other>"#) {
            Err(DeError::Xml(Error::EndEventMismatch { expected, found })) => {
                assert_eq!(expected, "root");
                assert_eq!(found, "other");
            }
            x => panic!(
                r#"Expected `Err(Xml(EndEventMismatch("root", "other")))`, but found {:?}"#,
                x
            ),
        }
    }
}

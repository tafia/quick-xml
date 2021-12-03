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
//! // quick-xml = { version = "0.21", features = [ "serialize" ] }
//! extern crate serde;
//! extern crate quick_xml;
//!
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

#[cfg(test)]
mod byte_buf;
mod escape;
mod map;
mod seq;
mod var;

pub use crate::errors::serialize::DeError;
use crate::{
    errors::Error,
    events::{BytesEnd, BytesStart, BytesText, Event},
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
    CData(BytesText<'a>),
    /// End of XML document.
    Eof,
}

/// An xml deserializer
pub struct Deserializer<'de, R>
where
    R: BorrowingReader<'de>,
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

/// Deserialize an instance of type T from a string of XML text.
pub fn from_str<'de, T>(s: &'de str) -> Result<T, DeError>
where
    T: Deserialize<'de>,
{
    from_bytes(s.as_bytes())
}

/// Deserialize a xml slice of bytes
pub fn from_bytes<'de, T>(s: &'de [u8]) -> Result<T, DeError>
where
    T: Deserialize<'de>,
{
    let mut de = Deserializer::from_bytes(s);
    T::deserialize(&mut de)
}

/// Deserialize an instance of type T from bytes of XML text.
pub fn from_slice<T>(b: &[u8]) -> Result<T, DeError>
where
    T: DeserializeOwned,
{
    from_reader(b)
}

/// Deserialize from a reader
pub fn from_reader<R, T>(reader: R) -> Result<T, DeError>
where
    R: BufRead,
    T: DeserializeOwned,
{
    let mut reader = Reader::from_reader(reader);
    reader
        .expand_empty_elements(true)
        .check_end_names(true)
        .trim_text(true);
    let mut de = Deserializer::from_borrowing_reader(IoReader {
        reader,
        buf: Vec::new(),
    });
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
    R: BorrowingReader<'de>,
{
    /// Get a new deserializer
    pub fn new(reader: R) -> Self {
        Deserializer {
            reader,
            peek: None,
            has_value_field: false,
        }
    }

    /// Get a new deserializer from a regular BufRead
    pub fn from_borrowing_reader(reader: R) -> Self {
        Self::new(reader)
    }

    fn peek(&mut self) -> Result<&DeEvent<'de>, DeError> {
        if self.peek.is_none() {
            self.peek = Some(self.next()?);
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

    /// Consumes an one XML element, returns associated text or empty string:
    ///
    /// |XML                  |Result     |Comments                    |
    /// |---------------------|-----------|----------------------------|
    /// |`<tag ...>text</tag>`|`text`     |Complete tag consumed       |
    /// |`<tag/>`             |empty slice|Virtual end tag not consumed|
    /// |`</tag>`             |empty slice|Not consumed                |
    fn next_text(&mut self) -> Result<BytesText<'de>, DeError> {
        match self.next()? {
            DeEvent::Text(e) | DeEvent::CData(e) => Ok(e),
            DeEvent::Eof => Err(DeError::Eof),
            DeEvent::Start(e) => {
                // allow one nested level
                let inner = self.next()?;
                let t = match inner {
                    DeEvent::Text(t) | DeEvent::CData(t) => t,
                    DeEvent::Start(_) => return Err(DeError::Start),
                    DeEvent::End(end) if end.name() == e.name() => {
                        return Ok(BytesText::from_escaped(&[] as &[u8]));
                    }
                    DeEvent::End(_) => return Err(DeError::End),
                    DeEvent::Eof => return Err(DeError::Eof),
                };
                self.read_to_end(e.name())?;
                Ok(t)
            }
            DeEvent::End(e) => {
                self.peek = Some(DeEvent::End(e));
                Ok(BytesText::from_escaped(&[] as &[u8]))
            }
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
        Self::from_bytes(s.as_bytes())
    }

    /// Create new deserializer that will borrow data from the specified byte array
    pub fn from_bytes(bytes: &'de [u8]) -> Self {
        let mut reader = Reader::from_bytes(bytes);
        reader
            .expand_empty_elements(true)
            .check_end_names(true)
            .trim_text(true);
        Self::from_borrowing_reader(SliceReader { reader })
    }
}

macro_rules! deserialize_type {
    ($deserialize:ident => $visit:ident) => {
        fn $deserialize<V>(self, visitor: V) -> Result<V::Value, DeError>
        where
            V: Visitor<'de>,
        {
            let txt = self.next_text()?;
            // No need to unescape because valid integer representations cannot be escaped
            let string = txt.decode(self.reader.decoder())?;
            visitor.$visit(string.parse()?)
        }
    };
}

impl<'de, 'a, R> de::Deserializer<'de> for &'a mut Deserializer<'de, R>
where
    R: BorrowingReader<'de>,
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
        let txt = self.next_text()?;

        deserialize_bool(txt.as_ref(), self.reader.decoder(), visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        let text = self.next_text()?;
        let string = text.decode_and_escape(self.reader.decoder())?;
        visitor.visit_string(string.into_owned())
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
        let text = self.next_text()?;
        let string = text.decode_and_escape(self.reader.decoder())?;
        match string {
            Cow::Borrowed(string) => visitor.visit_borrowed_str(string),
            Cow::Owned(string) => visitor.visit_string(string),
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        let text = self.next_text()?;
        let value = text.escaped();
        visitor.visit_bytes(value)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        let text = self.next_text()?;
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

/// A trait that borrows an XML reader that borrows from the input. For a &[u8]
/// input the events will borrow from that input, whereas with a BufRead input
/// all events will be converted to 'static, allocating whenever necessary.
pub trait BorrowingReader<'i>
where
    Self: 'i,
{
    /// Return an input-borrowing event.
    fn next(&mut self) -> Result<DeEvent<'i>, DeError>;

    /// Skips until end element is found. Unlike `next()` it will not allocate
    /// when it cannot satisfy the lifetime.
    fn read_to_end(&mut self, name: &[u8]) -> Result<(), DeError>;

    /// A copy of the reader's decoder used to decode strings.
    fn decoder(&self) -> Decoder;
}

struct IoReader<R: BufRead> {
    reader: Reader<R>,
    buf: Vec<u8>,
}

impl<'i, R: BufRead + 'i> BorrowingReader<'i> for IoReader<R> {
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

struct SliceReader<'de> {
    reader: Reader<&'de [u8]>,
}

impl<'de> BorrowingReader<'de> for SliceReader<'de> {
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
    use crate::de::byte_buf::ByteBuf;
    use serde::de::IgnoredAny;
    use serde::Deserialize;

    /// Deserialize an instance of type T from a string of XML text.
    /// If deserialization was succeeded checks that all XML events was consumed
    fn from_str<'de, T>(s: &'de str) -> Result<T, DeError>
    where
        T: Deserialize<'de>,
    {
        let mut de = Deserializer::from_str(s);
        let result = T::deserialize(&mut de);

        // If type was deserialized, the whole XML document should be consumed
        if let Ok(_) = result {
            assert_eq!(de.next().unwrap(), DeEvent::Eof);
        }

        result
    }

    #[test]
    fn read_to_end() {
        use crate::de::DeEvent::*;

        let mut reader = Reader::from_bytes(
            r#"
            <root>
                <tag a="1"><tag>text</tag>content</tag>
                <tag a="2"><![CDATA[cdata content]]></tag>
                <self-closed/>
            </root>
            "#
            .as_bytes(),
        );
        reader
            .expand_empty_elements(true)
            .check_end_names(true)
            .trim_text(true);
        let mut de = Deserializer::from_borrowing_reader(SliceReader { reader });

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
            CData(BytesText::from_plain_str("cdata content"))
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

    #[derive(Debug, Deserialize, PartialEq)]
    struct BorrowedText<'a> {
        #[serde(rename = "$value")]
        text: &'a str,
    }

    #[test]
    fn string_borrow() {
        let s = "<text>Hello world</text>";

        let borrowed_item: BorrowedText = from_str(s).unwrap();

        assert_eq!(borrowed_item.text, "Hello world");
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct Item {
        name: String,
        source: String,
    }

    #[test]
    fn multiple_roots_attributes() {
        let s = r##"
	    <item name="hello" source="world.rs" />
	    <item name="hello" source="world.rs" />
	"##;

        let item: Vec<Item> = from_str(s).unwrap();

        assert_eq!(
            item,
            vec![
                Item {
                    name: "hello".to_string(),
                    source: "world.rs".to_string(),
                },
                Item {
                    name: "hello".to_string(),
                    source: "world.rs".to_string(),
                },
            ]
        );
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct Project {
        name: String,

        #[serde(rename = "item", default)]
        items: Vec<Item>,
    }

    #[test]
    fn nested_collection() {
        let s = r##"
	    <project name="my_project">
		<item name="hello1" source="world1.rs" />
		<item name="hello2" source="world2.rs" />
	    </project>
	"##;

        let project: Project = from_str(s).unwrap();

        assert_eq!(
            project,
            Project {
                name: "my_project".to_string(),
                items: vec![
                    Item {
                        name: "hello1".to_string(),
                        source: "world1.rs".to_string(),
                    },
                    Item {
                        name: "hello2".to_string(),
                        source: "world2.rs".to_string(),
                    },
                ],
            }
        );
    }

    #[derive(Debug, Deserialize, PartialEq)]
    enum MyEnum {
        A(String),
        B { name: String, flag: bool },
        C,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct MyEnums {
        // TODO: This should be #[serde(flatten)], but right now serde don't support flattening of sequences
        // See https://github.com/serde-rs/serde/issues/1905
        #[serde(rename = "$value")]
        items: Vec<MyEnum>,
    }

    #[test]
    fn collection_of_enums() {
        let s = r##"
        <enums>
            <A>test</A>
            <B name="hello" flag="t" />
            <C />
        </enums>
        "##;

        let project: MyEnums = from_str(s).unwrap();

        assert_eq!(
            project,
            MyEnums {
                items: vec![
                    MyEnum::A("test".to_string()),
                    MyEnum::B {
                        name: "hello".to_string(),
                        flag: true,
                    },
                    MyEnum::C,
                ],
            }
        );
    }

    #[test]
    fn deserialize_bytes() {
        let s = r#"<item>bytes</item>"#;
        let item: ByteBuf = from_reader(s.as_bytes()).unwrap();

        assert_eq!(item, ByteBuf(b"bytes".to_vec()));
    }

    /// Test for https://github.com/tafia/quick-xml/issues/231
    #[test]
    fn implicit_value() {
        use serde_value::Value;

        let s = r#"<root>content</root>"#;
        let item: Value = from_str(s).unwrap();

        assert_eq!(
            item,
            Value::Map(
                vec![(
                    Value::String("$value".into()),
                    Value::String("content".into())
                )]
                .into_iter()
                .collect()
            )
        );
    }

    #[test]
    fn explicit_value() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Item {
            #[serde(rename = "$value")]
            content: String,
        }

        let s = r#"<root>content</root>"#;
        let item: Item = from_str(s).unwrap();

        assert_eq!(
            item,
            Item {
                content: "content".into()
            }
        );
    }

    #[test]
    fn without_value() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Item;

        let s = r#"<root>content</root>"#;
        let item: Item = from_str(s).unwrap();

        assert_eq!(item, Item);
    }

    /// Tests calling `deserialize_ignored_any`
    #[test]
    fn ignored_any() {
        let err = from_str::<IgnoredAny>("");
        match err {
            Err(DeError::Eof) => {}
            other => panic!("Expected `Eof`, found {:?}", other),
        }

        from_str::<IgnoredAny>(r#"<empty/>"#).unwrap();
        from_str::<IgnoredAny>(r#"<with-attributes key="value"/>"#).unwrap();
        from_str::<IgnoredAny>(r#"<nested>text</nested>"#).unwrap();
        from_str::<IgnoredAny>(r#"<nested><![CDATA[cdata]]></nested>"#).unwrap();
        from_str::<IgnoredAny>(r#"<nested><nested/></nested>"#).unwrap();
    }

    mod unit {
        use super::*;

        #[derive(Debug, Deserialize, PartialEq)]
        struct Unit;

        #[test]
        fn simple() {
            let data: Unit = from_str("<root/>").unwrap();
            assert_eq!(data, Unit);
        }

        #[test]
        fn excess_attribute() {
            let data: Unit = from_str(r#"<root excess="attribute"/>"#).unwrap();
            assert_eq!(data, Unit);
        }

        #[test]
        fn excess_element() {
            let data: Unit = from_str(r#"<root><excess>element</excess></root>"#).unwrap();
            assert_eq!(data, Unit);
        }

        #[test]
        fn excess_text() {
            let data: Unit = from_str(r#"<root>excess text</root>"#).unwrap();
            assert_eq!(data, Unit);
        }

        #[test]
        fn excess_cdata() {
            let data: Unit = from_str(r#"<root><![CDATA[excess CDATA]]></root>"#).unwrap();
            assert_eq!(data, Unit);
        }
    }

    mod newtype {
        use super::*;

        #[derive(Debug, Deserialize, PartialEq)]
        struct Newtype(bool);

        #[test]
        fn simple() {
            let data: Newtype = from_str("<root>true</root>").unwrap();
            assert_eq!(data, Newtype(true));
        }

        #[test]
        fn excess_attribute() {
            let data: Newtype = from_str(r#"<root excess="attribute">true</root>"#).unwrap();
            assert_eq!(data, Newtype(true));
        }
    }

    mod tuple {
        use super::*;

        #[test]
        fn simple() {
            let data: (f32, String) = from_str("<root>42</root><root>answer</root>").unwrap();
            assert_eq!(data, (42.0, "answer".into()));
        }

        #[test]
        fn excess_attribute() {
            let data: (f32, String) =
                from_str(r#"<root excess="attribute">42</root><root>answer</root>"#).unwrap();
            assert_eq!(data, (42.0, "answer".into()));
        }
    }

    mod tuple_struct {
        use super::*;

        #[derive(Debug, Deserialize, PartialEq)]
        struct Tuple(f32, String);

        #[test]
        fn simple() {
            let data: Tuple = from_str("<root>42</root><root>answer</root>").unwrap();
            assert_eq!(data, Tuple(42.0, "answer".into()));
        }

        #[test]
        fn excess_attribute() {
            let data: Tuple =
                from_str(r#"<root excess="attribute">42</root><root>answer</root>"#).unwrap();
            assert_eq!(data, Tuple(42.0, "answer".into()));
        }
    }

    macro_rules! maplike_errors {
        ($type:ty) => {
            mod non_closed {
                use super::*;

                #[test]
                fn attributes() {
                    let data = from_str::<$type>(r#"<root float="42" string="answer">"#);

                    match data {
                        Err(DeError::Eof) => (),
                        _ => panic!("Expected `Eof`, found {:?}", data),
                    }
                }

                #[test]
                fn elements_root() {
                    let data = from_str::<$type>(r#"<root float="42"><string>answer</string>"#);

                    match data {
                        Err(DeError::Eof) => (),
                        _ => panic!("Expected `Eof`, found {:?}", data),
                    }
                }

                #[test]
                fn elements_child() {
                    let data = from_str::<$type>(r#"<root float="42"><string>answer"#);

                    match data {
                        Err(DeError::Eof) => (),
                        _ => panic!("Expected `Eof`, found {:?}", data),
                    }
                }
            }

            mod mismatched_end {
                use super::*;
                use crate::errors::Error::EndEventMismatch;

                #[test]
                fn attributes() {
                    let data =
                        from_str::<$type>(r#"<root float="42" string="answer"></mismatched>"#);

                    match data {
                        Err(DeError::Xml(EndEventMismatch { .. })) => (),
                        _ => panic!("Expected `Xml(EndEventMismatch)`, found {:?}", data),
                    }
                }

                #[test]
                fn elements_root() {
                    let data = from_str::<$type>(
                        r#"<root float="42"><string>answer</string></mismatched>"#,
                    );

                    match data {
                        Err(DeError::Xml(EndEventMismatch { .. })) => (),
                        _ => panic!("Expected `Xml(EndEventMismatch)`, found {:?}", data),
                    }
                }

                #[test]
                fn elements_child() {
                    let data =
                        from_str::<$type>(r#"<root float="42"><string>answer</mismatched></root>"#);

                    match data {
                        Err(DeError::Xml(EndEventMismatch { .. })) => (),
                        _ => panic!("Expected `Xml(EndEventMismatch)`, found {:?}", data),
                    }
                }
            }
        };
    }

    mod map {
        use super::*;
        use std::collections::HashMap;
        use std::iter::FromIterator;

        #[test]
        fn elements() {
            let data: HashMap<(), ()> =
                from_str(r#"<root><float>42</float><string>answer</string></root>"#).unwrap();
            assert_eq!(
                data,
                HashMap::from_iter([((), ()), ((), ()),].iter().cloned())
            );
        }

        #[test]
        fn attributes() {
            let data: HashMap<(), ()> = from_str(r#"<root float="42" string="answer"/>"#).unwrap();
            assert_eq!(
                data,
                HashMap::from_iter([((), ()), ((), ()),].iter().cloned())
            );
        }

        #[test]
        fn attribute_and_element() {
            let data: HashMap<(), ()> = from_str(
                r#"
                <root float="42">
                    <string>answer</string>
                </root>
            "#,
            )
            .unwrap();

            assert_eq!(
                data,
                HashMap::from_iter([((), ()), ((), ()),].iter().cloned())
            );
        }

        maplike_errors!(HashMap<(), ()>);
    }

    mod struct_ {
        use super::*;

        #[derive(Debug, Deserialize, PartialEq)]
        struct Struct {
            float: f64,
            string: String,
        }

        #[test]
        fn elements() {
            let data: Struct =
                from_str(r#"<root><float>42</float><string>answer</string></root>"#).unwrap();
            assert_eq!(
                data,
                Struct {
                    float: 42.0,
                    string: "answer".into()
                }
            );
        }

        #[test]
        fn excess_elements() {
            let data: Struct = from_str(
                r#"
                <root>
                    <before/>
                    <float>42</float>
                    <in-the-middle/>
                    <string>answer</string>
                    <after/>
                </root>"#,
            )
            .unwrap();
            assert_eq!(
                data,
                Struct {
                    float: 42.0,
                    string: "answer".into()
                }
            );
        }

        #[test]
        fn attributes() {
            let data: Struct = from_str(r#"<root float="42" string="answer"/>"#).unwrap();
            assert_eq!(
                data,
                Struct {
                    float: 42.0,
                    string: "answer".into()
                }
            );
        }

        #[test]
        fn excess_attributes() {
            let data: Struct = from_str(
                r#"<root before="1" float="42" in-the-middle="2" string="answer" after="3"/>"#,
            )
            .unwrap();
            assert_eq!(
                data,
                Struct {
                    float: 42.0,
                    string: "answer".into()
                }
            );
        }

        #[test]
        fn attribute_and_element() {
            let data: Struct = from_str(
                r#"
                <root float="42">
                    <string>answer</string>
                </root>
            "#,
            )
            .unwrap();

            assert_eq!(
                data,
                Struct {
                    float: 42.0,
                    string: "answer".into()
                }
            );
        }

        maplike_errors!(Struct);
    }

    mod nested_struct {
        use super::*;

        #[derive(Debug, Deserialize, PartialEq)]
        struct Struct {
            nested: Nested,
            string: String,
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct Nested {
            float: f32,
        }

        #[test]
        fn elements() {
            let data: Struct = from_str(
                r#"<root><string>answer</string><nested><float>42</float></nested></root>"#,
            )
            .unwrap();
            assert_eq!(
                data,
                Struct {
                    nested: Nested { float: 42.0 },
                    string: "answer".into()
                }
            );
        }

        #[test]
        fn attributes() {
            let data: Struct =
                from_str(r#"<root string="answer"><nested float="42"/></root>"#).unwrap();
            assert_eq!(
                data,
                Struct {
                    nested: Nested { float: 42.0 },
                    string: "answer".into()
                }
            );
        }
    }

    mod flatten_struct {
        use super::*;

        #[derive(Debug, Deserialize, PartialEq)]
        struct Struct {
            #[serde(flatten)]
            nested: Nested,
            string: String,
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct Nested {
            //TODO: change to f64 after fixing https://github.com/serde-rs/serde/issues/1183
            float: String,
        }

        #[test]
        #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
        fn elements() {
            let data: Struct =
                from_str(r#"<root><float>42</float><string>answer</string></root>"#).unwrap();
            assert_eq!(
                data,
                Struct {
                    nested: Nested { float: "42".into() },
                    string: "answer".into()
                }
            );
        }

        #[test]
        fn attributes() {
            let data: Struct = from_str(r#"<root float="42" string="answer"/>"#).unwrap();
            assert_eq!(
                data,
                Struct {
                    nested: Nested { float: "42".into() },
                    string: "answer".into()
                }
            );
        }
    }

    mod enum_ {
        use super::*;

        mod externally_tagged {
            use super::*;

            #[derive(Debug, Deserialize, PartialEq)]
            enum Node {
                Unit,
                Newtype(bool),
                //TODO: serde bug https://github.com/serde-rs/serde/issues/1904
                // Tuple(f64, String),
                Struct {
                    float: f64,
                    string: String,
                },
                Holder {
                    nested: Nested,
                    string: String,
                },
                Flatten {
                    #[serde(flatten)]
                    nested: Nested,
                    string: String,
                },
            }

            #[derive(Debug, Deserialize, PartialEq)]
            struct Nested {
                //TODO: change to f64 after fixing https://github.com/serde-rs/serde/issues/1183
                float: String,
            }

            /// Workaround for serde bug https://github.com/serde-rs/serde/issues/1904
            #[derive(Debug, Deserialize, PartialEq)]
            enum Workaround {
                Tuple(f64, String),
            }

            #[test]
            fn unit() {
                let data: Node = from_str("<Unit/>").unwrap();
                assert_eq!(data, Node::Unit);
            }

            #[test]
            fn newtype() {
                let data: Node = from_str("<Newtype>true</Newtype>").unwrap();
                assert_eq!(data, Node::Newtype(true));
            }

            #[test]
            fn tuple_struct() {
                let data: Workaround = from_str("<Tuple>42</Tuple><Tuple>answer</Tuple>").unwrap();
                assert_eq!(data, Workaround::Tuple(42.0, "answer".into()));
            }

            mod struct_ {
                use super::*;

                #[test]
                fn elements() {
                    let data: Node =
                        from_str(r#"<Struct><float>42</float><string>answer</string></Struct>"#)
                            .unwrap();
                    assert_eq!(
                        data,
                        Node::Struct {
                            float: 42.0,
                            string: "answer".into()
                        }
                    );
                }

                #[test]
                fn attributes() {
                    let data: Node = from_str(r#"<Struct float="42" string="answer"/>"#).unwrap();
                    assert_eq!(
                        data,
                        Node::Struct {
                            float: 42.0,
                            string: "answer".into()
                        }
                    );
                }
            }

            mod nested_struct {
                use super::*;

                #[test]
                fn elements() {
                    let data: Node = from_str(
                        r#"<Holder><string>answer</string><nested><float>42</float></nested></Holder>"#
                    ).unwrap();
                    assert_eq!(
                        data,
                        Node::Holder {
                            nested: Nested { float: "42".into() },
                            string: "answer".into()
                        }
                    );
                }

                #[test]
                fn attributes() {
                    let data: Node =
                        from_str(r#"<Holder string="answer"><nested float="42"/></Holder>"#)
                            .unwrap();
                    assert_eq!(
                        data,
                        Node::Holder {
                            nested: Nested { float: "42".into() },
                            string: "answer".into()
                        }
                    );
                }
            }

            mod flatten_struct {
                use super::*;

                #[test]
                #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
                fn elements() {
                    let data: Node =
                        from_str(r#"<Flatten><float>42</float><string>answer</string></Flatten>"#)
                            .unwrap();
                    assert_eq!(
                        data,
                        Node::Flatten {
                            nested: Nested { float: "42".into() },
                            string: "answer".into()
                        }
                    );
                }

                #[test]
                fn attributes() {
                    let data: Node = from_str(r#"<Flatten float="42" string="answer"/>"#).unwrap();
                    assert_eq!(
                        data,
                        Node::Flatten {
                            nested: Nested { float: "42".into() },
                            string: "answer".into()
                        }
                    );
                }
            }
        }

        mod internally_tagged {
            use super::*;

            #[derive(Debug, Deserialize, PartialEq)]
            #[serde(tag = "tag")]
            enum Node {
                Unit,
                /// Primitives (such as `bool`) are not supported by serde in the internally tagged mode
                Newtype(NewtypeContent),
                // Tuple(f64, String),// Tuples are not supported in the internally tagged mode
                //TODO: change to f64 after fixing https://github.com/serde-rs/serde/issues/1183
                Struct {
                    float: String,
                    string: String,
                },
                Holder {
                    nested: Nested,
                    string: String,
                },
                Flatten {
                    #[serde(flatten)]
                    nested: Nested,
                    string: String,
                },
            }

            #[derive(Debug, Deserialize, PartialEq)]
            struct NewtypeContent {
                value: bool,
            }

            #[derive(Debug, Deserialize, PartialEq)]
            struct Nested {
                //TODO: change to f64 after fixing https://github.com/serde-rs/serde/issues/1183
                float: String,
            }

            mod unit {
                use super::*;

                #[test]
                fn elements() {
                    let data: Node = from_str(r#"<root><tag>Unit</tag></root>"#).unwrap();
                    assert_eq!(data, Node::Unit);
                }

                #[test]
                fn attributes() {
                    let data: Node = from_str(r#"<root tag="Unit"/>"#).unwrap();
                    assert_eq!(data, Node::Unit);
                }
            }

            mod newtype {
                use super::*;

                #[test]
                #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
                fn elements() {
                    let data: Node =
                        from_str(r#"<root><tag>Newtype</tag><value>true</value></root>"#).unwrap();
                    assert_eq!(data, Node::Newtype(NewtypeContent { value: true }));
                }

                #[test]
                #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
                fn attributes() {
                    let data: Node = from_str(r#"<root tag="Newtype" value="true"/>"#).unwrap();
                    assert_eq!(data, Node::Newtype(NewtypeContent { value: true }));
                }
            }

            mod struct_ {
                use super::*;

                #[test]
                #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
                fn elements() {
                    let data: Node = from_str(
                        r#"<root><tag>Struct</tag><float>42</float><string>answer</string></root>"#,
                    )
                    .unwrap();
                    assert_eq!(
                        data,
                        Node::Struct {
                            float: "42".into(),
                            string: "answer".into()
                        }
                    );
                }

                #[test]
                fn attributes() {
                    let data: Node =
                        from_str(r#"<root tag="Struct" float="42" string="answer"/>"#).unwrap();
                    assert_eq!(
                        data,
                        Node::Struct {
                            float: "42".into(),
                            string: "answer".into()
                        }
                    );
                }
            }

            mod nested_struct {
                use super::*;

                #[test]
                #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
                fn elements() {
                    let data: Node = from_str(
                        r#"<root><tag>Holder</tag><string>answer</string><nested><float>42</float></nested></root>"#
                    ).unwrap();
                    assert_eq!(
                        data,
                        Node::Holder {
                            nested: Nested { float: "42".into() },
                            string: "answer".into()
                        }
                    );
                }

                #[test]
                fn attributes() {
                    let data: Node = from_str(
                        r#"<root tag="Holder" string="answer"><nested float="42"/></root>"#,
                    )
                    .unwrap();
                    assert_eq!(
                        data,
                        Node::Holder {
                            nested: Nested { float: "42".into() },
                            string: "answer".into()
                        }
                    );
                }
            }

            mod flatten_struct {
                use super::*;

                #[test]
                #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
                fn elements() {
                    let data: Node = from_str(
                        r#"<root><tag>Flatten</tag><float>42</float><string>answer</string></root>"#
                    ).unwrap();
                    assert_eq!(
                        data,
                        Node::Flatten {
                            nested: Nested { float: "42".into() },
                            string: "answer".into()
                        }
                    );
                }

                #[test]
                fn attributes() {
                    let data: Node =
                        from_str(r#"<root tag="Flatten" float="42" string="answer"/>"#).unwrap();
                    assert_eq!(
                        data,
                        Node::Flatten {
                            nested: Nested { float: "42".into() },
                            string: "answer".into()
                        }
                    );
                }
            }
        }

        mod adjacently_tagged {
            use super::*;

            #[derive(Debug, Deserialize, PartialEq)]
            #[serde(tag = "tag", content = "content")]
            enum Node {
                Unit,
                Newtype(bool),
                //TODO: serde bug https://github.com/serde-rs/serde/issues/1904
                // Tuple(f64, String),
                Struct {
                    float: f64,
                    string: String,
                },
                Holder {
                    nested: Nested,
                    string: String,
                },
                Flatten {
                    #[serde(flatten)]
                    nested: Nested,
                    string: String,
                },
            }

            #[derive(Debug, Deserialize, PartialEq)]
            struct Nested {
                //TODO: change to f64 after fixing https://github.com/serde-rs/serde/issues/1183
                float: String,
            }

            /// Workaround for serde bug https://github.com/serde-rs/serde/issues/1904
            #[derive(Debug, Deserialize, PartialEq)]
            #[serde(tag = "tag", content = "content")]
            enum Workaround {
                Tuple(f64, String),
            }

            mod unit {
                use super::*;

                #[test]
                fn elements() {
                    let data: Node = from_str(r#"<root><tag>Unit</tag></root>"#).unwrap();
                    assert_eq!(data, Node::Unit);
                }

                #[test]
                fn attributes() {
                    let data: Node = from_str(r#"<root tag="Unit"/>"#).unwrap();
                    assert_eq!(data, Node::Unit);
                }
            }

            mod newtype {
                use super::*;

                #[test]
                fn elements() {
                    let data: Node =
                        from_str(r#"<root><tag>Newtype</tag><content>true</content></root>"#)
                            .unwrap();
                    assert_eq!(data, Node::Newtype(true));
                }

                #[test]
                fn attributes() {
                    let data: Node = from_str(r#"<root tag="Newtype" content="true"/>"#).unwrap();
                    assert_eq!(data, Node::Newtype(true));
                }
            }

            mod tuple_struct {
                use super::*;
                #[test]
                fn elements() {
                    let data: Workaround = from_str(
                        r#"<root><tag>Tuple</tag><content>42</content><content>answer</content></root>"#
                    ).unwrap();
                    assert_eq!(data, Workaround::Tuple(42.0, "answer".into()));
                }

                #[test]
                #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
                fn attributes() {
                    let data: Workaround = from_str(
                        r#"<root tag="Tuple" content="42"><content>answer</content></root>"#,
                    )
                    .unwrap();
                    assert_eq!(data, Workaround::Tuple(42.0, "answer".into()));
                }
            }

            mod struct_ {
                use super::*;

                #[test]
                fn elements() {
                    let data: Node = from_str(
                        r#"<root><tag>Struct</tag><content><float>42</float><string>answer</string></content></root>"#
                    ).unwrap();
                    assert_eq!(
                        data,
                        Node::Struct {
                            float: 42.0,
                            string: "answer".into()
                        }
                    );
                }

                #[test]
                fn attributes() {
                    let data: Node = from_str(
                        r#"<root tag="Struct"><content float="42" string="answer"/></root>"#,
                    )
                    .unwrap();
                    assert_eq!(
                        data,
                        Node::Struct {
                            float: 42.0,
                            string: "answer".into()
                        }
                    );
                }
            }

            mod nested_struct {
                use super::*;

                #[test]
                fn elements() {
                    let data: Node = from_str(
                        r#"<root>
                            <tag>Holder</tag>
                            <content>
                                <string>answer</string>
                                <nested>
                                    <float>42</float>
                                </nested>
                            </content>
                        </root>"#,
                    )
                    .unwrap();
                    assert_eq!(
                        data,
                        Node::Holder {
                            nested: Nested { float: "42".into() },
                            string: "answer".into()
                        }
                    );
                }

                #[test]
                fn attributes() {
                    let data: Node = from_str(
                        r#"<root tag="Holder"><content string="answer"><nested float="42"/></content></root>"#
                    ).unwrap();
                    assert_eq!(
                        data,
                        Node::Holder {
                            nested: Nested { float: "42".into() },
                            string: "answer".into()
                        }
                    );
                }
            }

            mod flatten_struct {
                use super::*;

                #[test]
                #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
                fn elements() {
                    let data: Node = from_str(
                        r#"<root><tag>Flatten</tag><content><float>42</float><string>answer</string></content></root>"#
                    ).unwrap();
                    assert_eq!(
                        data,
                        Node::Flatten {
                            nested: Nested { float: "42".into() },
                            string: "answer".into()
                        }
                    );
                }

                #[test]
                fn attributes() {
                    let data: Node = from_str(
                        r#"<root tag="Flatten"><content float="42" string="answer"/></root>"#,
                    )
                    .unwrap();
                    assert_eq!(
                        data,
                        Node::Flatten {
                            nested: Nested { float: "42".into() },
                            string: "answer".into()
                        }
                    );
                }
            }
        }

        mod untagged {
            use super::*;

            #[derive(Debug, Deserialize, PartialEq)]
            #[serde(untagged)]
            enum Node {
                Unit,
                Newtype(bool),
                // serde bug https://github.com/serde-rs/serde/issues/1904
                // Tuple(f64, String),
                Struct {
                    float: f64,
                    string: String,
                },
                Holder {
                    nested: Nested,
                    string: String,
                },
                Flatten {
                    #[serde(flatten)]
                    nested: Nested,
                    // Can't use "string" as name because in that case this variant
                    // will have no difference from `Struct` variant
                    string2: String,
                },
            }

            #[derive(Debug, Deserialize, PartialEq)]
            struct Nested {
                //TODO: change to f64 after fixing https://github.com/serde-rs/serde/issues/1183
                float: String,
            }

            /// Workaround for serde bug https://github.com/serde-rs/serde/issues/1904
            #[derive(Debug, Deserialize, PartialEq)]
            #[serde(untagged)]
            enum Workaround {
                Tuple(f64, String),
            }

            #[test]
            #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
            fn unit() {
                // Unit variant consists just from the tag, and because tags
                // are not written, nothing is written
                let data: Node = from_str("").unwrap();
                assert_eq!(data, Node::Unit);
            }

            #[test]
            #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
            fn newtype() {
                let data: Node = from_str("true").unwrap();
                assert_eq!(data, Node::Newtype(true));
            }

            #[test]
            #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
            fn tuple_struct() {
                let data: Workaround = from_str("<root>42</root><root>answer</root>").unwrap();
                assert_eq!(data, Workaround::Tuple(42.0, "answer".into()));
            }

            mod struct_ {
                use super::*;

                #[test]
                #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
                fn elements() {
                    let data: Node =
                        from_str(r#"<root><float>42</float><string>answer</string></root>"#)
                            .unwrap();
                    assert_eq!(
                        data,
                        Node::Struct {
                            float: 42.0,
                            string: "answer".into()
                        }
                    );
                }

                #[test]
                #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
                fn attributes() {
                    let data: Node = from_str(r#"<root float="42" string="answer"/>"#).unwrap();
                    assert_eq!(
                        data,
                        Node::Struct {
                            float: 42.0,
                            string: "answer".into()
                        }
                    );
                }
            }

            mod nested_struct {
                use super::*;

                #[test]
                #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
                fn elements() {
                    let data: Node = from_str(
                        r#"<root><string>answer</string><nested><float>42</float></nested></root>"#,
                    )
                    .unwrap();
                    assert_eq!(
                        data,
                        Node::Holder {
                            nested: Nested { float: "42".into() },
                            string: "answer".into()
                        }
                    );
                }

                #[test]
                fn attributes() {
                    let data: Node =
                        from_str(r#"<root string="answer"><nested float="42"/></root>"#).unwrap();
                    assert_eq!(
                        data,
                        Node::Holder {
                            nested: Nested { float: "42".into() },
                            string: "answer".into()
                        }
                    );
                }
            }

            mod flatten_struct {
                use super::*;

                #[test]
                #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
                fn elements() {
                    let data: Node =
                        from_str(r#"<root><float>42</float><string2>answer</string2></root>"#)
                            .unwrap();
                    assert_eq!(
                        data,
                        Node::Flatten {
                            nested: Nested { float: "42".into() },
                            string2: "answer".into()
                        }
                    );
                }

                #[test]
                fn attributes() {
                    let data: Node = from_str(r#"<root float="42" string2="answer"/>"#).unwrap();
                    assert_eq!(
                        data,
                        Node::Flatten {
                            nested: Nested { float: "42".into() },
                            string2: "answer".into()
                        }
                    );
                }
            }
        }
    }
}

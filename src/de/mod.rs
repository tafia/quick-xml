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

// Macros should be defined before the modules that using them
// Also, macros should be imported before using them
use serde::serde_if_integer128;

macro_rules! deserialize_type {
    ($deserialize:ident => $visit:ident, $($mut:tt)?) => {
        fn $deserialize<V>($($mut)? self, visitor: V) -> Result<V::Value, DeError>
        where
            V: Visitor<'de>,
        {
            let text = self.next_text()?;
            // No need to unescape because valid integer representations cannot be escaped
            let string = text.decode(self.decoder())?;
            visitor.$visit(string.parse()?)
        }
    };
}

/// Implement deserialization methods for scalar types, such as numbers, strings,
/// byte arrays, boolean and identifier.
macro_rules! deserialize_primitives {
    ($($mut:tt)?) => {
        deserialize_type!(deserialize_i8 => visit_i8, $($mut)?);
        deserialize_type!(deserialize_i16 => visit_i16, $($mut)?);
        deserialize_type!(deserialize_i32 => visit_i32, $($mut)?);
        deserialize_type!(deserialize_i64 => visit_i64, $($mut)?);

        deserialize_type!(deserialize_u8 => visit_u8, $($mut)?);
        deserialize_type!(deserialize_u16 => visit_u16, $($mut)?);
        deserialize_type!(deserialize_u32 => visit_u32, $($mut)?);
        deserialize_type!(deserialize_u64 => visit_u64, $($mut)?);

        serde_if_integer128! {
            deserialize_type!(deserialize_i128 => visit_i128, $($mut)?);
            deserialize_type!(deserialize_u128 => visit_u128, $($mut)?);
        }

        deserialize_type!(deserialize_f32 => visit_f32, $($mut)?);
        deserialize_type!(deserialize_f64 => visit_f64, $($mut)?);

        fn deserialize_bool<V>($($mut)? self, visitor: V) -> Result<V::Value, DeError>
        where
            V: Visitor<'de>,
        {
            let text = self.next_text()?;

            deserialize_bool(text.as_ref(), self.decoder(), visitor)
        }

        /// Representation of owned strings the same as [non-owned](#method.deserialize_str).
        fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, DeError>
        where
            V: Visitor<'de>,
        {
            self.deserialize_str(visitor)
        }

        /// Character represented as [strings](#method.deserialize_str).
        fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, DeError>
        where
            V: Visitor<'de>,
        {
            self.deserialize_string(visitor)
        }

        fn deserialize_str<V>($($mut)? self, visitor: V) -> Result<V::Value, DeError>
        where
            V: Visitor<'de>,
        {
            let text = self.next_text()?;
            let string = text.decode(self.decoder())?;
            match string {
                Cow::Borrowed(string) => visitor.visit_borrowed_str(string),
                Cow::Owned(string) => visitor.visit_string(string),
            }
        }

        fn deserialize_bytes<V>($($mut)? self, visitor: V) -> Result<V::Value, DeError>
        where
            V: Visitor<'de>,
        {
            let text = self.next_text()?;
            visitor.visit_bytes(&text)
        }

        fn deserialize_byte_buf<V>($($mut)? self, visitor: V) -> Result<V::Value, DeError>
        where
            V: Visitor<'de>,
        {
            let text = self.next_text()?;
            let value = text.into_inner().into_owned();
            visitor.visit_byte_buf(value)
        }

        /// Identifiers represented as [strings](#method.deserialize_str).
        fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, DeError>
        where
            V: Visitor<'de>,
        {
            self.deserialize_string(visitor)
        }
    };
}

#[cfg(test)]
mod byte_buf;
mod escape;
mod map;
mod seq;
mod simple_type;
mod var;

pub use crate::errors::serialize::DeError;
use crate::{
    errors::Error,
    events::{BytesCData, BytesEnd, BytesStart, BytesText, Event},
    reader::Decoder,
    Reader,
};
use serde::de::{self, Deserialize, DeserializeOwned, Visitor};
use std::borrow::Cow;
use std::collections::VecDeque;
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

//-------------------------------------------------------------------------------------------------

/// A structure that deserializes XML into Rust values.
pub struct Deserializer<'de, R>
where
    R: BorrowingReader<'de>,
{
    /// An XML reader that streams events into deserializer
    reader: R,
    /// When deserializing sequences sometimes we have to skip unwanted events.
    /// That events should be stored and then replayed. This is a replay buffer,
    /// that streams events while not empty. When it exhausted, events will
    /// requested from `Self::reader`.
    read: VecDeque<DeEvent<'de>>,
    /// When deserializing sequences sometimes we have to skip events, because XML
    /// is tolerant to elements order and even if in the XSD order is strictly
    /// specified (using `xs:sequence`) most of XML parsers allow order violations.
    /// That means, that elements, forming a sequence, could be overlapped with
    /// other elements, do not related to that sequence.
    ///
    /// In order to support this, deserializer will scan events and skip unwanted
    /// events, store them here. After call [`Self::start_replay()`] all events
    /// moved from this to [`Self::read`].
    write: VecDeque<DeEvent<'de>>,
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
    let mut de = Deserializer::new(IoReader {
        reader,
        buf: Vec::new(),
    });
    T::deserialize(&mut de)
}

// TODO: According to the https://www.w3.org/TR/xmlschema-2/#boolean,
// valid boolean representations are only "true", "false", "1", and "0"
fn str2bool<'de, V>(value: &str, visitor: V) -> Result<V::Value, DeError>
where
    V: de::Visitor<'de>,
{
    match value {
        "true" | "1" | "True" | "TRUE" | "t" | "Yes" | "YES" | "yes" | "y" => {
            visitor.visit_bool(true)
        }
        "false" | "0" | "False" | "FALSE" | "f" | "No" | "NO" | "no" | "n" => {
            visitor.visit_bool(false)
        }
        _ => Err(DeError::InvalidBoolean(value.into())),
    }
}

fn deserialize_bool<'de, V>(value: &[u8], decoder: Decoder, visitor: V) -> Result<V::Value, DeError>
where
    V: Visitor<'de>,
{
    #[cfg(feature = "encoding")]
    {
        let value = decoder.decode(value);
        // No need to unescape because valid boolean representations cannot be escaped
        str2bool(value.as_ref(), visitor)
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
            read: VecDeque::new(),
            write: VecDeque::new(),
        }
    }

    fn peek(&mut self) -> Result<&DeEvent<'de>, DeError> {
        if self.read.is_empty() {
            self.read.push_front(self.reader.next()?);
        }
        if let Some(event) = self.read.front() {
            return Ok(&event);
        }
        // SAFETY: `self.read` was filled in the code above.
        // NOTE: Can be replaced with `unsafe { std::hint::unreachable_unchecked() }`
        // if unsafe code will be allowed
        unreachable!()
    }

    fn next(&mut self) -> Result<DeEvent<'de>, DeError> {
        // Replay skipped or peeked events
        if let Some(event) = self.read.pop_front() {
            return Ok(event);
        }
        self.reader.next()
    }

    /// Extracts XML tree of events from and stores them in the skipped events
    /// buffer from which they can be retrieved later. You MUST call
    /// [`Self::start_replay()`] after calling this to give acces to the skipped
    /// events and release internal buffers.
    fn skip(&mut self) -> Result<(), DeError> {
        let event = self.next()?;
        self.write.push_back(event);
        match self.write.back() {
            // Skip all subtree, if we skip a start event
            Some(DeEvent::Start(e)) => {
                let end = e.name().to_owned();
                let mut depth = 0;
                loop {
                    let event = self.next()?;
                    match event {
                        DeEvent::Start(ref e) if e.name() == end => {
                            self.write.push_back(event);
                            depth += 1;
                        }
                        DeEvent::End(ref e) if e.name() == end => {
                            self.write.push_back(event);
                            if depth == 0 {
                                return Ok(());
                            }
                            depth -= 1;
                        }
                        _ => self.write.push_back(event),
                    }
                }
            }
            _ => Ok(()),
        }
    }

    /// Moves all buffered events to the end of [`Self::write`] buffer and swaps
    /// read and write buffers.
    ///
    /// After calling this method, [`Self::peek()`] and [`Self::next()`] starts
    /// return events that was skipped previously by calling [`Self::skip()`],
    /// and only when all that events will be consumed, the deserializer starts
    /// to drain events from underlying deserializer.
    ///
    /// This method MUST be called if any number of [`Self::skip()`] was called
    /// after [`Self::new()`] or `start_replay()`.
    fn start_replay(&mut self) {
        self.write.append(&mut self.read);
        std::mem::swap(&mut self.read, &mut self.write);
    }

    fn next_start(&mut self) -> Result<Option<BytesStart<'de>>, DeError> {
        loop {
            let e = self.next()?;
            match e {
                DeEvent::Start(e) => return Ok(Some(e)),
                DeEvent::End(e) => return Err(DeError::UnexpectedEnd(e.name().to_owned())),
                DeEvent::Eof => return Ok(None),
                _ => (), // ignore texts
            }
        }
    }

    #[inline]
    fn next_text(&mut self) -> Result<BytesCData<'de>, DeError> {
        self.next_text_impl(true)
    }

    /// Consumes a one XML element or an XML tree, returns associated text or
    /// an empty string.
    ///
    /// If `allow_start` is `false`, then only one event is consumed. If that
    /// event is [`DeEvent::Start`], then [`DeError::UnexpectedStart`] is returned.
    ///
    /// If `allow_start` is `true`, then first text of CDATA event inside it is
    /// returned and all other content is skipped until corresponding end tag
    /// will be consumed.
    ///
    /// # Handling events
    ///
    /// The table below shows how events is handled by this method:
    ///
    /// |Event             |XML                        |Handling
    /// |------------------|---------------------------|----------------------------------------
    /// |[`DeEvent::Start`]|`<tag>...</tag>`           |if `allow_start == true`, result determined by the second table, otherwise emits [`UnexpectedStart("tag")`](DeError::UnexpectedStart)
    /// |[`DeEvent::End`]  |`</any-tag>`               |Emits [`UnexpectedEnd("any-tag")`](DeError::UnexpectedEnd)
    /// |[`DeEvent::Text`] |`text content`             |Unescapes `text content` and returns it
    /// |[`DeEvent::CData`]|`<![CDATA[cdata content]]>`|Returns `cdata content` unchanged
    /// |[`DeEvent::Eof`]  |                           |Emits [`UnexpectedEof`](DeError::UnexpectedEof)
    ///
    /// Second event, consumed if [`DeEvent::Start`] was received:
    ///
    /// |Event             |XML                        |Handling
    /// |------------------|---------------------------|----------------------------------------------------------------------------------
    /// |[`DeEvent::Start`]|`<any-tag>...</any-tag>`   |Emits [`UnexpectedStart("any-tag")`](DeError::UnexpectedStart)
    /// |[`DeEvent::End`]  |`</tag>`                   |Returns an empty slice, if close tag matched the open one
    /// |[`DeEvent::End`]  |`</any-tag>`               |Emits [`UnexpectedEnd("any-tag")`](DeError::UnexpectedEnd)
    /// |[`DeEvent::Text`] |`text content`             |Unescapes `text content` and returns it, consumes events up to `</tag>`
    /// |[`DeEvent::CData`]|`<![CDATA[cdata content]]>`|Returns `cdata content` unchanged, consumes events up to `</tag>`
    /// |[`DeEvent::Eof`]  |                           |Emits [`UnexpectedEof`](DeError::UnexpectedEof)
    fn next_text_impl(&mut self, allow_start: bool) -> Result<BytesCData<'de>, DeError> {
        match self.next()? {
            DeEvent::Text(e) => Ok(e.unescape()?),
            DeEvent::CData(e) => Ok(e),
            DeEvent::Start(e) if allow_start => {
                // allow one nested level
                let inner = self.next()?;
                let t = match inner {
                    DeEvent::Text(t) => t.unescape()?,
                    DeEvent::CData(t) => t,
                    DeEvent::Start(s) => return Err(DeError::UnexpectedStart(s.name().to_owned())),
                    // We can get End event in case of `<tag></tag>` or `<tag/>` input
                    // Return empty text in that case
                    DeEvent::End(end) if end.name() == e.name() => {
                        return Ok(BytesCData::new(&[] as &[u8]));
                    }
                    DeEvent::End(end) => return Err(DeError::UnexpectedEnd(end.name().to_owned())),
                    DeEvent::Eof => return Err(DeError::UnexpectedEof),
                };
                self.read_to_end(e.name())?;
                Ok(t)
            }
            DeEvent::Start(e) => Err(DeError::UnexpectedStart(e.name().to_owned())),
            DeEvent::End(e) => Err(DeError::UnexpectedEnd(e.name().to_owned())),
            DeEvent::Eof => Err(DeError::UnexpectedEof),
        }
    }

    /// Returns a decoder, used inside `deserialize_primitives!()`
    #[inline]
    fn decoder(&self) -> Decoder {
        self.reader.decoder()
    }

    /// Drops all events until event with [name](BytesEnd::name()) `name` won't be
    /// dropped. This method should be called after `Self::next()`
    fn read_to_end(&mut self, name: &[u8]) -> Result<(), DeError> {
        let mut depth = 0;
        loop {
            match self.read.pop_front() {
                Some(DeEvent::Start(e)) if e.name() == name => {
                    depth += 1;
                }
                Some(DeEvent::End(e)) if e.name() == name => {
                    if depth == 0 {
                        return Ok(());
                    }
                    depth -= 1;
                }

                // Drop all other skipped events
                Some(_) => continue,

                // If we do not have skipped events, use effective reading that will
                // not allocate memory for events
                None => return self.reader.read_to_end(name),
            }
        }
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
        Self::new(SliceReader { reader })
    }
}

impl<'de, 'a, R> de::Deserializer<'de> for &'a mut Deserializer<'de, R>
where
    R: BorrowingReader<'de>,
{
    type Error = DeError;

    deserialize_primitives!();

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
            let map = map::MapAccess::new(self, e, fields)?;
            let value = visitor.visit_map(map)?;
            self.read_to_end(&name)?;
            Ok(value)
        } else {
            Err(DeError::ExpectedStart)
        }
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
    /// |[`DeEvent::End`]  |`</tag>`                   |Emits [`UnexpectedEnd("tag")`](DeError::UnexpectedEnd)
    /// |[`DeEvent::Text`] |`text content`             |Calls `visitor.visit_unit()`. Text content is ignored
    /// |[`DeEvent::CData`]|`<![CDATA[cdata content]]>`|Calls `visitor.visit_unit()`. CDATA content is ignored
    /// |[`DeEvent::Eof`]  |                           |Emits [`UnexpectedEof`](DeError::UnexpectedEof)
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
            DeEvent::End(e) => Err(DeError::UnexpectedEnd(e.name().to_owned())),
            DeEvent::Eof => Err(DeError::UnexpectedEof),
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
        let seq = visitor.visit_seq(seq::TopLevelSeqAccess::new(self)?);
        self.start_replay();
        seq
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
            DeEvent::End(e) => return Err(DeError::UnexpectedEnd(e.name().to_owned())),
            DeEvent::Eof => return Err(DeError::UnexpectedEof),
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
}

/// A trait that borrows an XML reader that borrows from the input. For a &[u8]
/// input the events will borrow from that input, whereas with a BufRead input
/// all events will be converted to 'static, allocating whenever necessary.
pub trait BorrowingReader<'i> {
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

impl<'i, R: BufRead> BorrowingReader<'i> for IoReader<R> {
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
            Err(Error::UnexpectedEof(_)) => Err(DeError::UnexpectedEof),
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
            Err(Error::UnexpectedEof(_)) => Err(DeError::UnexpectedEof),
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
    use pretty_assertions::assert_eq;
    use serde::de::IgnoredAny;
    use serde::Deserialize;

    /// Deserialize an instance of type T from a string of XML text.
    /// If deserialization was succeeded checks that all XML events was consumed
    fn from_str<'de, T>(s: &'de str) -> Result<T, DeError>
    where
        T: Deserialize<'de>,
    {
        // Log XM that we try to deserialize to see it in the failed tests output
        dbg!(s);
        let mut de = Deserializer::from_str(s);
        let result = T::deserialize(&mut de);

        // If type was deserialized, the whole XML document should be consumed
        if let Ok(_) = result {
            assert_eq!(de.next().unwrap(), DeEvent::Eof);
        }

        result
    }

    mod skip {
        use super::*;
        use pretty_assertions::assert_eq;

        /// Checks that `peek()` and `read()` behaves correctly after `skip()`
        #[test]
        fn read_and_peek() {
            use crate::de::DeEvent::*;
            use crate::events::{BytesEnd, BytesText};
            use pretty_assertions::assert_eq;

            let mut de = Deserializer::from_bytes(
                br#"
                <root>
                    <inner>
                        text
                        <inner/>
                    </inner>
                    <next/>
                    <target/>
                </root>
                "#,
            );

            // Initial conditions - both are empty
            assert_eq!(de.read, vec![]);
            assert_eq!(de.write, vec![]);

            assert_eq!(
                de.next().unwrap(),
                Start(BytesStart::borrowed_name(b"root"))
            );
            assert_eq!(
                de.peek().unwrap(),
                &Start(BytesStart::borrowed_name(b"inner"))
            );

            // Should skip first <inner> tree
            de.skip().unwrap();
            assert_eq!(de.read, vec![]);
            assert_eq!(
                de.write,
                vec![
                    Start(BytesStart::borrowed_name(b"inner")),
                    Text(BytesText::from_escaped_str("text")),
                    Start(BytesStart::borrowed_name(b"inner")),
                    End(BytesEnd::borrowed(b"inner")),
                    End(BytesEnd::borrowed(b"inner")),
                ]
            );

            // Consume <next/>. Now unconsumed XML looks like:
            //
            //   <inner>
            //     text
            //     <inner/>
            //   </inner>
            //   <target/>
            // </root>
            assert_eq!(
                de.next().unwrap(),
                Start(BytesStart::borrowed_name(b"next"))
            );
            assert_eq!(de.next().unwrap(), End(BytesEnd::borrowed(b"next")));

            // We finish writing. Next call to `next()` should start replay that messages:
            //
            //   <inner>
            //     text
            //     <inner/>
            //   </inner>
            //
            // and after that stream that messages:
            //
            //   <target/>
            // </root>
            de.start_replay();
            assert_eq!(
                de.read,
                vec![
                    Start(BytesStart::borrowed_name(b"inner")),
                    Text(BytesText::from_escaped_str("text")),
                    Start(BytesStart::borrowed_name(b"inner")),
                    End(BytesEnd::borrowed(b"inner")),
                    End(BytesEnd::borrowed(b"inner")),
                ]
            );
            assert_eq!(de.write, vec![]);
            assert_eq!(
                de.next().unwrap(),
                Start(BytesStart::borrowed_name(b"inner"))
            );

            // Skip `#text` node and consume <inner/> after it
            de.skip().unwrap();
            assert_eq!(
                de.read,
                vec![
                    Start(BytesStart::borrowed_name(b"inner")),
                    End(BytesEnd::borrowed(b"inner")),
                    End(BytesEnd::borrowed(b"inner")),
                ]
            );
            assert_eq!(
                de.write,
                vec![
                    // This comment here to keep the same formatting of both arrays
                    // otherwise rustfmt suggest one-line it
                    Text(BytesText::from_escaped_str("text")),
                ]
            );

            assert_eq!(
                de.next().unwrap(),
                Start(BytesStart::borrowed_name(b"inner"))
            );
            assert_eq!(de.next().unwrap(), End(BytesEnd::borrowed(b"inner")));

            // We finish writing. Next call to `next()` should start replay messages:
            //
            //     text
            //   </inner>
            //
            // and after that stream that messages:
            //
            //   <target/>
            // </root>
            de.start_replay();
            assert_eq!(
                de.read,
                vec![
                    Text(BytesText::from_escaped_str("text")),
                    End(BytesEnd::borrowed(b"inner")),
                ]
            );
            assert_eq!(de.write, vec![]);
            assert_eq!(
                de.next().unwrap(),
                Text(BytesText::from_escaped_str("text"))
            );
            assert_eq!(de.next().unwrap(), End(BytesEnd::borrowed(b"inner")));
            assert_eq!(
                de.next().unwrap(),
                Start(BytesStart::borrowed_name(b"target"))
            );
            assert_eq!(de.next().unwrap(), End(BytesEnd::borrowed(b"target")));
            assert_eq!(de.next().unwrap(), End(BytesEnd::borrowed(b"root")));
        }

        /// Checks that `read_to_end()` behaves correctly after `skip()`
        #[test]
        fn read_to_end() {
            use crate::de::DeEvent::*;
            use crate::events::{BytesEnd, BytesText};
            use pretty_assertions::assert_eq;

            let mut de = Deserializer::from_bytes(
                br#"
                <root>
                    <skip>
                        text
                        <skip/>
                    </skip>
                    <target>
                        <target/>
                    </target>
                </root>
                "#,
            );

            // Initial conditions - both are empty
            assert_eq!(de.read, vec![]);
            assert_eq!(de.write, vec![]);

            assert_eq!(
                de.next().unwrap(),
                Start(BytesStart::borrowed_name(b"root"))
            );

            // Skip the <skip> tree
            de.skip().unwrap();
            assert_eq!(de.read, vec![]);
            assert_eq!(
                de.write,
                vec![
                    Start(BytesStart::borrowed_name(b"skip")),
                    Text(BytesText::from_escaped_str("text")),
                    Start(BytesStart::borrowed_name(b"skip")),
                    End(BytesEnd::borrowed(b"skip")),
                    End(BytesEnd::borrowed(b"skip")),
                ]
            );

            // Drop all events thet represents <target> tree. Now unconsumed XML looks like:
            //
            //   <skip>
            //     text
            //     <skip/>
            //   </skip>
            // </root>
            assert_eq!(
                de.next().unwrap(),
                Start(BytesStart::borrowed_name(b"target"))
            );
            de.read_to_end(b"target").unwrap();
            assert_eq!(de.read, vec![]);
            assert_eq!(
                de.write,
                vec![
                    Start(BytesStart::borrowed_name(b"skip")),
                    Text(BytesText::from_escaped_str("text")),
                    Start(BytesStart::borrowed_name(b"skip")),
                    End(BytesEnd::borrowed(b"skip")),
                    End(BytesEnd::borrowed(b"skip")),
                ]
            );

            // We finish writing. Next call to `next()` should start replay that messages:
            //
            //   <skip>
            //     text
            //     <skip/>
            //   </skip>
            //
            // and after that stream that messages:
            //
            // </root>
            de.start_replay();
            assert_eq!(
                de.read,
                vec![
                    Start(BytesStart::borrowed_name(b"skip")),
                    Text(BytesText::from_escaped_str("text")),
                    Start(BytesStart::borrowed_name(b"skip")),
                    End(BytesEnd::borrowed(b"skip")),
                    End(BytesEnd::borrowed(b"skip")),
                ]
            );
            assert_eq!(de.write, vec![]);

            assert_eq!(
                de.next().unwrap(),
                Start(BytesStart::borrowed_name(b"skip"))
            );
            de.read_to_end(b"skip").unwrap();

            assert_eq!(de.next().unwrap(), End(BytesEnd::borrowed(b"root")));
        }
    }

    #[test]
    fn read_to_end() {
        use crate::de::DeEvent::*;

        let mut de = Deserializer::from_bytes(
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

    /// Tests for trivial XML documents: empty or contains only primitive type
    /// on a top level; all of them should be considered invalid
    mod trivial {
        use super::*;

        #[rustfmt::skip] // excess spaces used for readability
        macro_rules! eof {
            ($name:ident: $type:ty = $value:expr) => {
                #[test]
                fn $name() {
                    let item = from_str::<$type>($value).unwrap_err();

                    match item {
                        DeError::UnexpectedEof => (),
                        _ => panic!("Expected `Eof`, found {:?}", item),
                    }
                }
            };
            ($value:expr) => {
                eof!(i8_:    i8    = $value);
                eof!(i16_:   i16   = $value);
                eof!(i32_:   i32   = $value);
                eof!(i64_:   i64   = $value);
                eof!(isize_: isize = $value);

                eof!(u8_:    u8    = $value);
                eof!(u16_:   u16   = $value);
                eof!(u32_:   u32   = $value);
                eof!(u64_:   u64   = $value);
                eof!(usize_: usize = $value);

                serde_if_integer128! {
                    eof!(u128_: u128 = $value);
                    eof!(i128_: i128 = $value);
                }

                eof!(f32_: f32 = $value);
                eof!(f64_: f64 = $value);

                eof!(false_: bool = $value);
                eof!(true_: bool = $value);
                eof!(char_: char = $value);

                eof!(string: String = $value);
                eof!(byte_buf: ByteBuf = $value);

                #[test]
                fn unit() {
                    let item = from_str::<()>($value).unwrap_err();

                    match item {
                        DeError::UnexpectedEof => (),
                        _ => panic!("Expected `Eof`, found {:?}", item),
                    }
                }
            };
        }

        /// Empty document should considered invalid no matter which type we try to deserialize
        mod empty_doc {
            use super::*;
            eof!("");
        }

        /// Document that contains only comment should be handles as if it is empty
        mod only_comment {
            use super::*;
            eof!("<!--comment-->");
        }

        /// Tests deserialization from top-level tag content: `<root>...content...</root>`
        mod struct_ {
            use super::*;

            /// Well-formed XML must have a single tag at the root level.
            /// Any XML tag can be modeled as a struct, and content of this tag are modeled as
            /// fields of this struct.
            ///
            /// Because we want to get access to unnamed content of the tag (usually, this internal
            /// XML node called `#text`) we use a rename to a special name `$value`
            #[derive(Debug, Deserialize, PartialEq)]
            struct Trivial<T> {
                #[serde(rename = "$value")]
                value: T,
            }

            macro_rules! in_struct {
                ($name:ident: $type:ty = $value:expr, $expected:expr) => {
                    #[test]
                    fn $name() {
                        let item: Trivial<$type> = from_str($value).unwrap();

                        assert_eq!(item, Trivial { value: $expected });

                        match from_str::<Trivial<$type>>(&format!("<outer>{}</outer>", $value)) {
                            // Expected unexpected start element `<root>`
                            Err(DeError::UnexpectedStart(tag)) => assert_eq!(tag, b"root"),
                            x => panic!(
                                r#"Expected `Err(DeError::UnexpectedStart("root"))`, but got `{:?}`"#,
                                x
                            ),
                        }
                    }
                };
            }

            /// Tests deserialization from text content in a tag
            #[rustfmt::skip] // tests formatted in a table
            mod text {
                use super::*;
                use pretty_assertions::assert_eq;

                in_struct!(i8_:  i8  = "<root>-42</root>", -42i8);
                in_struct!(i16_: i16 = "<root>-4200</root>", -4200i16);
                in_struct!(i32_: i32 = "<root>-42000000</root>", -42000000i32);
                in_struct!(i64_: i64 = "<root>-42000000000000</root>", -42000000000000i64);
                in_struct!(isize_: isize = "<root>-42000000000000</root>", -42000000000000isize);

                in_struct!(u8_:  u8  = "<root>42</root>", 42u8);
                in_struct!(u16_: u16 = "<root>4200</root>", 4200u16);
                in_struct!(u32_: u32 = "<root>42000000</root>", 42000000u32);
                in_struct!(u64_: u64 = "<root>42000000000000</root>", 42000000000000u64);
                in_struct!(usize_: usize = "<root>42000000000000</root>", 42000000000000usize);

                serde_if_integer128! {
                    in_struct!(u128_: u128 = "<root>420000000000000000000000000000</root>", 420000000000000000000000000000u128);
                    in_struct!(i128_: i128 = "<root>-420000000000000000000000000000</root>", -420000000000000000000000000000i128);
                }

                in_struct!(f32_: f32 = "<root>4.2</root>", 4.2f32);
                in_struct!(f64_: f64 = "<root>4.2</root>", 4.2f64);

                in_struct!(false_: bool = "<root>false</root>", false);
                in_struct!(true_: bool = "<root>true</root>", true);
                in_struct!(char_: char = "<root>r</root>", 'r');

                in_struct!(string:   String  = "<root>escaped&#x20;string</root>", "escaped string".into());
                // Byte buffers give access to raw data from the input, so never deserialized
                // TODO: It is a bit unusual and it would be better comletely forbid deserialization
                // into bytes, because XML cannot store any bytes natively. User should use some sort
                // of encoding to string, for example, hex or base64
                in_struct!(byte_buf: ByteBuf = "<root>escaped&#x20;byte_buf</root>", ByteBuf(r"escaped&#x20;byte_buf".into()));
            }

            /// Tests deserialization from CDATA content in a tag.
            /// CDATA handling similar to text handling except that strings does not unescapes
            #[rustfmt::skip] // tests formatted in a table
            mod cdata {
                use super::*;
                use pretty_assertions::assert_eq;

                in_struct!(i8_:  i8  = "<root><![CDATA[-42]]></root>", -42i8);
                in_struct!(i16_: i16 = "<root><![CDATA[-4200]]></root>", -4200i16);
                in_struct!(i32_: i32 = "<root><![CDATA[-42000000]]></root>", -42000000i32);
                in_struct!(i64_: i64 = "<root><![CDATA[-42000000000000]]></root>", -42000000000000i64);
                in_struct!(isize_: isize = "<root><![CDATA[-42000000000000]]></root>", -42000000000000isize);

                in_struct!(u8_:  u8  = "<root><![CDATA[42]]></root>", 42u8);
                in_struct!(u16_: u16 = "<root><![CDATA[4200]]></root>", 4200u16);
                in_struct!(u32_: u32 = "<root><![CDATA[42000000]]></root>", 42000000u32);
                in_struct!(u64_: u64 = "<root><![CDATA[42000000000000]]></root>", 42000000000000u64);
                in_struct!(usize_: usize = "<root><![CDATA[42000000000000]]></root>", 42000000000000usize);

                serde_if_integer128! {
                    in_struct!(u128_: u128 = "<root><![CDATA[420000000000000000000000000000]]></root>", 420000000000000000000000000000u128);
                    in_struct!(i128_: i128 = "<root><![CDATA[-420000000000000000000000000000]]></root>", -420000000000000000000000000000i128);
                }

                in_struct!(f32_: f32 = "<root><![CDATA[4.2]]></root>", 4.2f32);
                in_struct!(f64_: f64 = "<root><![CDATA[4.2]]></root>", 4.2f64);

                in_struct!(false_: bool = "<root><![CDATA[false]]></root>", false);
                in_struct!(true_: bool = "<root><![CDATA[true]]></root>", true);
                in_struct!(char_: char = "<root><![CDATA[r]]></root>", 'r');

                // Escape sequences does not processed inside CDATA section
                in_struct!(string:   String  = "<root><![CDATA[escaped&#x20;string]]></root>", "escaped&#x20;string".into());
                in_struct!(byte_buf: ByteBuf = "<root><![CDATA[escaped&#x20;byte_buf]]></root>", ByteBuf(r"escaped&#x20;byte_buf".into()));
            }
        }
    }

    #[test]
    fn multiple_roots_attributes() {
        let s = r##"
            <item name="hello1" source="world1.rs" />
            <item name="hello2" source="world2.rs" />
        "##;

        let item: Vec<Item> = from_str(s).unwrap();

        assert_eq!(
            item,
            vec![
                Item {
                    name: "hello1".to_string(),
                    source: "world1.rs".to_string(),
                },
                Item {
                    name: "hello2".to_string(),
                    source: "world2.rs".to_string(),
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
            Err(DeError::UnexpectedEof) => {}
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
        use pretty_assertions::assert_eq;

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
        use pretty_assertions::assert_eq;

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
        use pretty_assertions::assert_eq;

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
        use pretty_assertions::assert_eq;

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

    mod seq {
        use super::*;

        /// Check that top-level sequences can be deserialized from the multi-root XML documents
        mod top_level {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn simple() {
                let data: [(); 3] = from_str("<root/><root>42</root><root>answer</root>").unwrap();
                assert_eq!(data, [(), (), ()]);
            }

            #[test]
            fn excess_attribute() {
                let data: [(); 3] =
                    from_str(r#"<root/><root excess="attribute">42</root><root>answer</root>"#)
                        .unwrap();
                assert_eq!(data, [(), (), ()]);
            }

            #[test]
            fn mixed_content() {
                let data: [(); 3] = from_str(
                    r#"
                    <element/>
                    text
                    <![CDATA[cdata]]>"#,
                )
                .unwrap();
                assert_eq!(data, [(), (), ()]);
            }
        }

        /// Tests where each sequence item have an identical name in an XML.
        /// That explicitly means that `enum`s as list elements are not supported
        /// in that case, because enum requires different tags.
        ///
        /// (by `enums` we mean [externally tagged enums] is serde terminology)
        ///
        /// [externally tagged enums]: https://serde.rs/enum-representations.html#externally-tagged
        mod fixed_name {
            use super::*;

            /// This module contains tests where size of the list have a compile-time size
            mod fixed_size {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize)]
                struct List {
                    item: [(); 3],
                }

                /// Simple case: count of elements matches expected size of sequence,
                /// each element has the same name. Successful deserialization expected
                #[test]
                fn simple() {
                    from_str::<List>(
                        r#"
                        <root>
                            <item/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    )
                    .unwrap();
                }

                /// Fever elements than expected size of sequence, each element has
                /// the same name. Failure expected
                #[test]
                fn fever_elements() {
                    match from_str::<List>(
                        r#"
                        <root>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    ) {
                        Err(DeError::Custom(e)) => {
                            assert_eq!(e, "invalid length 2, expected an array of length 3")
                        }
                        e => panic!(
                            r#"Expected `Err(Custom("invalid length 2, expected an array of length 3"))`, but found {:?}"#,
                            e
                        ),
                    }
                }

                /// More elements than expected size of sequence, each element has
                /// the same name. Failure expected. If you wish to ignore excess
                /// elements, use the special type, that consume as much elements
                /// as possible, but ignores excess elements
                #[test]
                fn excess_elements() {
                    match from_str::<List>(
                        r#"
                        <root>
                            <item/>
                            <item/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    ) {
                        Err(DeError::Custom(e)) => assert_eq!(e, "duplicate field `item`"),
                        e => panic!(
                            r#"Expected `Err(Custom("duplicate field `item`"))`, but found {:?}"#,
                            e
                        ),
                    }
                }

                /// Mixed content assumes, that some elements will have an internal
                /// name `$value`, so, unless field named the same, it is expected
                /// to fail
                #[test]
                fn mixed_content() {
                    match from_str::<List>(
                        r#"
                        <root>
                            <element/>
                            text
                            <![CDATA[cdata]]>
                        </root>
                        "#,
                    ) {
                        Err(DeError::Custom(e)) => assert_eq!(e, "missing field `item`"),
                        e => panic!(
                            r#"Expected `Err(Custom("missing field `item`"))`, but found {:?}"#,
                            e
                        ),
                    }
                }

                /// Mixed content assumes, that some elements will have an internal
                /// name `$value`, so, we should get all elements
                #[test]
                fn mixed_content_value() {
                    #[derive(Debug, PartialEq, Default, Deserialize)]
                    struct List {
                        #[serde(rename = "$value")]
                        item: [(); 3],
                    }

                    from_str::<List>(
                        r#"
                        <root>
                            <element/>
                            text
                            <![CDATA[cdata]]>
                        </root>
                        "#,
                    )
                    .unwrap();
                }

                /// In those tests sequence should be deserialized from an XML
                /// with additional elements that is not defined in the struct.
                /// That fields should be skipped during deserialization
                mod unknown_items {
                    use super::*;

                    #[test]
                    fn before() {
                        from_str::<List>(
                            r#"
                            <root>
                                <unknown/>
                                <item/>
                                <item/>
                                <item/>
                            </root>
                            "#,
                        )
                        .unwrap();
                    }

                    #[test]
                    fn after() {
                        from_str::<List>(
                            r#"
                            <root>
                                <item/>
                                <item/>
                                <item/>
                                <unknown/>
                            </root>
                            "#,
                        )
                        .unwrap();
                    }

                    #[test]
                    fn overlapped() {
                        from_str::<List>(
                            r#"
                            <root>
                                <item/>
                                <unknown/>
                                <item/>
                                <item/>
                            </root>
                            "#,
                        )
                        .unwrap();
                    }
                }

                /// In those tests non-sequential field is defined in the struct
                /// before sequential, so it will be deserialized before the list.
                /// That struct should be deserialized from an XML where these
                /// fields comes in an arbitrary order
                mod field_before_list {
                    use super::*;

                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Root {
                        node: (),
                        item: [(); 3],
                    }

                    #[test]
                    fn before() {
                        from_str::<Root>(
                            r#"
                            <root>
                                <node/>
                                <item/>
                                <item/>
                                <item/>
                            </root>
                            "#,
                        )
                        .unwrap();
                    }

                    #[test]
                    fn after() {
                        from_str::<Root>(
                            r#"
                            <root>
                                <item/>
                                <item/>
                                <item/>
                                <node/>
                            </root>
                            "#,
                        )
                        .unwrap();
                    }

                    #[test]
                    fn overlapped() {
                        from_str::<Root>(
                            r#"
                            <root>
                                <item/>
                                <node/>
                                <item/>
                                <item/>
                            </root>
                            "#,
                        )
                        .unwrap();
                    }
                }

                /// In those tests non-sequential field is defined in the struct
                /// after sequential, so it will be deserialized after the list.
                /// That struct should be deserialized from an XML where these
                /// fields comes in an arbitrary order
                mod field_after_list {
                    use super::*;

                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Root {
                        item: [(); 3],
                        node: (),
                    }

                    #[test]
                    fn before() {
                        from_str::<Root>(
                            r#"
                            <root>
                                <node/>
                                <item/>
                                <item/>
                                <item/>
                            </root>
                            "#,
                        )
                        .unwrap();
                    }

                    #[test]
                    fn after() {
                        from_str::<Root>(
                            r#"
                            <root>
                                <item/>
                                <item/>
                                <item/>
                                <node/>
                            </root>
                            "#,
                        )
                        .unwrap();
                    }

                    #[test]
                    fn overlapped() {
                        from_str::<Root>(
                            r#"
                            <root>
                                <item/>
                                <node/>
                                <item/>
                                <item/>
                            </root>
                            "#,
                        )
                        .unwrap();
                    }
                }

                /// In those tests two lists are deserialized simultaniously.
                /// Lists shuould be deserialized even when them overlaps
                mod two_lists {
                    use super::*;

                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Pair {
                        item: [(); 3],
                        element: [(); 2],
                    }

                    #[test]
                    fn splitted() {
                        from_str::<Pair>(
                            r#"
                            <root>
                                <element/>
                                <element/>
                                <item/>
                                <item/>
                                <item/>
                            </root>
                            "#,
                        )
                        .unwrap();
                    }

                    #[test]
                    fn overlapped() {
                        from_str::<Pair>(
                            r#"
                            <root>
                                <item/>
                                <element/>
                                <item/>
                                <element/>
                                <item/>
                            </root>
                            "#,
                        )
                        .unwrap();
                    }
                }
            }

            /// This module contains tests where size of the list have an unspecified size
            mod variable_size {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize)]
                struct List {
                    item: Vec<()>,
                }

                /// Simple case: count of elements matches expected size of sequence,
                /// each element has the same name. Successful deserialization expected
                #[test]
                fn simple() {
                    from_str::<List>(
                        r#"
                        <root>
                            <item/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    )
                    .unwrap();
                }

                /// Mixed content assumes, that some elements will have an internal
                /// name `$value`, so, unless field named the same, it is expected
                /// to fail
                #[test]
                fn mixed_content() {
                    match from_str::<List>(
                        r#"
                        <root>
                            <element/>
                            text
                            <![CDATA[cdata]]>
                        </root>
                        "#,
                    ) {
                        Err(DeError::Custom(e)) => assert_eq!(e, "missing field `item`"),
                        e => panic!(
                            r#"Expected `Err(Custom("missing field `item`"))`, but found {:?}"#,
                            e
                        ),
                    }
                }

                /// Mixed content assumes, that some elements will have an internal
                /// name `$value`, so, we should get all elements
                #[test]
                fn mixed_content_value() {
                    #[derive(Debug, PartialEq, Default, Deserialize)]
                    struct List {
                        #[serde(rename = "$value")]
                        item: Vec<()>,
                    }

                    from_str::<List>(
                        r#"
                        <root>
                            <element/>
                            text
                            <![CDATA[cdata]]>
                        </root>
                        "#,
                    )
                    .unwrap();
                }

                /// In those tests sequence should be deserialized from the XML
                /// with additional elements that is not defined in the struct.
                /// That fields should be skipped during deserialization
                mod unknown_items {
                    use super::*;

                    #[test]
                    fn before() {
                        from_str::<List>(
                            r#"
                            <root>
                                <unknown/>
                                <item/>
                                <item/>
                                <item/>
                            </root>
                            "#,
                        )
                        .unwrap();
                    }

                    #[test]
                    fn after() {
                        from_str::<List>(
                            r#"
                            <root>
                                <item/>
                                <item/>
                                <item/>
                                <unknown/>
                            </root>
                            "#,
                        )
                        .unwrap();
                    }

                    #[test]
                    fn overlapped() {
                        from_str::<List>(
                            r#"
                            <root>
                                <item/>
                                <unknown/>
                                <item/>
                                <item/>
                            </root>
                            "#,
                        )
                        .unwrap();
                    }
                }

                /// In those tests non-sequential field is defined in the struct
                /// before sequential, so it will be deserialized before the list.
                /// That struct should be deserialized from the XML where these
                /// fields comes in an arbitrary order
                mod field_before_list {
                    use super::*;

                    #[derive(Debug, PartialEq, Default, Deserialize)]
                    struct Root {
                        node: (),
                        item: [(); 3],
                    }

                    #[test]
                    fn before() {
                        from_str::<Root>(
                            r#"
                            <root>
                                <node/>
                                <item/>
                                <item/>
                                <item/>
                            </root>
                            "#,
                        )
                        .unwrap();
                    }

                    #[test]
                    fn after() {
                        from_str::<Root>(
                            r#"
                            <root>
                                <item/>
                                <item/>
                                <item/>
                                <node/>
                            </root>
                            "#,
                        )
                        .unwrap();
                    }

                    #[test]
                    fn overlapped() {
                        from_str::<Root>(
                            r#"
                            <root>
                                <item/>
                                <node/>
                                <item/>
                                <item/>
                            </root>
                            "#,
                        )
                        .unwrap();
                    }
                }

                /// In those tests non-sequential field is defined in the struct
                /// after sequential, so it will be deserialized after the list.
                /// That struct should be deserialized from the XML where these
                /// fields comes in an arbitrary order
                mod field_after_list {
                    use super::*;

                    #[derive(Debug, PartialEq, Default, Deserialize)]
                    struct Root {
                        item: [(); 3],
                        node: (),
                    }

                    #[test]
                    fn before() {
                        from_str::<Root>(
                            r#"
                            <root>
                                <node/>
                                <item/>
                                <item/>
                                <item/>
                            </root>
                            "#,
                        )
                        .unwrap();
                    }

                    #[test]
                    fn after() {
                        from_str::<Root>(
                            r#"
                            <root>
                                <item/>
                                <item/>
                                <item/>
                                <node/>
                            </root>
                            "#,
                        )
                        .unwrap();
                    }

                    #[test]
                    fn overlapped() {
                        from_str::<Root>(
                            r#"
                            <root>
                                <item/>
                                <node/>
                                <item/>
                                <item/>
                            </root>
                            "#,
                        )
                        .unwrap();
                    }
                }

                /// In those tests two lists are deserialized simultaniously.
                /// Lists shuould be deserialized even when them overlaps
                mod two_lists {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Pair {
                        item: Vec<()>,
                        element: Vec<()>,
                    }

                    #[test]
                    fn splitted() {
                        assert_eq!(
                            from_str::<Pair>(
                                r#"
                                <root>
                                    <element/>
                                    <element/>
                                    <item/>
                                    <item/>
                                    <item/>
                                </root>
                                "#,
                            )
                            .unwrap(),
                            Pair {
                                item: vec![(), (), ()],
                                element: vec![(), ()],
                            }
                        );
                    }

                    #[test]
                    fn overlapped() {
                        assert_eq!(
                            from_str::<Pair>(
                                r#"
                                <root>
                                    <item/>
                                    <element/>
                                    <item/>
                                    <element/>
                                    <item/>
                                </root>
                                "#,
                            )
                            .unwrap(),
                            Pair {
                                item: vec![(), (), ()],
                                element: vec![(), ()],
                            }
                        );
                    }
                }
            }
        }

        /// Check that sequences inside element can be deserialized.
        /// In terms of serde this is a sequence flatten into the struct:
        ///
        /// ```ignore
        /// struct Root {
        ///   #[serde(flatten)]
        ///   items: Vec<T>,
        /// }
        /// ```
        /// except that fact that this is not supported nowadays
        /// (https://github.com/serde-rs/serde/issues/1905)
        ///
        /// Because this is very frequently used pattern in the XML, quick-xml
        /// have a workaround for this. If a field will have a special name `$value`
        /// then any `xs:element`s in the `xs:sequence` / `xs:all`, excapt that
        /// which name matchesthe struct name, will be associated with this field:
        ///
        /// ```ignore
        /// struct Root {
        ///   field: U,
        ///   #[serde(rename = "$value")]
        ///   items: Vec<Enum>,
        /// }
        /// ```
        /// In this example `<field>` tag will be associated with a `field` field,
        /// but all other tags will be associated with an `items` field. Disadvantages
        /// of this approach that you can have only one field, but usually you don't
        /// want more
        mod variable_name {
            use super::*;
            use serde::de::{Deserializer, VariantAccess};
            use std::fmt::{self, Formatter};

            // NOTE: Derive could be possible once https://github.com/serde-rs/serde/issues/2126 is resolved
            macro_rules! impl_deserialize_choice {
                ($name:ident : $(($field:ident, $field_name:literal)),*) => {
                    impl<'de> Deserialize<'de> for $name {
                        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                        where
                            D: Deserializer<'de>,
                        {
                            #[derive(Deserialize)]
                            #[serde(field_identifier)]
                            #[serde(rename_all = "kebab-case")]
                            enum Tag {
                                $($field,)*
                                Other(String),
                            }

                            struct EnumVisitor;
                            impl<'de> de::Visitor<'de> for EnumVisitor {
                                type Value = $name;

                                fn expecting(&self, f: &mut Formatter) -> fmt::Result {
                                    f.write_str("enum ")?;
                                    f.write_str(stringify!($name))
                                }

                                fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
                                where
                                    A: de::EnumAccess<'de>,
                                {
                                    match data.variant()? {
                                        $(
                                            (Tag::$field, variant) => variant.unit_variant().map(|_| $name::$field),
                                        )*
                                        (Tag::Other(t), v) => v.unit_variant().map(|_| $name::Other(t)),
                                    }
                                }
                            }

                            const VARIANTS: &'static [&'static str] = &[
                                $($field_name,)*
                                "<any other tag>"
                            ];
                            deserializer.deserialize_enum(stringify!($name), VARIANTS, EnumVisitor)
                        }
                    }
                };
            }

            /// Type that can be deserialized from `<one>`, `<two>`, or any other element
            #[derive(Debug, PartialEq)]
            enum Choice {
                One,
                Two,
                /// Any other tag name except `One` or `Two`
                Other(String),
            }
            impl_deserialize_choice!(Choice: (One, "one"), (Two, "two"));

            /// Type that can be deserialized from `<first>`, `<second>`, or any other element
            #[derive(Debug, PartialEq)]
            enum Choice2 {
                First,
                Second,
                /// Any other tag name except `First` or `Second`
                Other(String),
            }
            impl_deserialize_choice!(Choice2: (First, "first"), (Second, "second"));

            /// This module contains tests where size of the list have a compile-time size
            mod fixed_size {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize)]
                struct List {
                    #[serde(rename = "$value")]
                    item: [Choice; 3],
                }

                /// Simple case: count of elements matches expected size of sequence,
                /// each element has the same name. Successful deserialization expected
                #[test]
                fn simple() {
                    assert_eq!(
                        from_str::<List>(
                            r#"
                            <root>
                                <one/>
                                <two/>
                                <three/>
                            </root>
                            "#,
                        )
                        .unwrap(),
                        List {
                            item: [Choice::One, Choice::Two, Choice::Other("three".into())]
                        }
                    );
                }

                /// Fever elements than expected size of sequence, each element has
                /// the same name. Failure expected
                #[test]
                fn fever_elements() {
                    from_str::<List>(
                        r#"
                        <root>
                            <one/>
                            <two/>
                        </root>
                        "#,
                    )
                    .unwrap_err();
                }

                /// More elements than expected size of sequence, each element has
                /// the same name. Failure expected. If you wish to ignore excess
                /// elements, use the special type, that consume as much elements
                /// as possible, but ignores excess elements
                #[test]
                fn excess_elements() {
                    from_str::<List>(
                        r#"
                        <root>
                            <one/>
                            <two/>
                            <three/>
                            <four/>
                        </root>
                        "#,
                    )
                    .unwrap_err();
                }

                #[test]
                fn mixed_content() {
                    #[derive(Debug, PartialEq, Deserialize)]
                    struct List {
                        #[serde(rename = "$value")]
                        item: [(); 3],
                    }

                    from_str::<List>(
                        r#"
                        <root>
                            <element/>
                            text
                            <![CDATA[cdata]]>
                        </root>
                        "#,
                    )
                    .unwrap();
                }

                // There cannot be unknown items, because any tag name is accepted

                /// In those tests non-sequential field is defined in the struct
                /// before sequential, so it will be deserialized before the list.
                /// That struct should be deserialized from the XML where these
                /// fields comes in an arbitrary order
                mod field_before_list {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Root {
                        node: (),
                        #[serde(rename = "$value")]
                        item: [Choice; 3],
                    }

                    #[test]
                    fn before() {
                        assert_eq!(
                            from_str::<Root>(
                                r#"
                                <root>
                                    <node/>
                                    <one/>
                                    <two/>
                                    <three/>
                                </root>
                                "#,
                            )
                            .unwrap(),
                            Root {
                                node: (),
                                item: [Choice::One, Choice::Two, Choice::Other("three".into())]
                            }
                        );
                    }

                    #[test]
                    fn after() {
                        assert_eq!(
                            from_str::<Root>(
                                r#"
                                <root>
                                    <one/>
                                    <two/>
                                    <three/>
                                    <node/>
                                </root>
                                "#,
                            )
                            .unwrap(),
                            Root {
                                node: (),
                                item: [Choice::One, Choice::Two, Choice::Other("three".into())]
                            }
                        );
                    }

                    #[test]
                    fn overlapped() {
                        assert_eq!(
                            from_str::<Root>(
                                r#"
                                <root>
                                    <one/>
                                    <node/>
                                    <two/>
                                    <three/>
                                </root>
                                "#,
                            )
                            .unwrap(),
                            Root {
                                node: (),
                                item: [Choice::One, Choice::Two, Choice::Other("three".into())]
                            }
                        );
                    }
                }

                /// In those tests non-sequential field is defined in the struct
                /// after sequential, so it will be deserialized after the list.
                /// That struct should be deserialized from the XML where these
                /// fields comes in an arbitrary order
                mod field_after_list {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Root {
                        #[serde(rename = "$value")]
                        item: [Choice; 3],
                        node: (),
                    }

                    #[test]
                    fn before() {
                        assert_eq!(
                            from_str::<Root>(
                                r#"
                                <root>
                                    <node/>
                                    <one/>
                                    <two/>
                                    <three/>
                                </root>
                                "#,
                            )
                            .unwrap(),
                            Root {
                                item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                node: (),
                            }
                        );
                    }

                    #[test]
                    fn after() {
                        assert_eq!(
                            from_str::<Root>(
                                r#"
                                <root>
                                    <one/>
                                    <two/>
                                    <three/>
                                    <node/>
                                </root>
                                "#,
                            )
                            .unwrap(),
                            Root {
                                item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                node: (),
                            }
                        );
                    }

                    #[test]
                    fn overlapped() {
                        assert_eq!(
                            from_str::<Root>(
                                r#"
                                <root>
                                    <one/>
                                    <node/>
                                    <two/>
                                    <three/>
                                </root>
                                "#,
                            )
                            .unwrap(),
                            Root {
                                item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                node: (),
                            }
                        );
                    }
                }

                /// In those tests two lists are deserialized simultaniously.
                /// Lists shuould be deserialized even when them overlaps
                mod two_lists {
                    use super::*;

                    /// A field with a variable-name items defined before a field with a fixed-name
                    /// items
                    mod choice_and_fixed {
                        use super::*;
                        use pretty_assertions::assert_eq;

                        #[derive(Debug, PartialEq, Deserialize)]
                        struct Pair {
                            #[serde(rename = "$value")]
                            item: [Choice; 3],
                            element: [(); 2],
                        }

                        /// A list with fixed-name elements located before a list with variable-name
                        /// elements in an XML
                        #[test]
                        fn fixed_before() {
                            assert_eq!(
                                from_str::<Pair>(
                                    r#"
                                    <root>
                                        <element/>
                                        <element/>
                                        <one/>
                                        <two/>
                                        <three/>
                                    </root>
                                    "#,
                                )
                                .unwrap(),
                                Pair {
                                    item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                    element: [(), ()],
                                }
                            );
                        }

                        /// A list with fixed-name elements located after a list with variable-name
                        /// elements in an XML
                        #[test]
                        fn fixed_after() {
                            assert_eq!(
                                from_str::<Pair>(
                                    r#"
                                    <root>
                                        <one/>
                                        <two/>
                                        <three/>
                                        <element/>
                                        <element/>
                                    </root>
                                    "#,
                                )
                                .unwrap(),
                                Pair {
                                    item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                    element: [(), ()],
                                }
                            );
                        }

                        /// A list with fixed-name elements are mixed with a list with variable-name
                        /// elements in an XML, and the first element is a fixed-name one
                        #[test]
                        fn overlapped_fixed_before() {
                            assert_eq!(
                                from_str::<Pair>(
                                    r#"
                                    <root>
                                        <element/>
                                        <one/>
                                        <two/>
                                        <element/>
                                        <three/>
                                    </root>
                                    "#,
                                )
                                .unwrap(),
                                Pair {
                                    item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                    element: [(), ()],
                                }
                            );
                        }

                        /// A list with fixed-name elements are mixed with a list with variable-name
                        /// elements in an XML, and the first element is a variable-name one
                        #[test]
                        fn overlapped_fixed_after() {
                            assert_eq!(
                                from_str::<Pair>(
                                    r#"
                                    <root>
                                        <one/>
                                        <element/>
                                        <two/>
                                        <three/>
                                        <element/>
                                    </root>
                                    "#,
                                )
                                .unwrap(),
                                Pair {
                                    item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                    element: [(), ()],
                                }
                            );
                        }
                    }

                    /// A field with a variable-name items defined after a field with a fixed-name
                    /// items
                    mod fixed_and_choice {
                        use super::*;
                        use pretty_assertions::assert_eq;

                        #[derive(Debug, PartialEq, Deserialize)]
                        struct Pair {
                            element: [(); 2],
                            #[serde(rename = "$value")]
                            item: [Choice; 3],
                        }

                        /// A list with fixed-name elements located before a list with variable-name
                        /// elements in an XML
                        #[test]
                        fn fixed_before() {
                            assert_eq!(
                                from_str::<Pair>(
                                    r#"
                                    <root>
                                        <element/>
                                        <element/>
                                        <one/>
                                        <two/>
                                        <three/>
                                    </root>
                                    "#,
                                )
                                .unwrap(),
                                Pair {
                                    item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                    element: [(), ()],
                                }
                            );
                        }

                        /// A list with fixed-name elements located after a list with variable-name
                        /// elements in an XML
                        #[test]
                        fn fixed_after() {
                            assert_eq!(
                                from_str::<Pair>(
                                    r#"
                                    <root>
                                        <one/>
                                        <two/>
                                        <three/>
                                        <element/>
                                        <element/>
                                    </root>
                                    "#,
                                )
                                .unwrap(),
                                Pair {
                                    item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                    element: [(), ()],
                                }
                            );
                        }

                        /// A list with fixed-name elements are mixed with a list with variable-name
                        /// elements in an XML, and the first element is a fixed-name one
                        #[test]
                        fn overlapped_fixed_before() {
                            assert_eq!(
                                from_str::<Pair>(
                                    r#"
                                    <root>
                                        <element/>
                                        <one/>
                                        <two/>
                                        <element/>
                                        <three/>
                                    </root>
                                    "#,
                                )
                                .unwrap(),
                                Pair {
                                    item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                    element: [(), ()],
                                }
                            );
                        }

                        /// A list with fixed-name elements are mixed with a list with variable-name
                        /// elements in an XML, and the first element is a variable-name one
                        #[test]
                        fn overlapped_fixed_after() {
                            assert_eq!(
                                from_str::<Pair>(
                                    r#"
                                    <root>
                                        <one/>
                                        <element/>
                                        <two/>
                                        <three/>
                                        <element/>
                                    </root>
                                    "#,
                                )
                                .unwrap(),
                                Pair {
                                    item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                    element: [(), ()],
                                }
                            );
                        }
                    }

                    /// Tests are ignored, but exists to show a problem.
                    /// May be it will be solved in the future
                    mod choice_and_choice {
                        use super::*;
                        use pretty_assertions::assert_eq;

                        #[derive(Debug, PartialEq, Deserialize)]
                        struct Pair {
                            #[serde(rename = "$value")]
                            item: [Choice; 3],
                            // Actually, we cannot rename both fields to `$value`, which is now
                            // required to indicate, that field accepts elements with any name
                            #[serde(rename = "$value")]
                            element: [Choice2; 2],
                        }

                        #[test]
                        #[ignore = "There is no way to associate XML elements with `item` or `element` without extra knoledge from type"]
                        fn splitted() {
                            assert_eq!(
                                from_str::<Pair>(
                                    r#"
                                    <root>
                                        <first/>
                                        <second/>
                                        <one/>
                                        <two/>
                                        <three/>
                                    </root>
                                    "#,
                                )
                                .unwrap(),
                                Pair {
                                    item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                    element: [Choice2::First, Choice2::Second],
                                }
                            );
                        }

                        #[test]
                        #[ignore = "There is no way to associate XML elements with `item` or `element` without extra knoledge from type"]
                        fn overlapped() {
                            assert_eq!(
                                from_str::<Pair>(
                                    r#"
                                    <root>
                                        <one/>
                                        <first/>
                                        <two/>
                                        <second/>
                                        <three/>
                                    </root>
                                    "#,
                                )
                                .unwrap(),
                                Pair {
                                    item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                    element: [Choice2::First, Choice2::Second],
                                }
                            );
                        }
                    }
                }
            }

            /// This module contains tests where size of the list have an unspecified size
            mod variable_size {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize)]
                struct List {
                    #[serde(rename = "$value")]
                    item: Vec<Choice>,
                }

                /// Simple case: count of elements matches expected size of sequence,
                /// each element has the same name. Successful deserialization expected
                #[test]
                fn simple() {
                    assert_eq!(
                        from_str::<List>(
                            r#"
                            <root>
                                <one/>
                                <two/>
                                <three/>
                            </root>
                            "#,
                        )
                        .unwrap(),
                        List {
                            item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                        }
                    );
                }

                #[test]
                fn mixed_content() {
                    #[derive(Debug, PartialEq, Deserialize)]
                    struct List {
                        #[serde(rename = "$value")]
                        item: Vec<()>,
                    }

                    assert_eq!(
                        from_str::<List>(
                            r#"
                            <root>
                                <element/>
                                text
                                <![CDATA[cdata]]>
                            </root>
                            "#,
                        )
                        .unwrap(),
                        List {
                            item: vec![(), (), ()],
                        }
                    );
                }

                // There cannot be unknown items, because any tag name is accepted

                /// In those tests non-sequential field is defined in the struct
                /// before sequential, so it will be deserialized before the list.
                /// That struct should be deserialized from the XML where these
                /// fields comes in an arbitrary order
                mod field_before_list {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Root {
                        node: (),
                        #[serde(rename = "$value")]
                        item: Vec<Choice>,
                    }

                    #[test]
                    fn before() {
                        assert_eq!(
                            from_str::<Root>(
                                r#"
                                <root>
                                    <node/>
                                    <one/>
                                    <two/>
                                    <three/>
                                </root>
                                "#,
                            )
                            .unwrap(),
                            Root {
                                node: (),
                                item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                            }
                        );
                    }

                    #[test]
                    fn after() {
                        assert_eq!(
                            from_str::<Root>(
                                r#"
                                <root>
                                    <one/>
                                    <two/>
                                    <three/>
                                    <node/>
                                </root>
                                "#,
                            )
                            .unwrap(),
                            Root {
                                node: (),
                                item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                            }
                        );
                    }

                    #[test]
                    fn overlapped() {
                        assert_eq!(
                            from_str::<Root>(
                                r#"
                                <root>
                                    <one/>
                                    <node/>
                                    <two/>
                                    <three/>
                                </root>
                                "#,
                            )
                            .unwrap(),
                            Root {
                                node: (),
                                item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                            }
                        );
                    }
                }

                /// In those tests non-sequential field is defined in the struct
                /// after sequential, so it will be deserialized after the list.
                /// That struct should be deserialized from the XML where these
                /// fields comes in an arbitrary order
                mod field_after_list {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Root {
                        #[serde(rename = "$value")]
                        item: Vec<Choice>,
                        node: (),
                    }

                    #[test]
                    fn before() {
                        assert_eq!(
                            from_str::<Root>(
                                r#"
                                <root>
                                    <node/>
                                    <one/>
                                    <two/>
                                    <three/>
                                </root>
                                "#,
                            )
                            .unwrap(),
                            Root {
                                item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                                node: (),
                            }
                        );
                    }

                    #[test]
                    fn after() {
                        assert_eq!(
                            from_str::<Root>(
                                r#"
                                <root>
                                    <one/>
                                    <two/>
                                    <three/>
                                    <node/>
                                </root>
                                "#,
                            )
                            .unwrap(),
                            Root {
                                item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                                node: (),
                            }
                        );
                    }

                    #[test]
                    fn overlapped() {
                        assert_eq!(
                            from_str::<Root>(
                                r#"
                                <root>
                                    <one/>
                                    <node/>
                                    <two/>
                                    <three/>
                                </root>
                                "#,
                            )
                            .unwrap(),
                            Root {
                                item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                                node: (),
                            }
                        );
                    }
                }

                /// In those tests two lists are deserialized simultaniously.
                /// Lists shuould be deserialized even when them overlaps
                mod two_lists {
                    use super::*;

                    /// A field with a variable-name items defined before a field with a fixed-name
                    /// items
                    mod choice_and_fixed {
                        use super::*;
                        use pretty_assertions::assert_eq;

                        #[derive(Debug, PartialEq, Deserialize)]
                        struct Pair {
                            #[serde(rename = "$value")]
                            item: Vec<Choice>,
                            element: Vec<()>,
                        }

                        /// A list with fixed-name elements located before a list with variable-name
                        /// elements in an XML
                        #[test]
                        fn fixed_before() {
                            assert_eq!(
                                from_str::<Pair>(
                                    r#"
                                    <root>
                                        <element/>
                                        <element/>
                                        <one/>
                                        <two/>
                                        <three/>
                                    </root>
                                    "#,
                                )
                                .unwrap(),
                                Pair {
                                    item: vec![
                                        Choice::One,
                                        Choice::Two,
                                        Choice::Other("three".into()),
                                    ],
                                    element: vec![(), ()],
                                }
                            );
                        }

                        /// A list with fixed-name elements located after a list with variable-name
                        /// elements in an XML
                        #[test]
                        fn fixed_after() {
                            assert_eq!(
                                from_str::<Pair>(
                                    r#"
                                    <root>
                                        <one/>
                                        <two/>
                                        <three/>
                                        <element/>
                                        <element/>
                                    </root>
                                    "#,
                                )
                                .unwrap(),
                                Pair {
                                    item: vec![
                                        Choice::One,
                                        Choice::Two,
                                        Choice::Other("three".into()),
                                    ],
                                    element: vec![(), ()],
                                }
                            );
                        }

                        /// A list with fixed-name elements are mixed with a list with variable-name
                        /// elements in an XML, and the first element is a fixed-name one
                        #[test]
                        fn overlapped_fixed_before() {
                            assert_eq!(
                                from_str::<Pair>(
                                    r#"
                                    <root>
                                        <element/>
                                        <one/>
                                        <two/>
                                        <element/>
                                        <three/>
                                    </root>
                                    "#,
                                )
                                .unwrap(),
                                Pair {
                                    item: vec![
                                        Choice::One,
                                        Choice::Two,
                                        Choice::Other("three".into()),
                                    ],
                                    element: vec![(), ()],
                                }
                            );
                        }

                        /// A list with fixed-name elements are mixed with a list with variable-name
                        /// elements in an XML, and the first element is a variable-name one
                        #[test]
                        fn overlapped_fixed_after() {
                            assert_eq!(
                                from_str::<Pair>(
                                    r#"
                                    <root>
                                        <one/>
                                        <element/>
                                        <two/>
                                        <three/>
                                        <element/>
                                    </root>
                                    "#,
                                )
                                .unwrap(),
                                Pair {
                                    item: vec![
                                        Choice::One,
                                        Choice::Two,
                                        Choice::Other("three".into()),
                                    ],
                                    element: vec![(), ()],
                                }
                            );
                        }
                    }

                    /// A field with a variable-name items defined after a field with a fixed-name
                    /// items
                    mod fixed_and_choice {
                        use super::*;
                        use pretty_assertions::assert_eq;

                        #[derive(Debug, PartialEq, Deserialize)]
                        struct Pair {
                            element: Vec<()>,
                            #[serde(rename = "$value")]
                            item: Vec<Choice>,
                        }

                        /// A list with fixed-name elements located before a list with variable-name
                        /// elements in an XML
                        #[test]
                        fn fixed_before() {
                            assert_eq!(
                                from_str::<Pair>(
                                    r#"
                                    <root>
                                        <element/>
                                        <element/>
                                        <one/>
                                        <two/>
                                        <three/>
                                    </root>
                                    "#,
                                )
                                .unwrap(),
                                Pair {
                                    element: vec![(), ()],
                                    item: vec![
                                        Choice::One,
                                        Choice::Two,
                                        Choice::Other("three".into()),
                                    ],
                                }
                            );
                        }

                        /// A list with fixed-name elements located after a list with variable-name
                        /// elements in an XML
                        #[test]
                        fn fixed_after() {
                            assert_eq!(
                                from_str::<Pair>(
                                    r#"
                                    <root>
                                        <one/>
                                        <two/>
                                        <three/>
                                        <element/>
                                        <element/>
                                    </root>
                                    "#,
                                )
                                .unwrap(),
                                Pair {
                                    element: vec![(), ()],
                                    item: vec![
                                        Choice::One,
                                        Choice::Two,
                                        Choice::Other("three".into()),
                                    ],
                                }
                            );
                        }

                        /// A list with fixed-name elements are mixed with a list with variable-name
                        /// elements in an XML, and the first element is a fixed-name one
                        #[test]
                        fn overlapped_fixed_before() {
                            assert_eq!(
                                from_str::<Pair>(
                                    r#"
                                    <root>
                                        <element/>
                                        <one/>
                                        <two/>
                                        <element/>
                                        <three/>
                                    </root>
                                    "#,
                                )
                                .unwrap(),
                                Pair {
                                    element: vec![(), ()],
                                    item: vec![
                                        Choice::One,
                                        Choice::Two,
                                        Choice::Other("three".into()),
                                    ],
                                }
                            );
                        }

                        /// A list with fixed-name elements are mixed with a list with variable-name
                        /// elements in an XML, and the first element is a variable-name one
                        #[test]
                        fn overlapped_fixed_after() {
                            assert_eq!(
                                from_str::<Pair>(
                                    r#"
                                    <root>
                                        <one/>
                                        <element/>
                                        <two/>
                                        <three/>
                                        <element/>
                                    </root>
                                    "#,
                                )
                                .unwrap(),
                                Pair {
                                    element: vec![(), ()],
                                    item: vec![
                                        Choice::One,
                                        Choice::Two,
                                        Choice::Other("three".into()),
                                    ],
                                }
                            );
                        }
                    }

                    /// Tests are ignored, but exists to show a problem.
                    /// May be it will be solved in the future
                    mod choice_and_choice {
                        use super::*;
                        use pretty_assertions::assert_eq;

                        #[derive(Debug, PartialEq, Deserialize)]
                        struct Pair {
                            #[serde(rename = "$value")]
                            item: Vec<Choice>,
                            // Actually, we cannot rename both fields to `$value`, which is now
                            // required to indicate, that field accepts elements with any name
                            #[serde(rename = "$value")]
                            element: Vec<Choice2>,
                        }

                        #[test]
                        #[ignore = "There is no way to associate XML elements with `item` or `element` without extra knoledge from type"]
                        fn splitted() {
                            assert_eq!(
                                from_str::<Pair>(
                                    r#"
                                    <root>
                                        <first/>
                                        <second/>
                                        <one/>
                                        <two/>
                                        <three/>
                                    </root>
                                    "#,
                                )
                                .unwrap(),
                                Pair {
                                    item: vec![
                                        Choice::One,
                                        Choice::Two,
                                        Choice::Other("three".into()),
                                    ],
                                    element: vec![Choice2::First, Choice2::Second],
                                }
                            );
                        }

                        #[test]
                        #[ignore = "There is no way to associate XML elements with `item` or `element` without extra knoledge from type"]
                        fn overlapped() {
                            assert_eq!(
                                from_str::<Pair>(
                                    r#"
                                    <root>
                                        <one/>
                                        <first/>
                                        <two/>
                                        <second/>
                                        <three/>
                                    </root>
                                    "#,
                                )
                                .unwrap(),
                                Pair {
                                    item: vec![
                                        Choice::One,
                                        Choice::Two,
                                        Choice::Other("three".into()),
                                    ],
                                    element: vec![Choice2::First, Choice2::Second],
                                }
                            );
                        }
                    }
                }
            }
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
                        Err(DeError::UnexpectedEof) => (),
                        _ => panic!("Expected `Eof`, found {:?}", data),
                    }
                }

                #[test]
                fn elements_root() {
                    let data = from_str::<$type>(r#"<root float="42"><string>answer</string>"#);

                    match data {
                        Err(DeError::UnexpectedEof) => (),
                        _ => panic!("Expected `Eof`, found {:?}", data),
                    }
                }

                #[test]
                fn elements_child() {
                    let data = from_str::<$type>(r#"<root float="42"><string>answer"#);

                    match data {
                        Err(DeError::UnexpectedEof) => (),
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
                        Err(DeError::InvalidXml(EndEventMismatch { .. })) => (),
                        _ => panic!("Expected `InvalidXml(EndEventMismatch)`, found {:?}", data),
                    }
                }

                #[test]
                fn elements_root() {
                    let data = from_str::<$type>(
                        r#"<root float="42"><string>answer</string></mismatched>"#,
                    );

                    match data {
                        Err(DeError::InvalidXml(EndEventMismatch { .. })) => (),
                        _ => panic!("Expected `InvalidXml(EndEventMismatch)`, found {:?}", data),
                    }
                }

                #[test]
                fn elements_child() {
                    let data =
                        from_str::<$type>(r#"<root float="42"><string>answer</mismatched></root>"#);

                    match data {
                        Err(DeError::InvalidXml(EndEventMismatch { .. })) => (),
                        _ => panic!("Expected `InvalidXml(EndEventMismatch)`, found {:?}", data),
                    }
                }
            }
        };
    }

    mod map {
        use super::*;
        use pretty_assertions::assert_eq;
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
        use pretty_assertions::assert_eq;

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
        use pretty_assertions::assert_eq;

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
        use pretty_assertions::assert_eq;

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
            use pretty_assertions::assert_eq;

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
                use pretty_assertions::assert_eq;

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
                use pretty_assertions::assert_eq;

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
                use pretty_assertions::assert_eq;

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
                use pretty_assertions::assert_eq;

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
                use pretty_assertions::assert_eq;

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
                use pretty_assertions::assert_eq;

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
                use pretty_assertions::assert_eq;

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
                use pretty_assertions::assert_eq;

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
                use pretty_assertions::assert_eq;

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
                use pretty_assertions::assert_eq;

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
                use pretty_assertions::assert_eq;

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
                use pretty_assertions::assert_eq;

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
                use pretty_assertions::assert_eq;

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
                use pretty_assertions::assert_eq;

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
            use pretty_assertions::assert_eq;

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
                use pretty_assertions::assert_eq;

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
                use pretty_assertions::assert_eq;

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
                use pretty_assertions::assert_eq;

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

    /// https://www.w3schools.com/xml/el_list.asp
    mod xml_schema_lists {
        use super::*;

        macro_rules! list {
            ($name:ident: $type:ty = $xml:literal => $result:expr) => {
                #[test]
                fn $name() {
                    let data: List<$type> = from_str($xml).unwrap();

                    assert_eq!(data, List { list: $result });
                }
            };
        }

        macro_rules! err {
            ($name:ident: $type:ty = $xml:literal => $kind:ident($err:literal)) => {
                #[test]
                fn $name() {
                    let err = from_str::<List<$type>>($xml).unwrap_err();

                    match err {
                        DeError::$kind(e) => assert_eq!(e, $err),
                        _ => panic!(
                            "Expected `{}({})`, found `{:?}`",
                            stringify!($kind),
                            $err,
                            err
                        ),
                    }
                }
            };
        }

        /// Checks that sequences can be deserialized from an XML attribute content
        /// according to the `xs:list` XML Schema type
        mod attribute {
            use super::*;
            use pretty_assertions::assert_eq;

            #[derive(Debug, Deserialize, PartialEq)]
            struct List<T> {
                list: Vec<T>,
            }

            list!(i8_:  i8  = r#"<root list="1 -2  3"/>"# => vec![1, -2, 3]);
            list!(i16_: i16 = r#"<root list="1 -2  3"/>"# => vec![1, -2, 3]);
            list!(i32_: i32 = r#"<root list="1 -2  3"/>"# => vec![1, -2, 3]);
            list!(i64_: i64 = r#"<root list="1 -2  3"/>"# => vec![1, -2, 3]);

            list!(u8_:  u8  = r#"<root list="1 2  3"/>"# => vec![1, 2, 3]);
            list!(u16_: u16 = r#"<root list="1 2  3"/>"# => vec![1, 2, 3]);
            list!(u32_: u32 = r#"<root list="1 2  3"/>"# => vec![1, 2, 3]);
            list!(u64_: u64 = r#"<root list="1 2  3"/>"# => vec![1, 2, 3]);

            serde_if_integer128! {
                list!(i128_: i128 = r#"<root list="1 -2  3"/>"# => vec![1, -2, 3]);
                list!(u128_: u128 = r#"<root list="1 2  3"/>"# => vec![1, 2, 3]);
            }

            list!(f32_: f32 = r#"<root list="1.23 -4.56  7.89"/>"# => vec![1.23, -4.56, 7.89]);
            list!(f64_: f64 = r#"<root list="1.23 -4.56  7.89"/>"# => vec![1.23, -4.56, 7.89]);

            list!(bool_: bool = r#"<root list="true false  true"/>"# => vec![true, false, true]);
            list!(char_: char = r#"<root list="4 2  j"/>"# => vec!['4', '2', 'j']);

            list!(string: String = r#"<root list="first second  third&#x20;3"/>"# => vec![
                "first".to_string(),
                "second".to_string(),
                "third 3".to_string(),
            ]);
            err!(byte_buf: ByteBuf = r#"<root list="first second  third&#x20;3"/>"#
                 => Unsupported("byte arrays are not supported as `xs:list` items"));

            list!(unit: () = r#"<root list="1 second  false"/>"# => vec![(), (), ()]);
        }

        /// Checks that sequences can be deserialized from an XML text content
        /// according to the `xs:list` XML Schema type
        mod element {
            use super::*;

            #[derive(Debug, Deserialize, PartialEq)]
            struct List<T> {
                // Give it a special name that means text content of the XML node
                #[serde(rename = "$value")]
                list: Vec<T>,
            }

            mod text {
                use super::*;
                use pretty_assertions::assert_eq;

                list!(i8_:  i8  = "<root>1 -2  3</root>" => vec![1, -2, 3]);
                list!(i16_: i16 = "<root>1 -2  3</root>" => vec![1, -2, 3]);
                list!(i32_: i32 = "<root>1 -2  3</root>" => vec![1, -2, 3]);
                list!(i64_: i64 = "<root>1 -2  3</root>" => vec![1, -2, 3]);

                list!(u8_:  u8  = "<root>1 2  3</root>" => vec![1, 2, 3]);
                list!(u16_: u16 = "<root>1 2  3</root>" => vec![1, 2, 3]);
                list!(u32_: u32 = "<root>1 2  3</root>" => vec![1, 2, 3]);
                list!(u64_: u64 = "<root>1 2  3</root>" => vec![1, 2, 3]);

                serde_if_integer128! {
                    list!(i128_: i128 = "<root>1 -2  3</root>" => vec![1, -2, 3]);
                    list!(u128_: u128 = "<root>1 2  3</root>" => vec![1, 2, 3]);
                }

                list!(f32_: f32 = "<root>1.23 -4.56  7.89</root>" => vec![1.23, -4.56, 7.89]);
                list!(f64_: f64 = "<root>1.23 -4.56  7.89</root>" => vec![1.23, -4.56, 7.89]);

                list!(bool_: bool = "<root>true false  true</root>" => vec![true, false, true]);
                list!(char_: char = "<root>4 2  j</root>" => vec!['4', '2', 'j']);

                list!(string: String = "<root>first second  third&#x20;3</root>" => vec![
                    "first".to_string(),
                    "second".to_string(),
                    "third 3".to_string(),
                ]);
                err!(byte_buf: ByteBuf = "<root>first second  third&#x20;3</root>"
                    => Unsupported("byte arrays are not supported as `xs:list` items"));

                list!(unit: () = "<root>1 second  false</root>" => vec![(), (), ()]);
            }

            mod cdata {
                use super::*;
                use pretty_assertions::assert_eq;

                list!(i8_:  i8  = "<root><![CDATA[1 -2  3]]></root>" => vec![1, -2, 3]);
                list!(i16_: i16 = "<root><![CDATA[1 -2  3]]></root>" => vec![1, -2, 3]);
                list!(i32_: i32 = "<root><![CDATA[1 -2  3]]></root>" => vec![1, -2, 3]);
                list!(i64_: i64 = "<root><![CDATA[1 -2  3]]></root>" => vec![1, -2, 3]);

                list!(u8_:  u8  = "<root><![CDATA[1 2  3]]></root>" => vec![1, 2, 3]);
                list!(u16_: u16 = "<root><![CDATA[1 2  3]]></root>" => vec![1, 2, 3]);
                list!(u32_: u32 = "<root><![CDATA[1 2  3]]></root>" => vec![1, 2, 3]);
                list!(u64_: u64 = "<root><![CDATA[1 2  3]]></root>" => vec![1, 2, 3]);

                serde_if_integer128! {
                    list!(i128_: i128 = "<root><![CDATA[1 -2  3]]></root>" => vec![1, -2, 3]);
                    list!(u128_: u128 = "<root><![CDATA[1 2  3]]></root>" => vec![1, 2, 3]);
                }

                list!(f32_: f32 = "<root><![CDATA[1.23 -4.56  7.89]]></root>" => vec![1.23, -4.56, 7.89]);
                list!(f64_: f64 = "<root><![CDATA[1.23 -4.56  7.89]]></root>" => vec![1.23, -4.56, 7.89]);

                list!(bool_: bool = "<root><![CDATA[true false  true]]></root>" => vec![true, false, true]);
                list!(char_: char = "<root><![CDATA[4 2  j]]></root>" => vec!['4', '2', 'j']);

                // Cannot get whitespace in the value in any way if CDATA used:
                // - literal spaces means list item delimiters
                // - escaped sequences are not decoded in CDATA
                list!(string: String = "<root><![CDATA[first second  third&#x20;3]]></root>" => vec![
                    "first".to_string(),
                    "second".to_string(),
                    "third&#x20;3".to_string(),
                ]);
                err!(byte_buf: ByteBuf = "<root>first second  third&#x20;3</root>"
                    => Unsupported("byte arrays are not supported as `xs:list` items"));

                list!(unit: () = "<root>1 second  false</root>" => vec![(), (), ()]);
            }
        }
    }

    /// Ensures, that [`Deserializer::next_text()`] never can get an `End` event,
    /// because parser reports error early
    #[test]
    fn next_text() {
        match from_str::<String>(r#"</root>"#) {
            Err(DeError::InvalidXml(Error::EndEventMismatch { expected, found })) => {
                assert_eq!(expected, "");
                assert_eq!(found, "root");
            }
            x => panic!(
                r#"Expected `Err(InvalidXml(EndEventMismatch("", "root")))`, but found {:?}"#,
                x
            ),
        }

        let s: String = from_str(r#"<root></root>"#).unwrap();
        assert_eq!(s, "");

        match from_str::<String>(r#"<root></other>"#) {
            Err(DeError::InvalidXml(Error::EndEventMismatch { expected, found })) => {
                assert_eq!(expected, "root");
                assert_eq!(found, "other");
            }
            x => panic!(
                r#"Expected `Err(InvalidXml(EndEventMismatch("root", "other")))`, but found {:?}"#,
                x
            ),
        }
    }
}

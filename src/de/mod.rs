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

mod escape;
mod map;
mod seq;
mod var;

pub use crate::errors::serialize::DeError;
use crate::{
    events::{BytesStart, BytesText, Event},
    Reader,
};
use serde::de::{self, DeserializeOwned};
use serde::serde_if_integer128;
use std::io::BufRead;

const INNER_VALUE: &str = "$value";

/// An xml deserializer
pub struct Deserializer<R: BufRead> {
    reader: Reader<R>,
    peek: Option<Event<'static>>,
    has_value_field: bool,
}

/// Deserialize a xml string
pub fn from_str<T: DeserializeOwned>(s: &str) -> Result<T, DeError> {
    from_reader(s.as_bytes())
}

/// Deserialize from a reader
pub fn from_reader<R: BufRead, T: DeserializeOwned>(reader: R) -> Result<T, DeError> {
    let mut de = Deserializer::from_reader(reader);
    T::deserialize(&mut de)
}

impl<R: BufRead> Deserializer<R> {
    /// Get a new deserializer
    pub fn new(reader: Reader<R>) -> Self {
        Deserializer {
            reader,
            peek: None,
            has_value_field: false,
        }
    }

    /// Get a new deserializer from a regular BufRead
    pub fn from_reader(reader: R) -> Self {
        let mut reader = Reader::from_reader(reader);
        reader
            .expand_empty_elements(true)
            .check_end_names(true)
            .trim_text(true);
        Self::new(reader)
    }

    fn peek(&mut self) -> Result<Option<&Event<'static>>, DeError> {
        if self.peek.is_none() {
            self.peek = Some(self.next(&mut Vec::new())?);
        }
        Ok(self.peek.as_ref())
    }

    fn next<'a>(&mut self, buf: &'a mut Vec<u8>) -> Result<Event<'static>, DeError> {
        if let Some(e) = self.peek.take() {
            return Ok(e);
        }
        loop {
            let e = self.reader.read_event(buf)?;
            match e {
                Event::Start(_) | Event::End(_) | Event::Text(_) | Event::Eof | Event::CData(_) => {
                    return Ok(e.into_owned())
                }
                _ => buf.clear(),
            }
        }
    }

    fn next_start(&mut self, buf: &mut Vec<u8>) -> Result<Option<BytesStart<'static>>, DeError> {
        loop {
            let e = self.next(buf)?;
            match e {
                Event::Start(e) => return Ok(Some(e)),
                Event::End(_) => return Err(DeError::End),
                Event::Eof => return Ok(None),
                _ => buf.clear(), // ignore texts
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
    fn next_text<'a>(&mut self) -> Result<BytesText<'static>, DeError> {
        match self.next(&mut Vec::new())? {
            Event::Text(e) | Event::CData(e) => Ok(e),
            Event::Eof => Err(DeError::Eof),
            Event::Start(e) => {
                // allow one nested level
                let inner = self.next(&mut Vec::new())?;
                let t = match inner {
                    Event::Text(t) | Event::CData(t) => t,
                    Event::Start(_) => return Err(DeError::Start),
                    Event::End(end) if end.name() == e.name() => {
                        return Ok(BytesText::from_escaped(&[] as &[u8]));
                    }
                    Event::End(_) => return Err(DeError::End),
                    Event::Eof => return Err(DeError::Eof),
                    _ => unreachable!(),
                };
                self.read_to_end(e.name())?;
                Ok(t)
            }
            Event::End(e) => {
                self.peek = Some(Event::End(e));
                Ok(BytesText::from_escaped(&[] as &[u8]))
            }
            _ => unreachable!(),
        }
    }

    fn read_to_end(&mut self, name: &[u8]) -> Result<(), DeError> {
        let mut buf = Vec::new();
        match self.next(&mut buf)? {
            Event::Start(e) => self.reader.read_to_end(e.name(), &mut Vec::new())?,
            Event::End(e) if e.name() == name => return Ok(()),
            _ => buf.clear(),
        }
        Ok(self.reader.read_to_end(name, &mut buf)?)
    }
}

macro_rules! deserialize_type {
    ($deserialize:ident => $visit:ident) => {
        fn $deserialize<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
            let txt = self.next_text()?;

            #[cfg(not(feature = "encoding"))]
            let value = self.reader.decode(&*txt)?.parse()?;

            #[cfg(feature = "encoding")]
            let value = self.reader.decode(&*txt).parse()?;

            visitor.$visit(value)
        }
    };
}

impl<'de, 'a, R: BufRead> de::Deserializer<'de> for &'a mut Deserializer<R> {
    type Error = DeError;

    fn deserialize_struct<V: de::Visitor<'de>>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, DeError> {
        if let Some(e) = self.next_start(&mut Vec::new())? {
            let name = e.name().to_vec();
            self.has_value_field = fields.contains(&INNER_VALUE);
            let map = map::MapAccess::new(self, e)?;
            let value = visitor.visit_map(map)?;
            self.has_value_field = false;
            self.read_to_end(&name)?;
            Ok(value)
        } else {
            Err(DeError::Start)
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

    fn deserialize_bool<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
        let txt = self.next_text()?;

        #[cfg(feature = "encoding")]
        {
            #[cfg(feature = "encoding")]
            let value = self.reader.decode(&*txt);

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
            match txt.as_ref() {
                b"true" | b"1" | b"True" | b"TRUE" | b"t" | b"Yes" | b"YES" | b"yes" | b"y" => {
                    visitor.visit_bool(true)
                }
                b"false" | b"0" | b"False" | b"FALSE" | b"f" | b"No" | b"NO" | b"no" | b"n" => {
                    visitor.visit_bool(false)
                }
                e => Err(DeError::InvalidBoolean(self.reader.decode(e)?.into())),
            }
        }
    }

    fn deserialize_string<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
        let value = self.next_text()?.unescape_and_decode(&self.reader)?;
        visitor.visit_string(value)
    }

    fn deserialize_char<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
        self.deserialize_string(visitor)
    }

    fn deserialize_str<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
        self.deserialize_string(visitor)
    }

    fn deserialize_bytes<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
        let text = self.next_text()?;
        let value = text.escaped();
        visitor.visit_bytes(value)
    }

    fn deserialize_byte_buf<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
        let text = self.next_text()?;
        let value = text.into_inner().into_owned();
        visitor.visit_byte_buf(value)
    }

    fn deserialize_unit<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
        let mut buf = Vec::new();
        match self.next(&mut buf)? {
            Event::Start(s) => {
                self.read_to_end(s.name())?;
                visitor.visit_unit()
            }
            e => Err(DeError::InvalidUnit(format!("{:?}", e))),
        }
    }

    fn deserialize_unit_struct<V: de::Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, DeError> {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V: de::Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, DeError> {
        self.deserialize_tuple(1, visitor)
    }

    fn deserialize_tuple<V: de::Visitor<'de>>(
        self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, DeError> {
        visitor.visit_seq(seq::SeqAccess::new(self, Some(len))?)
    }

    fn deserialize_tuple_struct<V: de::Visitor<'de>>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, DeError> {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_enum<V: de::Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, DeError> {
        let value = visitor.visit_enum(var::EnumAccess::new(self))?;
        Ok(value)
    }

    fn deserialize_seq<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
        visitor.visit_seq(seq::SeqAccess::new(self, None)?)
    }

    fn deserialize_map<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
        self.deserialize_struct("", &[], visitor)
    }

    fn deserialize_option<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
        match self.peek()? {
            Some(Event::Text(t)) if t.is_empty() => visitor.visit_none(),
            None | Some(Event::Eof) => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_identifier<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
        self.deserialize_string(visitor)
    }

    fn deserialize_ignored_any<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
        match self.next(&mut Vec::new())? {
            Event::Start(e) => self.read_to_end(e.name())?,
            Event::End(_) => return Err(DeError::End),
            _ => (),
        }
        visitor.visit_unit()
    }

    fn deserialize_any<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
        match self.peek()?.ok_or(DeError::Eof)? {
            Event::Start(_) => self.deserialize_map(visitor),
            Event::End(_) => self.deserialize_unit(visitor),
            _ => self.deserialize_string(visitor),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Item {
        name: String,
        source: String,
    }

    #[test]
    fn simple_struct_from_attributes() {
        let s = r##"
	    <item name="hello" source="world.rs" />
	"##;

        let item: Item = from_str(s).unwrap();

        assert_eq!(
            item,
            Item {
                name: "hello".to_string(),
                source: "world.rs".to_string(),
            }
        );
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

    #[test]
    fn simple_struct_from_attribute_and_child() {
        let s = r##"
	    <item name="hello">
            <source>world.rs</source>
            </item>
        "##;

        let item: Item = from_str(s).unwrap();

        assert_eq!(
            item,
            Item {
                name: "hello".to_string(),
                source: "world.rs".to_string(),
            }
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
        #[derive(Debug, PartialEq)]
        struct Item {
            bytes: Vec<u8>,
        }

        impl<'de> Deserialize<'de> for Item {
            fn deserialize<D>(d: D) -> Result<Self, D::Error>
            where
                D: serde::de::Deserializer<'de>,
            {
                struct ItemVisitor;

                impl<'de> de::Visitor<'de> for ItemVisitor {
                    type Value = Item;

                    fn expecting(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
                        fmt.write_str("byte data")
                    }

                    fn visit_byte_buf<E: de::Error>(self, v: Vec<u8>) -> Result<Self::Value, E> {
                        Ok(Item { bytes: v })
                    }
                }

                Ok(d.deserialize_byte_buf(ItemVisitor)?)
            }
        }

        let s = r#"<item>bytes</item>"#;
        let item: Item = from_reader(s.as_bytes()).unwrap();

        assert_eq!(
            item,
            Item {
                bytes: "bytes".as_bytes().to_vec(),
            }
        );
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

    #[test]
    fn unit() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Unit;

        let data: Unit = from_str("<root/>").unwrap();
        assert_eq!(data, Unit);
    }

    #[test]
    fn newtype() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Newtype(bool);

        let data: Newtype = from_str("<root>true</root>").unwrap();
        assert_eq!(data, Newtype(true));
    }

    #[test]
    fn tuple() {
        let data: (f32, String) = from_str("<root>42</root><root>answer</root>").unwrap();
        assert_eq!(data, (42.0, "answer".into()));
    }

    #[test]
    fn tuple_struct() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Tuple(f32, String);

        let data: Tuple = from_str("<root>42</root><root>answer</root>").unwrap();
        assert_eq!(data, Tuple(42.0, "answer".into()));
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

//! Module to handle custom serde `Serializer`

/// Implements writing primitives to the underlying writer.
/// Implementor must provide `write_str(self, &str) -> Result<(), DeError>` method
macro_rules! write_primitive {
    ($method:ident ( $ty:ty )) => {
        fn $method(mut self, value: $ty) -> Result<Self::Ok, Self::Error> {
            self.write_str(&value.to_string())?;
            Ok(self.writer)
        }
    };
    () => {
        fn serialize_bool(mut self, value: bool) -> Result<Self::Ok, Self::Error> {
            self.write_str(if value { "true" } else { "false" })?;
            Ok(self.writer)
        }

        write_primitive!(serialize_i8(i8));
        write_primitive!(serialize_i16(i16));
        write_primitive!(serialize_i32(i32));
        write_primitive!(serialize_i64(i64));

        write_primitive!(serialize_u8(u8));
        write_primitive!(serialize_u16(u16));
        write_primitive!(serialize_u32(u32));
        write_primitive!(serialize_u64(u64));

        serde_if_integer128! {
            write_primitive!(serialize_i128(i128));
            write_primitive!(serialize_u128(u128));
        }

        write_primitive!(serialize_f32(f32));
        write_primitive!(serialize_f64(f64));

        fn serialize_char(self, value: char) -> Result<Self::Ok, Self::Error> {
            self.serialize_str(&value.to_string())
        }

        fn serialize_bytes(self, _value: &[u8]) -> Result<Self::Ok, Self::Error> {
            //TODO: customization point - allow user to decide how to encode bytes
            Err(DeError::Unsupported(
                "`serialize_bytes` not supported yet".into(),
            ))
        }

        fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
            Ok(self.writer)
        }

        fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, Self::Error> {
            value.serialize(self)
        }

        fn serialize_unit_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
        ) -> Result<Self::Ok, Self::Error> {
            self.serialize_str(variant)
        }

        fn serialize_newtype_struct<T: ?Sized + Serialize>(
            self,
            _name: &'static str,
            value: &T,
        ) -> Result<Self::Ok, Self::Error> {
            value.serialize(self)
        }
    };
}

////////////////////////////////////////////////////////////////////////////////////////////////////

mod content;
mod element;
mod key;
pub(crate) mod simple_type;
mod var;

use self::var::{Map, Seq, Struct, Tuple};
use crate::{
    de::PRIMITIVE_PREFIX,
    errors::serialize::DeError,
    events::{BytesEnd, BytesStart, BytesText, Event},
    writer::Writer,
};
use serde::ser::{self, Serialize};
use serde::serde_if_integer128;
use std::io::Write;

/// Serialize struct into a `Write`r
pub fn to_writer<W: Write, S: Serialize>(writer: W, value: &S) -> Result<(), DeError> {
    let mut serializer = Serializer::new(writer);
    value.serialize(&mut serializer)
}

/// Serialize struct into a `String`
pub fn to_string<S: Serialize>(value: &S) -> Result<String, DeError> {
    let mut writer = Vec::new();
    to_writer(&mut writer, value)?;
    Ok(String::from_utf8(writer)?)
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// Defines which characters would be escaped in [`Text`] events and attribute
/// values.
///
/// [`Text`]: Event::Text
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteLevel {
    /// Performs escaping, escape all characters that could have special meaning
    /// in the XML. This mode is compatible with SGML specification.
    ///
    /// Characters that will be replaced:
    ///
    /// Original | Replacement
    /// ---------|------------
    /// `<`      | `&lt;`
    /// `>`      | `&gt;`
    /// `&`      | `&amp;`
    /// `"`      | `&quot;`
    /// `'`      | `&apos;`
    Full,
    /// Performs escaping that is compatible with SGML specification.
    ///
    /// This level adds escaping of `>` to the `Minimal` level, which is [required]
    /// for compatibility with SGML.
    ///
    /// Characters that will be replaced:
    ///
    /// Original | Replacement
    /// ---------|------------
    /// `<`      | `&lt;`
    /// `>`      | `&gt;`
    /// `&`      | `&amp;`
    ///
    /// [required]: https://www.w3.org/TR/xml11/#syntax
    Partial,
    /// Performs the minimal possible escaping, escape only strictly necessary
    /// characters.
    ///
    /// Characters that will be replaced:
    ///
    /// Original | Replacement
    /// ---------|------------
    /// `<`      | `&lt;`
    /// `&`      | `&amp;`
    Minimal,
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// Almost all characters can form a name. Citation from <https://www.w3.org/TR/xml11/#sec-xml11>:
///
/// > The overall philosophy of names has changed since XML 1.0. Whereas XML 1.0
/// > provided a rigid definition of names, wherein everything that was not permitted
/// > was forbidden, XML 1.1 names are designed so that everything that is not
/// > forbidden (for a specific reason) is permitted. Since Unicode will continue
/// > to grow past version 4.0, further changes to XML can be avoided by allowing
/// > almost any character, including those not yet assigned, in names.
///
/// <https://www.w3.org/TR/xml11/#NT-NameStartChar>
const fn is_xml11_name_start_char(ch: char) -> bool {
    match ch {
        ':'
        | 'A'..='Z'
        | '_'
        | 'a'..='z'
        | '\u{00C0}'..='\u{00D6}'
        | '\u{00D8}'..='\u{00F6}'
        | '\u{00F8}'..='\u{02FF}'
        | '\u{0370}'..='\u{037D}'
        | '\u{037F}'..='\u{1FFF}'
        | '\u{200C}'..='\u{200D}'
        | '\u{2070}'..='\u{218F}'
        | '\u{2C00}'..='\u{2FEF}'
        | '\u{3001}'..='\u{D7FF}'
        | '\u{F900}'..='\u{FDCF}'
        | '\u{FDF0}'..='\u{FFFD}'
        | '\u{10000}'..='\u{EFFFF}' => true,
        _ => false,
    }
}
/// <https://www.w3.org/TR/xml11/#NT-NameChar>
const fn is_xml11_name_char(ch: char) -> bool {
    match ch {
        '-' | '.' | '0'..='9' | '\u{00B7}' | '\u{0300}'..='\u{036F}' | '\u{203F}'..='\u{2040}' => {
            true
        }
        _ => is_xml11_name_start_char(ch),
    }
}

/// Helper struct to self-defense from errors
#[derive(Clone, Copy, Debug, PartialEq)]
pub(self) struct XmlName<'n>(&'n str);

impl<'n> XmlName<'n> {
    /// Checks correctness of the XML name according to [XML 1.1 specification]
    ///
    /// [XML 1.1 specification]: https://www.w3.org/TR/REC-xml/#NT-Name
    pub fn try_from(name: &'n str) -> Result<XmlName<'n>, DeError> {
        //TODO: Customization point: allow user to decide if he want to reject or encode the name
        match name.chars().next() {
            Some(ch) if !is_xml11_name_start_char(ch) => Err(DeError::Unsupported(
                format!("character `{ch}` is not allowed at the start of an XML name `{name}`")
                    .into(),
            )),
            _ => match name.matches(|ch| !is_xml11_name_char(ch)).next() {
                Some(s) => Err(DeError::Unsupported(
                    format!("character `{s}` is not allowed in an XML name `{name}`").into(),
                )),
                None => Ok(XmlName(name)),
            },
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// A Serializer
pub struct Serializer<'r, W: Write> {
    writer: Writer<W>,
    /// Name of the root tag. If not specified, deduced from the structure name
    root_tag: Option<&'r str>,
}

impl<'r, W: Write> Serializer<'r, W> {
    /// Creates a new `Serializer` that uses struct name as a root tag name.
    ///
    /// Note, that attempt to serialize a non-struct (including unit structs
    /// and newtype structs) will end up to an error. Use `with_root` to create
    /// serializer with explicitly defined root element name
    pub fn new(writer: W) -> Self {
        Self::with_root(Writer::new(writer), None)
    }

    /// Creates a new `Serializer` that uses specified root tag name
    ///
    /// # Examples
    ///
    /// When serializing a primitive type, only its representation will be written:
    ///
    /// ```
    /// # use pretty_assertions::assert_eq;
    /// # use serde::Serialize;
    /// # use quick_xml::se::Serializer;
    /// use quick_xml::writer::Writer;
    ///
    /// let mut buffer = Vec::new();
    /// let mut writer = Writer::new_with_indent(&mut buffer, b' ', 2);
    /// let mut ser = Serializer::with_root(writer, Some("root"));
    ///
    /// "node".serialize(&mut ser).unwrap();
    /// assert_eq!(String::from_utf8(buffer).unwrap(), "node");
    /// ```
    ///
    /// When serializing a struct, newtype struct, unit struct or tuple `root_tag`
    /// is used as tag name of root(s) element(s):
    ///
    /// ```
    /// # use pretty_assertions::assert_eq;
    /// # use serde::Serialize;
    /// use quick_xml::se::Serializer;
    /// use quick_xml::writer::Writer;
    ///
    /// #[derive(Debug, PartialEq, Serialize)]
    /// struct Struct {
    ///     question: String,
    ///     answer: u32,
    /// }
    ///
    /// let mut buffer = Vec::new();
    /// let mut writer = Writer::new_with_indent(&mut buffer, b' ', 2);
    /// let mut ser = Serializer::with_root(writer, Some("root"));
    ///
    /// Struct {
    ///     question: "The Ultimate Question of Life, the Universe, and Everything".into(),
    ///     answer: 42,
    /// }.serialize(&mut ser).unwrap();
    /// assert_eq!(
    ///     String::from_utf8(buffer.clone()).unwrap(),
    ///     r#"<root question="The Ultimate Question of Life, the Universe, and Everything" answer="42"/>"#
    /// );
    /// ```
    pub fn with_root(writer: Writer<W>, root_tag: Option<&'r str>) -> Self {
        Self { writer, root_tag }
    }

    fn write_primitive<P: std::fmt::Display>(
        &mut self,
        value: P,
        escaped: bool,
    ) -> Result<(), DeError> {
        let value = value.to_string();
        let event = if escaped {
            BytesText::from_escaped(&value)
        } else {
            BytesText::new(&value)
        };
        self.writer.write_event(Event::Text(event))?;
        Ok(())
    }

    /// Writes self-closed tag `<tag_name/>` into inner writer
    fn write_self_closed(&mut self, tag_name: &str) -> Result<(), DeError> {
        self.writer
            .write_event(Event::Empty(BytesStart::new(tag_name)))?;
        Ok(())
    }

    /// Writes a serialized `value` surrounded by `<tag_name>...</tag_name>`
    fn write_paired<T: ?Sized + Serialize>(
        &mut self,
        tag_name: &str,
        value: &T,
    ) -> Result<(), DeError> {
        self.writer
            .write_event(Event::Start(BytesStart::new(tag_name)))?;
        value.serialize(&mut *self)?;
        self.writer
            .write_event(Event::End(BytesEnd::new(tag_name)))?;
        Ok(())
    }
}

impl<'r, 'w, W: Write> ser::Serializer for &'w mut Serializer<'r, W> {
    type Ok = ();
    type Error = DeError;

    type SerializeSeq = Seq<'r, 'w, W>;
    type SerializeTuple = Tuple<'r, 'w, W>;
    type SerializeTupleStruct = Tuple<'r, 'w, W>;
    type SerializeTupleVariant = Tuple<'r, 'w, W>;
    type SerializeMap = Map<'r, 'w, W>;
    type SerializeStruct = Struct<'r, 'w, W>;
    type SerializeStructVariant = Struct<'r, 'w, W>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, DeError> {
        self.write_primitive(if v { "true" } else { "false" }, true)
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, DeError> {
        self.write_primitive(v, true)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, DeError> {
        self.write_primitive(v, true)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, DeError> {
        self.write_primitive(v, true)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, DeError> {
        self.write_primitive(v, true)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, DeError> {
        self.write_primitive(v, true)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, DeError> {
        self.write_primitive(v, true)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, DeError> {
        self.write_primitive(v, true)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, DeError> {
        self.write_primitive(v, true)
    }

    serde_if_integer128! {
        fn serialize_i128(self, v: i128) -> Result<Self::Ok, DeError> {
            self.write_primitive(v, true)
        }

        fn serialize_u128(self, v: u128) -> Result<Self::Ok, DeError> {
            self.write_primitive(v, true)
        }
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, DeError> {
        self.write_primitive(v, true)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, DeError> {
        self.write_primitive(v, true)
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, DeError> {
        self.write_primitive(v, false)
    }

    fn serialize_str(self, value: &str) -> Result<Self::Ok, DeError> {
        self.write_primitive(value, false)
    }

    fn serialize_bytes(self, _value: &[u8]) -> Result<Self::Ok, DeError> {
        // TODO: I imagine you'd want to use base64 here.
        // Not sure how to roundtrip effectively though...
        Err(DeError::Unsupported(
            "`serialize_bytes` not supported yet".into(),
        ))
    }

    fn serialize_none(self) -> Result<Self::Ok, DeError> {
        Ok(())
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, DeError> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, DeError> {
        self.serialize_none()
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, DeError> {
        self.write_self_closed(self.root_tag.unwrap_or(name))
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, DeError> {
        if variant.starts_with(PRIMITIVE_PREFIX) {
            let variant = variant.split_at(PRIMITIVE_PREFIX.len()).1;
            self.write_primitive(variant, false)
        } else {
            self.write_self_closed(variant)
        }
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, DeError> {
        self.write_paired(self.root_tag.unwrap_or(name), value)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, DeError> {
        // Flatten structs in enums are serialized as newtype struct variant + map.
        // As serialize_map should write `root_tag` for ordinal maps (because it's
        // only way for maps), and for enums this method already written a tag name
        // (`variant`), we need to clear root tag before writing content and restore
        // it after
        let root = self.root_tag.take();
        let result = self.write_paired(variant, value);
        self.root_tag = root;
        result
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, DeError> {
        Ok(Seq::new(self))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, DeError> {
        let tag = match self.root_tag {
            Some(tag) => tag,
            None => {
                return Err(DeError::Custom(
                    "root tag name must be specified when serialize unnamed tuple".into(),
                ))
            }
        };
        Ok(Tuple::new(self, tag))
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, DeError> {
        Ok(Tuple::new(self, self.root_tag.unwrap_or(name)))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, DeError> {
        Ok(Tuple::new(self, variant))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, DeError> {
        if let Some(tag) = self.root_tag {
            // TODO: Write self-closed tag if map is empty
            self.writer
                .write_event(Event::Start(BytesStart::new(tag)))?;
        }
        Ok(Map::new(self))
    }

    fn serialize_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, DeError> {
        Ok(Struct::new(self, self.root_tag.unwrap_or(name)))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, DeError> {
        Ok(Struct::new(self, variant))
    }
}

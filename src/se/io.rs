#![allow(missing_docs)]
//! Serializers to an std::io output stream.

mod content;
mod element;
mod simple_type;
mod text;

use content::*;
use element::*;
use simple_type::*;
use text::*;

use ref_cast::RefCast;
use std::str::from_utf8;

use serde::ser::{
    self, Impossible, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant,
    SerializeTuple, SerializeTupleStruct, SerializeTupleVariant,
};
use serde::{serde_if_integer128, Serialize};

use crate::de::VALUE_KEY;
use crate::{de::TEXT_KEY, writer::Indentation, DeError};

use super::{simple_type::QuoteTarget, Indent, QuoteLevel, XmlName};

/// Wrapper for a std::io::Write writer that also implements std::fmt::Write for
/// compatibility with original serializers that work only with
/// std::fmt::Write writers.
#[derive(RefCast)]
#[repr(transparent)]
pub(crate) struct FmtWriter<W> {
    pub(crate) writer: W,
}

impl<W: std::io::Write> std::fmt::Write for FmtWriter<W> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.writer
            .write(s.as_bytes())
            .map_err(|_| std::fmt::Error)?;
        Ok(())
    }
    fn write_char(&mut self, c: char) -> std::fmt::Result {
        std::fmt::Write::write_str(self, c.encode_utf8(&mut [0; 4]))
    }
}

pub trait Write: std::io::Write {
    fn write_str(&mut self, s: &str) -> Result<(), DeError> {
        self.write(s.as_bytes())?;
        Ok(())
    }
    fn write_char(&mut self, c: char) -> Result<(), DeError> {
        self.write_str(c.encode_utf8(&mut [0; 4]))
    }
}

impl<W: std::io::Write> Write for W {}

impl<'i> Indent<'i> {
    pub fn write_io_indent<W: Write>(&mut self, mut writer: W) -> Result<(), DeError> {
        match self {
            Self::None => {}
            Self::Owned(i) => {
                writer.write_char('\n')?;
                writer.write_str(from_utf8(i.current())?)?;
            }
            Self::Borrow(i) => {
                writer.write_char('\n')?;
                writer.write_str(from_utf8(i.current())?)?;
            }
        }
        Ok(())
    }
}

/// Implements serialization method by forwarding it to the serializer created by
/// the helper method [`Serializer::ser`].
macro_rules! forward {
    ($name:ident($ty:ty)) => {
        fn $name(self, value: $ty) -> Result<Self::Ok, Self::Error> {
            self.ser(&concat!("`", stringify!($ty), "`"))?.$name(value)
        }
    };
}

/// A Serializer
pub struct Serializer<'w, 'r, W> {
    ser: ContentSerializer<'w, 'r, W>,
    /// Name of the root tag. If not specified, deduced from the structure name
    root_tag: Option<XmlName<'r>>,
}

impl<'w, 'r, W: std::io::Write> Serializer<'w, 'r, W> {
    /// Creates a new `Serializer` that uses struct name as a root tag name.
    ///
    /// Note, that attempt to serialize a non-struct (including unit structs
    /// and newtype structs) will end up to an error. Use `with_root` to create
    /// serializer with explicitly defined root element name
    pub fn new(writer: &'w mut W) -> Self {
        Self {
            ser: ContentSerializer {
                writer,
                level: QuoteLevel::Partial,
                indent: Indent::None,
                write_indent: false,
                expand_empty_elements: false,
            },
            root_tag: None,
        }
    }

    /// Creates a new `Serializer` that uses specified root tag name. `name` should
    /// be valid [XML name], otherwise error is returned.
    ///
    /// # Examples
    ///
    /// When serializing a primitive type, only its representation will be written:
    ///
    /// ```
    /// # use pretty_assertions::assert_eq;
    /// # use serde::Serialize;
    /// # use quick_xml::se::Serializer;
    ///
    /// let mut buffer = String::new();
    /// let ser = Serializer::with_root(&mut buffer, Some("root")).unwrap();
    ///
    /// "node".serialize(ser).unwrap();
    /// assert_eq!(buffer, "<root>node</root>");
    /// ```
    ///
    /// When serializing a struct, newtype struct, unit struct or tuple `root_tag`
    /// is used as tag name of root(s) element(s):
    ///
    /// ```
    /// # use pretty_assertions::assert_eq;
    /// # use serde::Serialize;
    /// # use quick_xml::se::Serializer;
    ///
    /// #[derive(Debug, PartialEq, Serialize)]
    /// struct Struct {
    ///     question: String,
    ///     answer: u32,
    /// }
    ///
    /// let mut buffer = String::new();
    /// let ser = Serializer::with_root(&mut buffer, Some("root")).unwrap();
    ///
    /// let data = Struct {
    ///     question: "The Ultimate Question of Life, the Universe, and Everything".into(),
    ///     answer: 42,
    /// };
    ///
    /// data.serialize(ser).unwrap();
    /// assert_eq!(
    ///     buffer,
    ///     "<root>\
    ///         <question>The Ultimate Question of Life, the Universe, and Everything</question>\
    ///         <answer>42</answer>\
    ///      </root>"
    /// );
    /// ```
    ///
    /// [XML name]: https://www.w3.org/TR/xml11/#NT-Name
    pub fn with_root(writer: &'w mut W, root_tag: Option<&'r str>) -> Result<Self, DeError> {
        Ok(Self {
            ser: ContentSerializer {
                writer,
                level: QuoteLevel::Partial,
                indent: Indent::None,
                write_indent: false,
                expand_empty_elements: false,
            },
            root_tag: root_tag.map(|tag| XmlName::try_from(tag)).transpose()?,
        })
    }

    /// Enable or disable expansion of empty elements. Defaults to `false`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use pretty_assertions::assert_eq;
    /// # use serde::Serialize;
    /// # use quick_xml::se::Serializer;
    ///
    /// #[derive(Debug, PartialEq, Serialize)]
    /// struct Struct {
    ///     question: Option<String>,
    /// }
    ///
    /// let mut buffer = String::new();
    /// let mut ser = Serializer::new(&mut buffer);
    /// ser.expand_empty_elements(true);
    ///
    /// let data = Struct {
    ///   question: None,
    /// };
    ///
    /// data.serialize(ser).unwrap();
    /// assert_eq!(
    ///     buffer,
    ///     "<Struct><question></question></Struct>"
    /// );
    /// ```
    pub fn expand_empty_elements(&mut self, expand: bool) -> &mut Self {
        self.ser.expand_empty_elements = expand;
        self
    }

    /// Configure indent for a serializer
    pub fn indent(&mut self, indent_char: char, indent_size: usize) -> &mut Self {
        self.ser.indent = Indent::Owned(Indentation::new(indent_char as u8, indent_size));
        self
    }

    /// Set the level of quoting used when writing texts
    ///
    /// Default: [`QuoteLevel::Minimal`]
    pub fn set_quote_level(&mut self, level: QuoteLevel) -> &mut Self {
        self.ser.level = level;
        self
    }

    /// Creates actual serializer or returns an error if root tag is not defined.
    /// In that case `err` contains the name of type that cannot be serialized.
    fn ser(self, err: &str) -> Result<ElementSerializer<'w, 'r, W>, DeError> {
        if let Some(key) = self.root_tag {
            Ok(ElementSerializer { ser: self.ser, key })
        } else {
            Err(DeError::Unsupported(
                format!("cannot serialize {} without defined root tag", err).into(),
            ))
        }
    }

    /// Creates actual serializer using root tag or a specified `key` if root tag
    /// is not defined. Returns an error if root tag is not defined and a `key`
    /// does not conform [XML rules](XmlName::try_from) for names.
    fn ser_name(self, key: &'static str) -> Result<ElementSerializer<'w, 'r, W>, DeError> {
        Ok(self.ser.into_element_serializer(match self.root_tag {
            Some(key) => key,
            None => XmlName::try_from(key)?,
        }))
    }

    /// Get writer.
    pub fn get_mut(&mut self) -> &mut W {
        self.ser.writer
    }
}

impl<'w, 'r, W: std::io::Write> ser::Serializer for Serializer<'w, 'r, W> {
    type Ok = ();
    type Error = DeError;

    type SerializeSeq = ElementSerializer<'w, 'r, W>;
    type SerializeTuple = ElementSerializer<'w, 'r, W>;
    type SerializeTupleStruct = ElementSerializer<'w, 'r, W>;
    type SerializeTupleVariant = Tuple<'w, 'r, W>;
    type SerializeMap = Map<'w, 'r, W>;
    type SerializeStruct = Struct<'w, 'r, W>;
    type SerializeStructVariant = Struct<'w, 'r, W>;

    forward!(serialize_bool(bool));

    forward!(serialize_i8(i8));
    forward!(serialize_i16(i16));
    forward!(serialize_i32(i32));
    forward!(serialize_i64(i64));

    forward!(serialize_u8(u8));
    forward!(serialize_u16(u16));
    forward!(serialize_u32(u32));
    forward!(serialize_u64(u64));

    serde_if_integer128! {
        forward!(serialize_i128(i128));
        forward!(serialize_u128(u128));
    }

    forward!(serialize_f32(f32));
    forward!(serialize_f64(f64));

    forward!(serialize_char(char));
    forward!(serialize_str(&str));
    forward!(serialize_bytes(&[u8]));

    fn serialize_none(self) -> Result<Self::Ok, DeError> {
        Ok(())
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, DeError> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, DeError> {
        self.ser("`()`")?.serialize_unit()
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, DeError> {
        self.ser_name(name)?.serialize_unit_struct(name)
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, DeError> {
        if variant == TEXT_KEY {
            // We should write some text but we don't known what text to write
            Err(DeError::Unsupported(
                format!(
                    "cannot serialize enum unit variant `{}::$text` as text content value",
                    name
                )
                .into(),
            ))
        } else {
            let name = XmlName::try_from(variant)?;
            self.ser.write_empty(name)
        }
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, DeError> {
        self.ser_name(name)?.serialize_newtype_struct(name, value)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, DeError> {
        if variant == TEXT_KEY {
            value.serialize(self.ser.into_simple_type_serializer())?;
            Ok(())
        } else {
            value.serialize(self.ser.try_into_element_serializer(variant)?)
        }
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, DeError> {
        self.ser("sequence")?.serialize_seq(len)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, DeError> {
        self.ser("unnamed tuple")?.serialize_tuple(len)
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, DeError> {
        self.ser_name(name)?.serialize_tuple_struct(name, len)
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, DeError> {
        if variant == TEXT_KEY {
            self.ser
                .into_simple_type_serializer()
                .serialize_tuple_struct(name, len)
                .map(Tuple::Text)
        } else {
            let ser = self.ser.try_into_element_serializer(variant)?;
            ser.serialize_tuple_struct(name, len).map(Tuple::Element)
        }
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, DeError> {
        self.ser("map")?.serialize_map(len)
    }

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, DeError> {
        self.ser_name(name)?.serialize_struct(name, len)
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, DeError> {
        if variant == TEXT_KEY {
            Err(DeError::Unsupported(
                format!(
                    "cannot serialize enum struct variant `{}::$text` as text content value",
                    name
                )
                .into(),
            ))
        } else {
            let ser = self.ser.try_into_element_serializer(variant)?;
            ser.serialize_struct(name, len)
        }
    }
}

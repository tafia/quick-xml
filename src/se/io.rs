#![allow(missing_docs)]
//! Serializers to an std::io output stream.

use std::str::from_utf8;

use serde::ser::{
    self, Impossible, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant,
    SerializeTuple, SerializeTupleStruct, SerializeTupleVariant,
};
use serde::{serde_if_integer128, Serialize};

use crate::de::VALUE_KEY;
use crate::{de::TEXT_KEY, writer::Indentation, DeError};

use super::{simple_type::QuoteTarget, Indent, QuoteLevel, XmlName};

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

        fn serialize_bytes(mut self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
            self.writer.write(&value)?;
            Ok(self.writer)
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

pub trait Write: std::io::Write {
    fn write_str(&mut self, s: &str) -> Result<(), DeError> {
        self.write(s.as_bytes())?;
        Ok(())
    }
    fn write_char(&mut self, c: char) -> Result<(), DeError> {
        self.write_str(c.encode_utf8(&mut [0; 4]))
    }
    // fn write_fmt(&mut self, args: Arguments<'_>) -> Result {
    //     // We use a specialization for `Sized` types to avoid an indirection
    //     // through `&mut self`
    //     trait SpecWriteFmt {
    //         fn spec_write_fmt(self, args: Arguments<'_>) -> Result;
    //     }

    //     impl<W: Write + ?Sized> SpecWriteFmt for &mut W {
    //         #[inline]
    //         default fn spec_write_fmt(mut self, args: Arguments<'_>) -> Result {
    //             if let Some(s) = args.as_statically_known_str() {
    //                 self.write_str(s)
    //             } else {
    //                 write(&mut self, args)
    //             }
    //         }
    //     }

    //     impl<W: Write> SpecWriteFmt for &mut W {
    //         #[inline]
    //         fn spec_write_fmt(self, args: Arguments<'_>) -> Result {
    //             if let Some(s) = args.as_statically_known_str() {
    //                 self.write_str(s)
    //             } else {
    //                 write(self, args)
    //             }
    //         }
    //     }

    //     self.spec_write_fmt(args)
    // }
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

/// An IO Serializer
pub struct Serializer<'w, 'r, W: Write> {
    ser: ContentSerializer<'w, 'r, W>,
    /// Name of the root tag. If not specified, deduced from the structure name
    root_tag: Option<XmlName<'r>>,
}

impl<'w, 'r, W: Write> Serializer<'w, 'r, W> {
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
        Ok(ElementSerializer {
            ser: self.ser,
            key: match self.root_tag {
                Some(key) => key,
                None => XmlName::try_from(key)?,
            },
        })
    }
}

impl<'w, 'r, W: Write> ser::Serializer for Serializer<'w, 'r, W> {
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
            let ser = ElementSerializer {
                ser: self.ser,
                key: XmlName::try_from(variant)?,
            };
            value.serialize(ser)
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
            let ser = ElementSerializer {
                ser: self.ser,
                key: XmlName::try_from(variant)?,
            };
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
            let ser = ElementSerializer {
                ser: self.ser,
                key: XmlName::try_from(variant)?,
            };
            ser.serialize_struct(name, len)
        }
    }
}

macro_rules! write_primitive_content {
    ($method:ident ( $ty:ty )) => {
        #[inline]
        fn $method(self, value: $ty) -> Result<Self::Ok, Self::Error> {
            self.into_simple_type_serializer().$method(value)?;
            Ok(())
        }
    };
}

/// A serializer used to serialize content of an element. It does not write
/// surrounding tags. Unlike the [`ElementSerializer`], this serializer serializes
/// enums using variant names as tag names, i. e. as `<variant>...</variant>`
///
/// This serializer does the following:
/// - numbers converted to a decimal representation and serialized as naked strings;
/// - booleans serialized ether as `"true"` or `"false"`;
/// - strings and characters are serialized as naked strings;
/// - `None` does not write anything;
/// - `Some` and newtypes are serialized as an inner type using the same serializer;
/// - units (`()`) and unit structs does not write anything;
/// - sequences, tuples and tuple structs are serialized without delimiters.
///   `[1, 2, 3]` would be serialized as `123` (if not using indent);
/// - structs and maps are not supported ([`DeError::Unsupported`] is returned);
/// - enums:
///   - unit variants are serialized as self-closed `<variant/>`;
///   - newtype variants are serialized as inner value wrapped in `<variant>...</variant>`;
///   - tuple variants are serialized as sequences where each element is wrapped
///     in `<variant>...</variant>`;
///   - struct variants are serialized as a sequence of fields wrapped in
///     `<variant>...</variant>`. Each field is serialized recursively using
///     either [`ElementSerializer`], `ContentSerializer` (`$value` fields), or
///     [`SimpleTypeSerializer`] (`$text` fields). In particular, the empty struct
///     is serialized as `<variant/>`;
///
/// Usage of empty tags depends on the [`Self::expand_empty_elements`] setting.
///
/// The difference between this serializer and [`SimpleTypeSerializer`] is in how
/// sequences and maps are serialized. Unlike `SimpleTypeSerializer` it supports
/// any types in sequences and serializes them as list of elements, but that has
/// drawbacks. Sequence of primitives would be serialized without delimiters and
/// it will be impossible to distinguish between them. Even worse, when serializing
/// with indent, sequence of strings become one big string with additional content
/// and it would be impossible to distinguish between content of the original
/// strings and inserted indent characters.
pub struct ContentSerializer<'w, 'i, W: Write> {
    pub writer: &'w mut W,
    /// Defines which XML characters need to be escaped in text content
    pub level: QuoteLevel,
    /// Current indentation level. Note, that `Indent::None` means that there is
    /// no indentation at all, but `write_indent == false` means only, that indent
    /// writing is disabled in this instantiation of `ContentSerializer`, but
    /// child serializers should have access to the actual state of indentation.
    pub(super) indent: Indent<'i>,
    /// If `true`, then current indent will be written before writing the content,
    /// but only if content is not empty.
    pub write_indent: bool,
    // If `true`, then empty elements will be serialized as `<element></element>`
    // instead of `<element/>`.
    pub expand_empty_elements: bool,
    //TODO: add settings to disallow consequent serialization of primitives
}

impl<'w, 'i, W: Write> ContentSerializer<'w, 'i, W> {
    /// Turns this serializer into serializer of a text content
    #[inline]
    pub fn into_simple_type_serializer(self) -> SimpleTypeSerializer<'i, &'w mut W> {
        //TODO: Customization point: choose between CDATA and Text representation
        SimpleTypeSerializer {
            writer: self.writer,
            target: QuoteTarget::Text,
            level: self.level,
            indent: if self.write_indent {
                self.indent
            } else {
                Indent::None
            },
        }
    }

    /// Creates new serializer that shares state with this serializer and
    /// writes to the same underlying writer
    #[inline]
    pub fn new_seq_element_serializer(&mut self) -> ContentSerializer<W> {
        ContentSerializer {
            writer: self.writer,
            level: self.level,
            indent: self.indent.borrow(),
            write_indent: self.write_indent,
            expand_empty_elements: self.expand_empty_elements,
        }
    }

    /// Writes `name` as self-closed tag
    #[inline]
    pub(super) fn write_empty(mut self, name: XmlName) -> Result<(), DeError> {
        self.write_indent()?;
        if self.expand_empty_elements {
            self.writer.write_char('<')?;
            self.writer.write_str(name.0)?;
            self.writer.write_str("></")?;
            self.writer.write_str(name.0)?;
            self.writer.write_char('>')?;
        } else {
            self.writer.write_str("<")?;
            self.writer.write_str(name.0)?;
            self.writer.write_str("/>")?;
        }
        Ok(())
    }

    /// Writes simple type content between `name` tags
    pub(super) fn write_wrapped<S>(mut self, name: XmlName, serialize: S) -> Result<(), DeError>
    where
        S: for<'a> FnOnce(SimpleTypeSerializer<'i, &'a mut W>) -> Result<&'a mut W, DeError>,
    {
        self.write_indent()?;
        self.writer.write_char('<')?;
        self.writer.write_str(name.0)?;
        self.writer.write_char('>')?;

        let writer = serialize(self.into_simple_type_serializer())?;

        writer.write_str("</")?;
        writer.write_str(name.0)?;
        writer.write_char('>')?;
        Ok(())
    }

    pub(super) fn write_indent(&mut self) -> Result<(), DeError> {
        if self.write_indent {
            self.indent.write_io_indent(&mut self.writer)?;
            self.write_indent = false;
        }
        Ok(())
    }
}

impl<'w, 'i, W: Write> ser::Serializer for ContentSerializer<'w, 'i, W> {
    type Ok = ();
    type Error = DeError;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Tuple<'w, 'i, W>;
    type SerializeMap = Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = Impossible<Self::Ok, Self::Error>;
    type SerializeStructVariant = Struct<'w, 'i, W>;

    write_primitive_content!(serialize_bool(bool));

    write_primitive_content!(serialize_i8(i8));
    write_primitive_content!(serialize_i16(i16));
    write_primitive_content!(serialize_i32(i32));
    write_primitive_content!(serialize_i64(i64));

    write_primitive_content!(serialize_u8(u8));
    write_primitive_content!(serialize_u16(u16));
    write_primitive_content!(serialize_u32(u32));
    write_primitive_content!(serialize_u64(u64));

    serde_if_integer128! {
        write_primitive_content!(serialize_i128(i128));
        write_primitive_content!(serialize_u128(u128));
    }

    write_primitive_content!(serialize_f32(f32));
    write_primitive_content!(serialize_f64(f64));

    write_primitive_content!(serialize_char(char));
    write_primitive_content!(serialize_bytes(&[u8]));
    write_primitive_content!(serialize_str(&str));

    /// Does not write anything
    #[inline]
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_some<T: ?Sized + ser::Serialize>(
        self,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    /// Does not write anything
    #[inline]
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    /// Does not write anything
    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    /// If `variant` is a special `$text` variant, then do nothing, otherwise
    /// checks `variant` for XML name validity and writes `<variant/>`.
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        if variant == TEXT_KEY {
            Ok(())
        } else {
            let name = XmlName::try_from(variant)?;
            self.write_empty(name)
        }
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    /// If `variant` is a special `$text` variant, then writes `value` as a `xs:simpleType`,
    /// otherwise checks `variant` for XML name validity and writes `value` as a new
    /// `<variant>` element.
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        if variant == TEXT_KEY {
            value.serialize(self.into_simple_type_serializer())?;
            Ok(())
        } else {
            value.serialize(ElementSerializer {
                key: XmlName::try_from(variant)?,
                ser: self,
            })
        }
    }

    #[inline]
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(self)
    }

    #[inline]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_tuple(len)
    }

    /// Serializes variant as a tuple with name `variant`, producing
    ///
    /// ```xml
    /// <variant><!-- 1st element of a tuple --></variant>
    /// <variant><!-- 2nd element of a tuple --></variant>
    /// <!-- ... -->
    /// <variant><!-- Nth element of a tuple --></variant>
    /// ```
    #[inline]
    fn serialize_tuple_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        if variant == TEXT_KEY {
            self.into_simple_type_serializer()
                .serialize_tuple_struct(name, len)
                .map(Tuple::Text)
        } else {
            let ser = ElementSerializer {
                key: XmlName::try_from(variant)?,
                ser: self,
            };
            ser.serialize_tuple_struct(name, len).map(Tuple::Element)
        }
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(DeError::Unsupported(
            "serialization of map types is not supported in `$value` field".into(),
        ))
    }

    #[inline]
    fn serialize_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(DeError::Unsupported(
            format!("serialization of struct `{name}` is not supported in `$value` field").into(),
        ))
    }

    /// Serializes variant as an element with name `variant`, producing
    ///
    /// ```xml
    /// <variant>
    ///   <!-- struct fields... -->
    /// </variant>
    /// ```
    ///
    /// If struct has no fields which is represented by nested elements or a text,
    /// it may be serialized as self-closed element `<variant/>`.
    #[inline]
    fn serialize_struct_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        if variant == TEXT_KEY {
            Err(DeError::Unsupported(
                format!("cannot serialize `$text` struct variant of `{}` enum", name).into(),
            ))
        } else {
            let ser = ElementSerializer {
                key: XmlName::try_from(variant)?,
                ser: self,
            };
            ser.serialize_struct(name, len)
        }
    }
}

impl<'w, 'i, W: Write> SerializeSeq for ContentSerializer<'w, 'i, W> {
    type Ok = ();
    type Error = DeError;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self.new_seq_element_serializer())?;
        // Write indent for next element
        self.write_indent = true;
        Ok(())
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'w, 'i, W: Write> SerializeTuple for ContentSerializer<'w, 'i, W> {
    type Ok = ();
    type Error = DeError;

    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        SerializeSeq::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeSeq::end(self)
    }
}

impl<'w, 'i, W: Write> SerializeTupleStruct for ContentSerializer<'w, 'i, W> {
    type Ok = ();
    type Error = DeError;

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        SerializeSeq::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeSeq::end(self)
    }
}

macro_rules! write_primitive_elem {
    ($method:ident ( $ty:ty )) => {
        fn $method(self, value: $ty) -> Result<Self::Ok, Self::Error> {
            self.ser.write_wrapped(self.key, |ser| ser.$method(value))
        }
    };
}

/// A serializer used to serialize element with specified name. Unlike the [`ContentSerializer`],
/// this serializer never uses variant names of enum variants, and because of that
/// it is unable to serialize any enum values, except unit variants.
///
/// This serializer is used for an ordinary fields in structs, which are not special
/// fields named `$text` ([`TEXT_KEY`]) or `$value` ([`VALUE_KEY`]). `$text` field
/// should be serialized using [`SimpleTypeSerializer`] and `$value` field should be
/// serialized using [`ContentSerializer`].
///
/// This serializer does the following:
/// - numbers converted to a decimal representation and serialized as `<key>value</key>`;
/// - booleans serialized ether as `<key>true</key>` or `<key>false</key>`;
/// - strings and characters are serialized as `<key>value</key>`. In particular,
///   an empty string is serialized as `<key/>`;
/// - `None` is serialized as `<key/>`;
/// - `Some` and newtypes are serialized as an inner type using the same serializer;
/// - units (`()`) and unit structs are serialized as `<key/>`;
/// - sequences, tuples and tuple structs are serialized as repeated `<key>` tag.
///   In particular, empty sequence is serialized to nothing;
/// - structs are serialized as a sequence of fields wrapped in a `<key>` tag. Each
///   field is serialized recursively using either `ElementSerializer`, [`ContentSerializer`]
///   (`$value` fields), or [`SimpleTypeSerializer`] (`$text` fields).
///   In particular, the empty struct is serialized as `<key/>`;
/// - maps are serialized as a sequence of entries wrapped in a `<key>` tag. If key is
///   serialized to a special name, the same rules as for struct fields are applied.
///   In particular, the empty map is serialized as `<key/>`;
/// - enums:
///   - unit variants are serialized as `<key>variant</key>`;
///   - other variants are not supported ([`DeError::Unsupported`] is returned);
///
/// Usage of empty tags depends on the [`ContentSerializer::expand_empty_elements`] setting.
pub struct ElementSerializer<'w, 'k, W: Write> {
    /// The inner serializer that contains the settings and mostly do the actual work
    pub ser: ContentSerializer<'w, 'k, W>,
    /// Tag name used to wrap serialized types except enum variants which uses the variant name
    pub(super) key: XmlName<'k>,
}

impl<'w, 'k, W: Write> ser::Serializer for ElementSerializer<'w, 'k, W> {
    type Ok = ();
    type Error = DeError;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Impossible<Self::Ok, Self::Error>;
    type SerializeMap = Map<'w, 'k, W>;
    type SerializeStruct = Struct<'w, 'k, W>;
    type SerializeStructVariant = Struct<'w, 'k, W>;

    write_primitive_elem!(serialize_bool(bool));

    write_primitive_elem!(serialize_i8(i8));
    write_primitive_elem!(serialize_i16(i16));
    write_primitive_elem!(serialize_i32(i32));
    write_primitive_elem!(serialize_i64(i64));

    write_primitive_elem!(serialize_u8(u8));
    write_primitive_elem!(serialize_u16(u16));
    write_primitive_elem!(serialize_u32(u32));
    write_primitive_elem!(serialize_u64(u64));

    serde_if_integer128! {
        write_primitive_elem!(serialize_i128(i128));
        write_primitive_elem!(serialize_u128(u128));
    }

    write_primitive_elem!(serialize_f32(f32));
    write_primitive_elem!(serialize_f64(f64));

    write_primitive_elem!(serialize_char(char));
    write_primitive_elem!(serialize_bytes(&[u8]));

    fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
        if value.is_empty() {
            self.ser.write_empty(self.key)
        } else {
            self.ser
                .write_wrapped(self.key, |ser| ser.serialize_str(value))
        }
    }

    /// By serde contract we should serialize key of [`None`] values. If someone
    /// wants to skip the field entirely, he should use
    /// `#[serde(skip_serializing_if = "Option::is_none")]`.
    ///
    /// In XML when we serialize field, we write field name as:
    /// - element name, or
    /// - attribute name
    ///
    /// and field value as
    /// - content of the element, or
    /// - attribute value
    ///
    /// So serialization of `None` works the same as [serialization of `()`](#method.serialize_unit)
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.ser.write_empty(self.key)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.ser.write_empty(self.key)
    }

    /// Writes a tag with name [`Self::key`] and content of unit variant inside.
    /// If variant is a special `$text` value, then empty tag `<key/>` is written.
    /// Otherwise a `<key>variant</key>` is written.
    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        if variant == TEXT_KEY {
            self.ser.write_empty(self.key)
        } else {
            self.ser.write_wrapped(self.key, |ser| {
                ser.serialize_unit_variant(name, variant_index, variant)
            })
        }
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    /// Always returns [`DeError::Unsupported`]. Newtype variants can be serialized
    /// only in `$value` fields, which is serialized using [`ContentSerializer`].
    #[inline]
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Err(DeError::Unsupported(
            format!(
                "cannot serialize enum newtype variant `{}::{}`",
                name, variant
            )
            .into(),
        ))
    }

    #[inline]
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(self)
    }

    #[inline]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_tuple(len)
    }

    /// Always returns [`DeError::Unsupported`]. Tuple variants can be serialized
    /// only in `$value` fields, which is serialized using [`ContentSerializer`].
    #[inline]
    fn serialize_tuple_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(DeError::Unsupported(
            format!(
                "cannot serialize enum tuple variant `{}::{}`",
                name, variant
            )
            .into(),
        ))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(Map {
            ser: self.serialize_struct("", 0)?,
            key: None,
        })
    }

    #[inline]
    fn serialize_struct(
        mut self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.ser.write_indent()?;
        self.ser.indent.increase();

        self.ser.writer.write_char('<')?;
        self.ser.writer.write_str(self.key.0)?;
        Ok(Struct {
            ser: self,
            children: Vec::new(),
        })
    }

    /// Always returns [`DeError::Unsupported`]. Struct variants can be serialized
    /// only in `$value` fields, which is serialized using [`ContentSerializer`].
    #[inline]
    fn serialize_struct_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(DeError::Unsupported(
            format!(
                "cannot serialize enum struct variant `{}::{}`",
                name, variant
            )
            .into(),
        ))
    }
}

impl<'w, 'k, W: Write> SerializeSeq for ElementSerializer<'w, 'k, W> {
    type Ok = ();
    type Error = DeError;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(ElementSerializer {
            ser: self.ser.new_seq_element_serializer(),
            key: self.key,
        })?;
        // Write indent for the next element
        self.ser.write_indent = true;
        Ok(())
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'w, 'k, W: Write> SerializeTuple for ElementSerializer<'w, 'k, W> {
    type Ok = ();
    type Error = DeError;

    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        SerializeSeq::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeSeq::end(self)
    }
}

impl<'w, 'k, W: Write> SerializeTupleStruct for ElementSerializer<'w, 'k, W> {
    type Ok = ();
    type Error = DeError;

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        SerializeSeq::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeSeq::end(self)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// A serializer for tuple variants. Tuples can be serialized in two modes:
/// - wrapping each tuple field into a tag
/// - without wrapping, fields are delimited by a space
pub enum Tuple<'w, 'k, W: Write> {
    /// Serialize each tuple field as an element
    Element(ElementSerializer<'w, 'k, W>),
    /// Serialize tuple as an `xs:list`: space-delimited content of fields
    Text(SimpleSeq<'k, &'w mut W>),
}

impl<'w, 'k, W: Write> SerializeTupleVariant for Tuple<'w, 'k, W> {
    type Ok = ();
    type Error = DeError;

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        match self {
            Self::Element(ser) => SerializeTuple::serialize_element(ser, value),
            Self::Text(ser) => SerializeTuple::serialize_element(ser, value),
        }
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        match self {
            Self::Element(ser) => SerializeTuple::end(ser),
            Self::Text(ser) => SerializeTuple::end(ser).map(|_| ()),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// A serializer for struct variants, which serializes the struct contents inside
/// of wrapping tags (`<${tag}>...</${tag}>`).
///
/// Serialization of each field depends on it representation:
/// - attributes written directly to the higher serializer
/// - elements buffered into internal buffer and at the end written into higher
///   serializer
pub struct Struct<'w, 'k, W: Write> {
    ser: ElementSerializer<'w, 'k, W>,
    /// Buffer to store serialized elements
    // TODO: Customization point: allow direct writing of elements, but all
    // attributes should be listed first. Fail, if attribute encountered after
    // element. Use feature to configure
    children: Vec<u8>,
}

impl<'w, 'k, W: Write> Struct<'w, 'k, W> {
    #[inline]
    fn write_field<T>(&mut self, key: &str, value: &T) -> Result<(), DeError>
    where
        T: ?Sized + Serialize,
    {
        //TODO: Customization point: allow user to determine if field is attribute or not
        if let Some(key) = key.strip_prefix('@') {
            let key = XmlName::try_from(key)?;
            self.write_attribute(key, value)
        } else {
            self.write_element(key, value)
        }
    }

    /// Writes `value` as an attribute
    #[inline]
    fn write_attribute<T>(&mut self, key: XmlName, value: &T) -> Result<(), DeError>
    where
        T: ?Sized + Serialize,
    {
        //TODO: Customization point: each attribute on new line
        self.ser.ser.writer.write_char(' ')?;
        self.ser.ser.writer.write_str(key.0)?;
        self.ser.ser.writer.write_char('=')?;

        //TODO: Customization point: preferred quote style
        self.ser.ser.writer.write_char('"')?;
        value.serialize(SimpleTypeSerializer {
            writer: &mut self.ser.ser.writer,
            target: QuoteTarget::DoubleQAttr,
            level: self.ser.ser.level,
            indent: Indent::None,
        })?;
        self.ser.ser.writer.write_char('"')?;

        Ok(())
    }

    /// Writes `value` either as a text content, or as an element.
    ///
    /// If `key` has a magic value [`TEXT_KEY`], then `value` serialized as a
    /// [simple type].
    ///
    /// If `key` has a magic value [`VALUE_KEY`], then `value` serialized as a
    /// [content] without wrapping in tags, otherwise it is wrapped in
    /// `<${key}>...</${key}>`.
    ///
    /// [simple type]: SimpleTypeSerializer
    /// [content]: ContentSerializer
    fn write_element<T>(&mut self, key: &str, value: &T) -> Result<(), DeError>
    where
        T: ?Sized + Serialize,
    {
        let ser = ContentSerializer {
            writer: &mut self.children,
            level: self.ser.ser.level,
            indent: self.ser.ser.indent.borrow(),
            write_indent: true,
            expand_empty_elements: self.ser.ser.expand_empty_elements,
        };

        if key == TEXT_KEY {
            value.serialize(TextSerializer(ser.into_simple_type_serializer()))?;
        } else if key == VALUE_KEY {
            value.serialize(ser)?;
        } else {
            value.serialize(ElementSerializer {
                key: XmlName::try_from(key)?,
                ser,
            })?;
        }
        Ok(())
    }
}

impl<'w, 'k, W: Write> SerializeStruct for Struct<'w, 'k, W> {
    type Ok = ();
    type Error = DeError;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.write_field(key, value)
    }

    fn end(mut self) -> Result<Self::Ok, Self::Error> {
        self.ser.ser.indent.decrease();

        if self.children.is_empty() {
            if self.ser.ser.expand_empty_elements {
                self.ser.ser.writer.write_str("></")?;
                self.ser.ser.writer.write_str(self.ser.key.0)?;
                self.ser.ser.writer.write_char('>')?;
            } else {
                self.ser.ser.writer.write_str("/>")?;
            }
        } else {
            self.ser.ser.writer.write_char('>')?;
            self.ser.ser.writer.write(&self.children)?;

            self.ser
                .ser
                .indent
                .write_io_indent(&mut self.ser.ser.writer)?;

            self.ser.ser.writer.write_str("</")?;
            self.ser.ser.writer.write_str(self.ser.key.0)?;
            self.ser.ser.writer.write_char('>')?;
        }
        Ok(())
    }
}

impl<'w, 'k, W: Write> SerializeStructVariant for Struct<'w, 'k, W> {
    type Ok = ();
    type Error = DeError;

    #[inline]
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        SerializeStruct::serialize_field(self, key, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeStruct::end(self)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Map<'w, 'k, W: Write> {
    ser: Struct<'w, 'k, W>,
    /// Key, serialized by `QNameSerializer` if consumer uses `serialize_key` +
    /// `serialize_value` calls instead of `serialize_entry`
    key: Option<String>,
}

impl<'w, 'k, W: Write> Map<'w, 'k, W> {
    fn make_key<T>(&mut self, key: &T) -> Result<String, DeError>
    where
        T: ?Sized + Serialize,
    {
        key.serialize(super::key::QNameSerializer {
            writer: String::new(),
        })
    }
}

impl<'w, 'k, W: Write> SerializeMap for Map<'w, 'k, W> {
    type Ok = ();
    type Error = DeError;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        if let Some(_) = self.key.take() {
            return Err(DeError::Custom(
                "calling `serialize_key` twice without `serialize_value`".to_string(),
            ));
        }
        self.key = Some(self.make_key(key)?);
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        if let Some(key) = self.key.take() {
            return self.ser.write_field(&key, value);
        }
        Err(DeError::Custom(
            "calling `serialize_value` without call of `serialize_key`".to_string(),
        ))
    }

    fn serialize_entry<K, V>(&mut self, key: &K, value: &V) -> Result<(), Self::Error>
    where
        K: ?Sized + Serialize,
        V: ?Sized + Serialize,
    {
        let key = self.make_key(key)?;
        self.ser.write_field(&key, value)
    }

    fn end(mut self) -> Result<Self::Ok, Self::Error> {
        if let Some(key) = self.key.take() {
            return Err(DeError::Custom(format!(
                "calling `end` without call of `serialize_value` for key `{key}`"
            )));
        }
        SerializeStruct::end(self.ser)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
///
macro_rules! write_atomic {
    ($method:ident ( $ty:ty )) => {
        fn $method(mut self, value: $ty) -> Result<Self::Ok, Self::Error> {
            self.write_str(&value.to_string())?;
            Ok(true)
        }
    };
}

/// A serializer that handles ordinary [simple type definition][item] with
/// `{variety} = atomic`, or an ordinary [simple type] definition with
/// `{variety} = union` whose basic members are all atomic.
///
/// This serializer can serialize only primitive types:
/// - numbers
/// - booleans
/// - strings
/// - units
/// - options
/// - unit variants of enums
///
/// Identifiers represented as strings and serialized accordingly.
///
/// Serialization of all other types returns [`Unsupported`][DeError::Unsupported] error.
///
/// This serializer returns `true` if something was written and `false` otherwise.
///
/// [item]: https://www.w3.org/TR/xmlschema11-1/#std-item_type_definition
/// [simple type]: https://www.w3.org/TR/xmlschema11-1/#Simple_Type_Definition
pub struct AtomicSerializer<'i, W: Write> {
    pub writer: W,
    pub target: QuoteTarget,
    /// Defines which XML characters need to be escaped
    pub level: QuoteLevel,
    /// When `Some`, the indent that should be written before the content
    /// if content is not an empty string.
    /// When `None` an `xs:list` delimiter (a space) should be written
    pub(crate) indent: Option<Indent<'i>>,
}

impl<'i, W: Write> AtomicSerializer<'i, W> {
    fn write_str(&mut self, value: &str) -> Result<(), DeError> {
        if let Some(indent) = self.indent.as_mut() {
            indent.write_io_indent(&mut self.writer)?;
        } else {
            // TODO: Customization point -- possible non-XML compatible extension to specify delimiter char
            self.writer.write_char(' ')?;
        }
        Ok(self.writer.write_str(value)?)
    }
}

impl<'i, W: Write> ser::Serializer for AtomicSerializer<'i, W> {
    type Ok = bool;
    type Error = DeError;

    type SerializeSeq = Impossible<Self::Ok, Self::Error>;
    type SerializeTuple = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = Impossible<Self::Ok, Self::Error>;
    type SerializeMap = Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = Impossible<Self::Ok, Self::Error>;
    type SerializeStructVariant = Impossible<Self::Ok, Self::Error>;

    fn serialize_bool(mut self, value: bool) -> Result<Self::Ok, Self::Error> {
        self.write_str(if value { "true" } else { "false" })?;
        Ok(true)
    }

    write_atomic!(serialize_i8(i8));
    write_atomic!(serialize_i16(i16));
    write_atomic!(serialize_i32(i32));
    write_atomic!(serialize_i64(i64));

    write_atomic!(serialize_u8(u8));
    write_atomic!(serialize_u16(u16));
    write_atomic!(serialize_u32(u32));
    write_atomic!(serialize_u64(u64));

    serde_if_integer128! {
        write_atomic!(serialize_i128(i128));
        write_atomic!(serialize_u128(u128));
    }

    write_atomic!(serialize_f32(f32));
    write_atomic!(serialize_f64(f64));

    fn serialize_char(self, value: char) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(&value.to_string())
    }

    fn serialize_str(mut self, value: &str) -> Result<Self::Ok, Self::Error> {
        if !value.is_empty() {
            self.write_str(&super::simple_type::escape_item(
                value,
                self.target,
                self.level,
            ))?;
        }
        Ok(!value.is_empty())
    }

    fn serialize_bytes(mut self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
        if !value.is_empty() {
            self.writer.write(&value)?;
        }
        Ok(!value.is_empty())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(false)
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    /// We cannot store anything, so the absence of a unit and presence of it
    /// does not differ, so serialization of unit returns `Err(Unsupported)`
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(DeError::Unsupported(
            "cannot serialize unit type `()` as an `xs:list` item".into(),
        ))
    }

    /// We cannot store anything, so the absence of a unit and presence of it
    /// does not differ, so serialization of unit returns `Err(Unsupported)`
    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(DeError::Unsupported(
            format!(
                "cannot serialize unit struct `{}` as an `xs:list` item",
                name
            )
            .into(),
        ))
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

    /// We cannot store both a variant discriminant and a variant value,
    /// so serialization of enum newtype variant returns `Err(Unsupported)`
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, DeError> {
        Err(DeError::Unsupported(
            format!(
                "cannot serialize enum newtype variant `{}::{}` as an `xs:list` item",
                name, variant
            )
            .into(),
        ))
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(DeError::Unsupported(
            "cannot serialize sequence as an `xs:list` item".into(),
        ))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(DeError::Unsupported(
            "cannot serialize tuple as an `xs:list` item".into(),
        ))
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(DeError::Unsupported(
            format!(
                "cannot serialize tuple struct `{}` as an `xs:list` item",
                name
            )
            .into(),
        ))
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(DeError::Unsupported(
            format!(
                "cannot serialize enum tuple variant `{}::{}` as an `xs:list` item",
                name, variant
            )
            .into(),
        ))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(DeError::Unsupported(
            "cannot serialize map as an `xs:list` item".into(),
        ))
    }

    fn serialize_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(DeError::Unsupported(
            format!("cannot serialize struct `{}` as an `xs:list` item", name).into(),
        ))
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(DeError::Unsupported(
            format!(
                "cannot serialize enum struct variant `{}::{}` as an `xs:list` item",
                name, variant
            )
            .into(),
        ))
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// A serializer for a values representing XSD [simple types], which used in:
/// - attribute values (`<... ...="value" ...>`)
/// - text content (`<...>text</...>`)
/// - CDATA content (`<...><![CDATA[cdata]]></...>`)
///
/// [simple types]: https://www.w3.org/TR/xmlschema11-1/#Simple_Type_Definition
pub struct SimpleTypeSerializer<'i, W: Write> {
    /// Writer to which this serializer writes content
    pub writer: W,
    /// Target for which element is serializing. Affects additional characters to escape.
    pub target: QuoteTarget,
    /// Defines which XML characters need to be escaped
    pub level: QuoteLevel,
    /// Indent that should be written before the content if content is not an empty string
    pub(crate) indent: Indent<'i>,
}

impl<'i, W: Write> SimpleTypeSerializer<'i, W> {
    fn write_str(&mut self, value: &str) -> Result<(), DeError> {
        self.indent.write_io_indent(&mut self.writer)?;
        Ok(self.writer.write_str(value)?)
    }
}

impl<'i, W: Write> ser::Serializer for SimpleTypeSerializer<'i, W> {
    type Ok = W;
    type Error = DeError;

    type SerializeSeq = SimpleSeq<'i, W>;
    type SerializeTuple = SimpleSeq<'i, W>;
    type SerializeTupleStruct = SimpleSeq<'i, W>;
    type SerializeTupleVariant = Impossible<Self::Ok, Self::Error>;
    type SerializeMap = Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = Impossible<Self::Ok, Self::Error>;
    type SerializeStructVariant = Impossible<Self::Ok, Self::Error>;

    write_primitive!();

    fn serialize_str(mut self, value: &str) -> Result<Self::Ok, Self::Error> {
        if !value.is_empty() {
            self.write_str(&super::simple_type::escape_list(
                value,
                self.target,
                self.level,
            ))?;
        }
        Ok(self.writer)
    }

    /// Does not write anything
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.writer)
    }

    /// Does not write anything
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(self.writer)
    }

    /// We cannot store both a variant discriminant and a variant value,
    /// so serialization of enum newtype variant returns `Err(Unsupported)`
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, DeError> {
        Err(DeError::Unsupported(
            format!("cannot serialize enum newtype variant `{}::{}` as an attribute or text content value", name, variant).into(),
        ))
    }

    #[inline]
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SimpleSeq {
            writer: self.writer,
            target: self.target,
            level: self.level,
            indent: self.indent,
            is_empty: true,
        })
    }

    #[inline]
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(None)
    }

    #[inline]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(None)
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(DeError::Unsupported(
            format!("cannot serialize enum tuple variant `{}::{}` as an attribute or text content value", name, variant).into(),
        ))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(DeError::Unsupported(
            "cannot serialize map as an attribute or text content value".into(),
        ))
    }

    fn serialize_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(DeError::Unsupported(
            format!(
                "cannot serialize struct `{}` as an attribute or text content value",
                name
            )
            .into(),
        ))
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(DeError::Unsupported(
            format!("cannot serialize enum struct variant `{}::{}` as an attribute or text content value", name, variant).into(),
        ))
    }
}

/// Serializer for a sequence of atomic values delimited by space
pub struct SimpleSeq<'i, W: Write> {
    writer: W,
    target: QuoteTarget,
    level: QuoteLevel,
    /// Indent that should be written before the content if content is not an empty string
    indent: Indent<'i>,
    /// If `true`, nothing was written yet to the `writer`
    is_empty: bool,
}

impl<'i, W: Write> SerializeSeq for SimpleSeq<'i, W> {
    type Ok = W;
    type Error = DeError;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        // Write indent for the first element and delimiter for others
        let indent = if self.is_empty {
            Some(self.indent.borrow())
        } else {
            None
        };
        if value.serialize(AtomicSerializer {
            writer: &mut self.writer,
            target: self.target,
            level: self.level,
            indent,
        })? {
            self.is_empty = false;
        }
        Ok(())
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.writer)
    }
}

impl<'i, W: Write> SerializeTuple for SimpleSeq<'i, W> {
    type Ok = W;
    type Error = DeError;

    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        SerializeSeq::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeSeq::end(self)
    }
}

impl<'i, W: Write> SerializeTupleStruct for SimpleSeq<'i, W> {
    type Ok = W;
    type Error = DeError;

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        SerializeSeq::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeSeq::end(self)
    }
}

impl<'i, W: Write> SerializeTupleVariant for SimpleSeq<'i, W> {
    type Ok = W;
    type Error = DeError;

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        SerializeSeq::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeSeq::end(self)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
///

////////////////////////////////////////////////////////////////////////////////////////////////////

macro_rules! write_primitive_text {
    ($method:ident ( $ty:ty )) => {
        #[inline]
        fn $method(self, value: $ty) -> Result<Self::Ok, Self::Error> {
            self.0.$method(value)
        }
    };
}

/// A serializer used to serialize a `$text` field of a struct or map.
///
/// This serializer a very similar to [`SimpleTypeSerializer`], but different
/// from it in how it processes unit enum variants. Unlike [`SimpleTypeSerializer`]
/// this serializer does not write anything for the unit variant.
pub struct TextSerializer<'i, W: Write>(pub SimpleTypeSerializer<'i, W>);

impl<'i, W: Write> ser::Serializer for TextSerializer<'i, W> {
    type Ok = W;
    type Error = DeError;

    type SerializeSeq = SimpleSeq<'i, W>;
    type SerializeTuple = SimpleSeq<'i, W>;
    type SerializeTupleStruct = SimpleSeq<'i, W>;
    type SerializeTupleVariant = SimpleSeq<'i, W>;
    type SerializeMap = Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = Impossible<Self::Ok, Self::Error>;
    type SerializeStructVariant = Impossible<Self::Ok, Self::Error>;

    write_primitive_text!(serialize_bool(bool));

    write_primitive_text!(serialize_i8(i8));
    write_primitive_text!(serialize_i16(i16));
    write_primitive_text!(serialize_i32(i32));
    write_primitive_text!(serialize_i64(i64));

    write_primitive_text!(serialize_u8(u8));
    write_primitive_text!(serialize_u16(u16));
    write_primitive_text!(serialize_u32(u32));
    write_primitive_text!(serialize_u64(u64));

    serde_if_integer128! {
        write_primitive_text!(serialize_i128(i128));
        write_primitive_text!(serialize_u128(u128));
    }

    write_primitive_text!(serialize_f32(f32));
    write_primitive_text!(serialize_f64(f64));

    write_primitive_text!(serialize_char(char));
    write_primitive_text!(serialize_str(&str));
    write_primitive_text!(serialize_bytes(&[u8]));

    #[inline]
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_none()
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    #[inline]
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_unit()
    }

    #[inline]
    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_unit_struct(name)
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        if variant == TEXT_KEY {
            Ok(self.0.writer)
        } else {
            self.0.serialize_unit_variant(name, variant_index, variant)
        }
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    #[inline]
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Err(DeError::Unsupported(
            format!(
                "cannot serialize enum newtype variant `{}::{}` as text content value",
                name, variant
            )
            .into(),
        ))
    }

    #[inline]
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.0.serialize_seq(len)
    }

    #[inline]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.0.serialize_tuple(len)
    }

    #[inline]
    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.0.serialize_tuple_struct(name, len)
    }

    #[inline]
    fn serialize_tuple_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(DeError::Unsupported(
            format!(
                "cannot serialize enum tuple variant `{}::{}` as text content value",
                name, variant
            )
            .into(),
        ))
    }

    #[inline]
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(DeError::Unsupported(
            "cannot serialize map as text content value".into(),
        ))
    }

    #[inline]
    fn serialize_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(DeError::Unsupported(
            format!("cannot serialize struct `{}` as text content value", name).into(),
        ))
    }

    #[inline]
    fn serialize_struct_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(DeError::Unsupported(
            format!(
                "cannot serialize enum struct variant `{}::{}` as text content value",
                name, variant
            )
            .into(),
        ))
    }
}

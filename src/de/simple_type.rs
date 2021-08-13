//! Contains Serde `Deserializer` for XML [simple types] [as defined] in the XML Schema.
//!
//! [simple types]: https://www.w3schools.com/xml/el_simpletype.asp
//! [as defined]: https://www.w3.org/TR/xmlschema11-1/#Simple_Type_Definition

use crate::de::str2bool;
use crate::errors::serialize::DeError;
use crate::escape::unescape;
use serde::de::{DeserializeSeed, Deserializer, EnumAccess, VariantAccess, Visitor};
use serde::{self, serde_if_integer128};
use std::borrow::Cow;

macro_rules! deserialize_num {
    ($method:ident, $visit:ident) => {
        fn $method<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            visitor.$visit(self.content.as_str().parse()?)
        }
    };
}

macro_rules! unsupported {
    (
        $deserialize:ident
        $(
            ($($type:ty),*)
        )?
        => $message:literal
    ) => {
        #[inline]
        fn $deserialize<V: Visitor<'de>>(
            self,
            $($(_: $type,)*)?
            _visitor: V
        ) -> Result<V::Value, Self::Error> {
            Err(DeError::Unsupported($message))
        }
    };
}

//-----------------------------------------------------------------------------

/// A version of [`Cow`] that can borrow from two different buffers, one of them
/// is a deserializer input
#[derive(Clone)]
enum Content<'de, 'a> {
    /// An input borrowed from the parsed data
    Input(&'de str),
    /// An input borrowed from the buffer owned by another deserializer
    Slice(&'a str),
    /// An input taken from an external deserializer, owned by that deserializer.
    /// Only part of this data, located after offset represented by `usize`, used
    /// to deserialize data, the other is a garbage that can't be dropped because
    /// we do not want to make reallocations if they will not required.
    Owned(String, usize),
}
impl<'de, 'a> Content<'de, 'a> {
    /// Returns string representation of the content
    fn as_str(&self) -> &str {
        match self {
            Content::Input(s) => s,
            Content::Slice(s) => s,
            Content::Owned(s, offset) => s.split_at(*offset).1,
        }
    }

    /// Supply to the visitor borrowed string, string slice, or owned string
    /// depending on the kind of input. Unlike [`Self::deserialize_item`],
    /// the whole [`Self::Owned`] string will be passed to the visitor.
    ///
    /// Calls
    /// - `visitor.visit_borrowed_str` if data borrowed from the input
    /// - `visitor.visit_str` if data borrowed from another source
    /// - `visitor.visit_string` if data owned by this type
    #[inline]
    fn deserialize_all<V>(self, visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        match self {
            Content::Input(s) => visitor.visit_borrowed_str(s),
            Content::Slice(s) => visitor.visit_str(s),
            Content::Owned(s, _) => visitor.visit_string(s),
        }
    }

    /// Supply to the visitor borrowed string, string slice, or owned string
    /// depending on the kind of input. Unlike [`Self::deserialize_all`],
    /// only part of [`Self::Owned`] string will be passed to the visitor.
    ///
    /// Calls
    /// - `visitor.visit_borrowed_str` if data borrowed from the input
    /// - `visitor.visit_str` if data borrowed from another source
    /// - `visitor.visit_string` if data owned by this type
    #[inline]
    fn deserialize_item<V>(self, visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        match self {
            Content::Input(s) => visitor.visit_borrowed_str(s),
            Content::Slice(s) => visitor.visit_str(s),
            Content::Owned(s, 0) => visitor.visit_string(s),
            Content::Owned(s, offset) => visitor.visit_str(s.split_at(offset).1),
        }
    }
}

/// A deserializer that handles ordinary [simple type definition][item] with
/// `{variety} = atomic`, or an ordinary [simple type] definition with
/// `{variety} = union` whose basic members are all atomic.
///
/// This deserializer can deserialize only primitive types:
/// - numbers
/// - booleans
/// - strings
/// - units
/// - options
/// - unit variants of enums
///
/// Identifiers represented as strings and deserialized accordingly.
///
/// Deserialization of all other types returns [`Unsupported`][DeError::Unsupported] error.
///
/// [item]: https://www.w3.org/TR/xmlschema11-1/#std-item_type_definition
/// [simple type]: https://www.w3.org/TR/xmlschema11-1/#Simple_Type_Definition
#[derive(Clone)]
struct AtomicDeserializer<'de, 'a> {
    /// Content of the attribute value, text content or CDATA content
    content: Content<'de, 'a>,
    /// If `true`, `content` in an escaped form and should be unescaped before use
    escaped: bool,
}

impl<'de, 'a> Deserializer<'de> for AtomicDeserializer<'de, 'a> {
    type Error = DeError;

    /// Forwards deserialization to the [`Self::deserialize_str`]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    /// According to the <https://www.w3.org/TR/xmlschema-2/#boolean>,
    /// valid boolean representations are only `"true"`, `"false"`, `"1"`,
    /// and `"0"`. But this method also handles following:
    ///
    /// |`bool` |XML content
    /// |-------|-------------------------------------------------------------
    /// |`true` |`"True"`,  `"TRUE"`,  `"t"`, `"Yes"`, `"YES"`, `"yes"`, `"y"`
    /// |`false`|`"False"`, `"FALSE"`, `"f"`, `"No"`,  `"NO"`,  `"no"`,  `"n"`
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        str2bool(self.content.as_str(), visitor)
    }

    deserialize_num!(deserialize_i8, visit_i8);
    deserialize_num!(deserialize_i16, visit_i16);
    deserialize_num!(deserialize_i32, visit_i32);
    deserialize_num!(deserialize_i64, visit_i64);

    deserialize_num!(deserialize_u8, visit_u8);
    deserialize_num!(deserialize_u16, visit_u16);
    deserialize_num!(deserialize_u32, visit_u32);
    deserialize_num!(deserialize_u64, visit_u64);

    serde_if_integer128! {
        deserialize_num!(deserialize_i128, visit_i128);
        deserialize_num!(deserialize_u128, visit_u128);
    }

    deserialize_num!(deserialize_f32, visit_f32);
    deserialize_num!(deserialize_f64, visit_f64);

    /// Forwards deserialization to the [`Self::deserialize_str`]
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    /// Supply to the visitor borrowed string, string slice, or owned string
    /// depending on the kind of input and presence of the escaped data.
    ///
    /// If string requires unescaping, then calls [`Visitor::visit_string`] with
    /// new allocated buffer with unescaped data.
    ///
    /// Otherwise calls
    /// - [`Visitor::visit_borrowed_str`] if data borrowed from the input
    /// - [`Visitor::visit_str`] if data borrowed from other deserializer
    /// - [`Visitor::visit_string`] if data owned by this deserializer
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if self.escaped {
            match unescape(self.content.as_str().as_bytes())? {
                Cow::Borrowed(_) => self.content.deserialize_item(visitor),
                Cow::Owned(buf) => visitor.visit_string(String::from_utf8(buf)?),
            }
        } else {
            self.content.deserialize_item(visitor)
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    /// If `content` is an empty string then calls [`Visitor::visit_none`],
    /// otherwise calls [`Visitor::visit_some`] with itself
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if self.content.as_str().is_empty() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    /// Forwards deserialization to the [`Self::deserialize_unit`]
    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_enum(self)
    }

    /// Forwards deserialization to the [`Self::deserialize_str`]
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    unsupported!(deserialize_bytes        => "byte arrays are not supported as `xs:list` items");
    unsupported!(deserialize_byte_buf     => "byte arrays are not supported as `xs:list` items");
    unsupported!(deserialize_seq          => "sequences are not supported as `xs:list` items");
    unsupported!(deserialize_tuple(usize) => "tuples are not supported as `xs:list` items");
    unsupported!(deserialize_tuple_struct(&'static str, usize) => "tuples are not supported as `xs:list` items");
    unsupported!(deserialize_map          => "maps are not supported as `xs:list` items");
    unsupported!(deserialize_struct(&'static str, &'static [&'static str]) => "structures are not supported as `xs:list` items");
}

impl<'de, 'a> EnumAccess<'de> for AtomicDeserializer<'de, 'a> {
    type Error = DeError;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self), DeError>
    where
        V: DeserializeSeed<'de>,
    {
        let name = seed.deserialize(self.clone())?;
        Ok((name, self))
    }
}

impl<'de, 'a> VariantAccess<'de> for AtomicDeserializer<'de, 'a> {
    type Error = DeError;

    #[inline]
    fn unit_variant(self) -> Result<(), DeError> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, _seed: T) -> Result<T::Value, DeError>
    where
        T: DeserializeSeed<'de>,
    {
        Err(DeError::Unsupported(
            "enum newtype variants are not supported as `xs:list` items",
        ))
    }

    fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        Err(DeError::Unsupported(
            "enum tuple variants are not supported as `xs:list` items",
        ))
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        Err(DeError::Unsupported(
            "enum struct variants are not supported as `xs:list` items",
        ))
    }
}

//-----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::de::byte_buf::{ByteBuf, Bytes};
    use serde::de::IgnoredAny;
    use serde::Deserialize;
    use std::collections::HashMap;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Unit;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Newtype(String);

    #[derive(Debug, Deserialize, PartialEq)]
    struct BorrowedNewtype<'a>(&'a str);

    #[derive(Debug, Deserialize, PartialEq)]
    struct Struct {
        key: String,
        val: usize,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    enum Enum {
        Unit,
        Newtype(String),
        Tuple(String, usize),
        Struct { key: String, val: usize },
    }

    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(field_identifier)]
    enum Id {
        Field,
    }

    #[derive(Debug, Deserialize)]
    struct Any(IgnoredAny);
    impl PartialEq for Any {
        fn eq(&self, _other: &Any) -> bool {
            true
        }
    }

    /// Tests for deserialize atomic and union values, as defined in XSD specification
    mod atomic {
        use super::*;

        /// Checks that given `$input` successfully deserializing into given `$result`
        macro_rules! deserialized_to {
            ($name:ident: $type:ty = $input:literal => $result:expr) => {
                #[test]
                fn $name() {
                    let de = AtomicDeserializer {
                        content: Content::Input($input),
                        escaped: true,
                    };
                    let data: $type = Deserialize::deserialize(de).unwrap();

                    assert_eq!(data, $result);
                }
            };
        }

        /// Checks that attempt to deserialize given `$input` as a `$type` results to a
        /// deserialization error `$kind` with `$reason`
        macro_rules! err {
            ($name:ident: $type:ty = $input:literal => $kind:ident($reason:literal)) => {
                #[test]
                fn $name() {
                    let de = AtomicDeserializer {
                        content: Content::Input($input),
                        escaped: true,
                    };
                    let err = <$type as Deserialize>::deserialize(de).unwrap_err();

                    match err {
                        DeError::$kind(e) => assert_eq!(e, $reason),
                        _ => panic!(
                            "Expected `{}({})`, found `{:?}`",
                            stringify!($kind),
                            $reason,
                            err
                        ),
                    }
                }
            };
        }

        deserialized_to!(any_owned: String = "&lt;escaped&#x20;string" => "<escaped string");
        deserialized_to!(any_borrowed: &str = "non-escaped string" => "non-escaped string");

        deserialized_to!(false_: bool = "false" => false);
        deserialized_to!(true_: bool  = "true" => true);

        deserialized_to!(i8_:  i8  = "-2" => -2);
        deserialized_to!(i16_: i16 = "-2" => -2);
        deserialized_to!(i32_: i32 = "-2" => -2);
        deserialized_to!(i64_: i64 = "-2" => -2);

        deserialized_to!(u8_:  u8  = "3" => 3);
        deserialized_to!(u16_: u16 = "3" => 3);
        deserialized_to!(u32_: u32 = "3" => 3);
        deserialized_to!(u64_: u64 = "3" => 3);

        serde_if_integer128! {
            deserialized_to!(i128_: i128 = "-2" => -2);
            deserialized_to!(u128_: u128 = "2" => 2);
        }

        deserialized_to!(f32_: f32 = "1.23" => 1.23);
        deserialized_to!(f64_: f64 = "1.23" => 1.23);

        deserialized_to!(char_unescaped: char = "h" => 'h');
        deserialized_to!(char_escaped: char = "&lt;" => '<');

        deserialized_to!(string: String = "&lt;escaped&#x20;string" => "<escaped string");
        deserialized_to!(borrowed_str: &str = "non-escaped string" => "non-escaped string");
        err!(escaped_str: &str = "escaped&#x20;string"
                => Custom("invalid type: string \"escaped string\", expected a borrowed string"));

        err!(byte_buf: ByteBuf = "&lt;escaped&#x20;string"
                => Unsupported("byte arrays are not supported as `xs:list` items"));
        err!(borrowed_bytes: Bytes = "non-escaped string"
                => Unsupported("byte arrays are not supported as `xs:list` items"));

        deserialized_to!(option_none: Option<&str> = "" => None);
        deserialized_to!(option_some: Option<&str> = "non-escaped string" => Some("non-escaped string"));

        deserialized_to!(unit: () = "<root>anything</root>" => ());
        deserialized_to!(unit_struct: Unit = "<root>anything</root>" => Unit);

        deserialized_to!(newtype_owned: Newtype = "&lt;escaped&#x20;string" => Newtype("<escaped string".into()));
        deserialized_to!(newtype_borrowed: BorrowedNewtype = "non-escaped string" => BorrowedNewtype("non-escaped string"));

        err!(seq: Vec<()> = "non-escaped string"
                => Unsupported("sequences are not supported as `xs:list` items"));
        err!(tuple: ((), ()) = "non-escaped string"
                => Unsupported("tuples are not supported as `xs:list` items"));
        err!(tuple_struct: ((), ()) = "non-escaped string"
                => Unsupported("tuples are not supported as `xs:list` items"));

        err!(map: HashMap<(), ()> = "non-escaped string"
                => Unsupported("maps are not supported as `xs:list` items"));
        err!(struct_: Struct = "non-escaped string"
                => Unsupported("structures are not supported as `xs:list` items"));

        deserialized_to!(enum_unit: Enum = "Unit" => Enum::Unit);
        err!(enum_newtype: Enum = "Newtype"
                => Unsupported("enum newtype variants are not supported as `xs:list` items"));
        err!(enum_tuple: Enum = "Tuple"
                => Unsupported("enum tuple variants are not supported as `xs:list` items"));
        err!(enum_struct: Enum = "Struct"
                => Unsupported("enum struct variants are not supported as `xs:list` items"));
        err!(enum_other: Enum = "any data"
                => Custom("unknown variant `any data`, expected one of `Unit`, `Newtype`, `Tuple`, `Struct`"));

        deserialized_to!(identifier: Id = "Field" => Id::Field);
        deserialized_to!(ignored_any: Any = "any data" => Any(IgnoredAny));

        /// Checks that deserialization from an owned content is working
        #[test]
        fn owned_data() {
            let de = AtomicDeserializer {
                content: Content::Owned("string slice".into(), 7),
                escaped: true,
            };
            assert_eq!(de.content.as_str(), "slice");

            let data: String = Deserialize::deserialize(de).unwrap();
            assert_eq!(data, "slice");
        }

        /// Checks that deserialization from a content borrowed from some
        /// buffer other that input is working
        #[test]
        fn borrowed_from_deserializer() {
            let de = AtomicDeserializer {
                content: Content::Slice("string slice"),
                escaped: true,
            };
            assert_eq!(de.content.as_str(), "string slice");

            let data: String = Deserialize::deserialize(de).unwrap();
            assert_eq!(data, "string slice");
        }
    }
}

//! Serde `Deserializer` module

use crate::{errors::serialize::DeError, errors::Error, escape::unescape, reader::Decoder};
use serde::de::{self, Visitor};
use serde::{self, forward_to_deserialize_any};
use std::borrow::Cow;

/// A deserializer for a xml escaped and encoded value
///
/// # Note
///
/// Escaping the value is actually not always necessary, for instance
/// when converting to float, we don't expect any escapable character
/// anyway
#[derive(Clone)]
pub(crate) struct EscapedDeserializer {
    decoder: Decoder,
    /// Possible escaped value of text/CDATA or attribute value
    escaped_value: Vec<u8>,
    /// If `true`, value requires unescaping before using
    escaped: bool,
}

impl EscapedDeserializer {
    pub fn new(escaped_value: Vec<u8>, decoder: Decoder, escaped: bool) -> Self {
        EscapedDeserializer {
            decoder,
            escaped_value,
            escaped,
        }
    }
    fn unescaped(&self) -> Result<Cow<[u8]>, DeError> {
        if self.escaped {
            unescape(&self.escaped_value).map_err(|e| DeError::Xml(Error::EscapeError(e)))
        } else {
            Ok(Cow::Borrowed(&self.escaped_value))
        }
    }
}

macro_rules! deserialize_num {
    ($method:ident, $visit:ident) => {
        fn $method<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            #[cfg(not(feature = "encoding"))]
            let value = self.decoder.decode(&self.escaped_value)?.parse()?;

            #[cfg(feature = "encoding")]
            let value = self.decoder.decode(&self.escaped_value).parse()?;

            visitor.$visit(value)
        }
    };
}

impl<'de> serde::Deserializer<'de> for EscapedDeserializer {
    type Error = DeError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let unescaped = self.unescaped()?;
        #[cfg(not(feature = "encoding"))]
        let value = self.decoder.decode(&unescaped)?;

        #[cfg(feature = "encoding")]
        let value = self.decoder.decode(&unescaped);
        visitor.visit_str(&value)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let v = self.unescaped()?;
        visitor.visit_bytes(&v)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        #[cfg(feature = "encoding")]
        {
            #[cfg(feature = "encoding")]
            let value = self.decoder.decode(&self.escaped_value);

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
            match &*self.escaped_value {
                b"true" | b"1" | b"True" | b"TRUE" | b"t" | b"Yes" | b"YES" | b"yes" | b"y" => {
                    visitor.visit_bool(true)
                }
                b"false" | b"0" | b"False" | b"FALSE" | b"f" | b"No" | b"NO" | b"no" | b"n" => {
                    visitor.visit_bool(false)
                }
                e => Err(DeError::InvalidBoolean(self.decoder.decode(e)?.into())),
            }
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if self.escaped_value.is_empty() {
            visitor.visit_unit()
        } else {
            Err(DeError::InvalidUnit(
                "Expecting unit, got non empty attribute".into(),
            ))
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if self.escaped_value.is_empty() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_enum<V: de::Visitor<'de>>(
        self,
        _name: &str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_enum(self)
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

    deserialize_num!(deserialize_i64, visit_i64);
    deserialize_num!(deserialize_i32, visit_i32);
    deserialize_num!(deserialize_i16, visit_i16);
    deserialize_num!(deserialize_i8, visit_i8);
    deserialize_num!(deserialize_u64, visit_u64);
    deserialize_num!(deserialize_u32, visit_u32);
    deserialize_num!(deserialize_u16, visit_u16);
    deserialize_num!(deserialize_u8, visit_u8);
    deserialize_num!(deserialize_f64, visit_f64);
    deserialize_num!(deserialize_f32, visit_f32);

    forward_to_deserialize_any! {
        unit_struct seq tuple tuple_struct map struct identifier ignored_any
    }
}

impl<'de> de::EnumAccess<'de> for EscapedDeserializer {
    type Error = DeError;
    type Variant = Self;

    fn variant_seed<V: de::DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self), DeError> {
        let name = seed.deserialize(self.clone())?;
        Ok((name, self))
    }
}

impl<'de> de::VariantAccess<'de> for EscapedDeserializer {
    type Error = DeError;

    fn unit_variant(self) -> Result<(), DeError> {
        Ok(())
    }

    fn newtype_variant_seed<T: de::DeserializeSeed<'de>>(
        self,
        seed: T,
    ) -> Result<T::Value, DeError> {
        seed.deserialize(self)
    }

    fn tuple_variant<V: de::Visitor<'de>>(
        self,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, DeError> {
        unimplemented!()
    }

    fn struct_variant<V: de::Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, DeError> {
        unimplemented!()
    }
}

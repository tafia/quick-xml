//! Serde `Deserializer` module

use crate::{de::errors::DeError, errors::Error, escape::unescape, reader::Decoder};
use serde::de::Visitor;
use serde::{self, forward_to_deserialize_any};
use std::borrow::Cow;

/// A deserializer for a xml escaped and encoded value
///
/// # Note
///
/// Escaping the value is actually not always necessary, for instance
/// when converting to float, we don't expect any escapable character
/// anyway
pub(crate) struct EscapedDeserializer {
    pub decoder: Decoder,
    pub escaped_value: Vec<u8>,
    pub escaped: bool,
}

impl EscapedDeserializer {
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
            let v = self.decoder.decode(&self.escaped_value)?;
            visitor.$visit(v.parse()?)
        }
    }
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
        let v = self.unescaped()?;
        visitor.visit_str(self.decoder.decode(&v)?)
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
        match &*self.decoder.decode(&self.escaped_value)? {
            "TRUE" | "true" | "True" | "1" => visitor.visit_bool(true),
            _ => visitor.visit_bool(false),
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let s = self.decoder.decode(&self.escaped_value)?;
        visitor.visit_char(s.chars().next().expect("s not empty"))
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if self.escaped_value.is_empty() {
            visitor.visit_unit()
        } else {
            Err(DeError::Custom(
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

    //fn deserialize_enum<V: de::Visitor<'de>>(
    //    self,
    //    _name: &str,
    //    _variants: &'static [&'static str],
    //    visitor: V,
    //) -> Result<V::Value, Self::Error> {
    //    visitor.visit_enum(self.0.into_deserializer())
    //}

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
        unit_struct seq tuple tuple_struct map struct enum identifier ignored_any
    }
}

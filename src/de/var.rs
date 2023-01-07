use crate::{
    de::simple_type::SimpleTypeDeserializer,
    de::{deserialize_bool, DeEvent, Deserializer, XmlRead, TEXT_KEY},
    encoding::Decoder,
    errors::serialize::DeError,
    escape::unescape,
};
use serde::de::value::StrDeserializer;
use serde::de::{self, DeserializeSeed, Deserializer as _, Visitor};
use serde::{forward_to_deserialize_any, serde_if_integer128};
use std::borrow::Cow;

/// An enum access
pub struct EnumAccess<'de, 'a, R>
where
    R: XmlRead<'de>,
{
    de: &'a mut Deserializer<'de, R>,
}

impl<'de, 'a, R> EnumAccess<'de, 'a, R>
where
    R: XmlRead<'de>,
{
    pub fn new(de: &'a mut Deserializer<'de, R>) -> Self {
        EnumAccess { de }
    }
}

impl<'de, 'a, R> de::EnumAccess<'de> for EnumAccess<'de, 'a, R>
where
    R: XmlRead<'de>,
{
    type Error = DeError;
    type Variant = VariantAccess<'de, 'a, R>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, VariantAccess<'de, 'a, R>), DeError>
    where
        V: DeserializeSeed<'de>,
    {
        let decoder = self.de.reader.decoder();
        let (name, is_text) = match self.de.peek()? {
            DeEvent::Start(e) => (
                seed.deserialize(VariantDeserializer::new(
                    e.local_name().into_inner(),
                    decoder,
                    false,
                ))?,
                false,
            ),
            DeEvent::Text(_) | DeEvent::CData(_) => (
                seed.deserialize(StrDeserializer::<DeError>::new(TEXT_KEY))?,
                true,
            ),
            DeEvent::End(e) => return Err(DeError::UnexpectedEnd(e.name().into_inner().to_vec())),
            DeEvent::Eof => return Err(DeError::UnexpectedEof),
        };
        Ok((
            name,
            VariantAccess {
                de: self.de,
                is_text,
            },
        ))
    }
}

pub struct VariantAccess<'de, 'a, R>
where
    R: XmlRead<'de>,
{
    de: &'a mut Deserializer<'de, R>,
    /// `true` if variant should be deserialized from a textual content
    /// and `false` if from tag
    is_text: bool,
}

impl<'de, 'a, R> de::VariantAccess<'de> for VariantAccess<'de, 'a, R>
where
    R: XmlRead<'de>,
{
    type Error = DeError;

    fn unit_variant(self) -> Result<(), DeError> {
        match self.de.next()? {
            // Consume subtree
            DeEvent::Start(e) => self.de.read_to_end(e.name()),
            // Does not needed to deserialize using SimpleTypeDeserializer, because
            // it returns `()` when `deserialize_unit()` is requested
            DeEvent::Text(_) | DeEvent::CData(_) => Ok(()),
            // SAFETY: the other events are filtered in `variant_seed()`
            _ => unreachable!("Only `Start`, `Text` or `CData` events are possible here"),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, DeError>
    where
        T: DeserializeSeed<'de>,
    {
        if self.is_text {
            match self.de.next()? {
                DeEvent::Text(e) => {
                    seed.deserialize(SimpleTypeDeserializer::from_text_content(e.decode(true)?))
                }
                DeEvent::CData(e) => {
                    seed.deserialize(SimpleTypeDeserializer::from_text_content(e.decode()?))
                }
                // SAFETY: the other events are filtered in `variant_seed()`
                _ => unreachable!("Only `Text` or `CData` events are possible here"),
            }
        } else {
            seed.deserialize(&mut *self.de)
        }
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        if self.is_text {
            match self.de.next()? {
                DeEvent::Text(e) => SimpleTypeDeserializer::from_text_content(e.decode(true)?)
                    .deserialize_tuple(len, visitor),
                DeEvent::CData(e) => SimpleTypeDeserializer::from_text_content(e.decode()?)
                    .deserialize_tuple(len, visitor),
                // SAFETY: the other events are filtered in `variant_seed()`
                _ => unreachable!("Only `Text` or `CData` events are possible here"),
            }
        } else {
            self.de.deserialize_tuple(len, visitor)
        }
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        if self.is_text {
            match self.de.next()? {
                DeEvent::Text(e) => SimpleTypeDeserializer::from_text_content(e.decode(true)?)
                    .deserialize_struct("", fields, visitor),
                DeEvent::CData(e) => SimpleTypeDeserializer::from_text_content(e.decode()?)
                    .deserialize_struct("", fields, visitor),
                // SAFETY: the other events are filtered in `variant_seed()`
                _ => unreachable!("Only `Text` or `CData` events are possible here"),
            }
        } else {
            self.de.deserialize_struct("", fields, visitor)
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// A deserializer for a xml escaped and encoded value
///
/// # Note
///
/// Escaping the value is actually not always necessary, for instance
/// when converting to float, we don't expect any escapable character
/// anyway
#[derive(Clone, Debug)]
struct VariantDeserializer<'a> {
    /// Possible escaped value of text/CDATA or tag name value
    escaped_value: &'a [u8],
    /// If `true`, value requires unescaping before using
    escaped: bool,
    decoder: Decoder,
}

impl<'a> VariantDeserializer<'a> {
    pub fn new(escaped_value: &'a [u8], decoder: Decoder, escaped: bool) -> Self {
        Self {
            decoder,
            escaped_value,
            escaped,
        }
    }
}

macro_rules! deserialize_num {
    ($method:ident, $visit:ident) => {
        fn $method<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            let value = self.decoder.decode(self.escaped_value)?.parse()?;

            visitor.$visit(value)
        }
    };
}

impl<'de, 'a> de::Deserializer<'de> for VariantDeserializer<'a> {
    type Error = DeError;

    deserialize_num!(deserialize_i8, visit_i8);
    deserialize_num!(deserialize_i16, visit_i16);
    deserialize_num!(deserialize_i32, visit_i32);
    deserialize_num!(deserialize_i64, visit_i64);

    deserialize_num!(deserialize_u8, visit_u8);
    deserialize_num!(deserialize_u16, visit_u16);
    deserialize_num!(deserialize_u32, visit_u32);
    deserialize_num!(deserialize_u64, visit_u64);

    deserialize_num!(deserialize_f32, visit_f32);
    deserialize_num!(deserialize_f64, visit_f64);

    serde_if_integer128! {
        deserialize_num!(deserialize_i128, visit_i128);
        deserialize_num!(deserialize_u128, visit_u128);
    }

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
        let decoded = self.decoder.decode(self.escaped_value)?;
        if self.escaped {
            match unescape(&decoded)? {
                Cow::Borrowed(s) => visitor.visit_str(s),
                Cow::Owned(s) => visitor.visit_string(s),
            }
        } else {
            match decoded {
                Cow::Borrowed(s) => visitor.visit_str(s),
                Cow::Owned(s) => visitor.visit_string(s),
            }
        }
    }

    /// Returns [`DeError::Unsupported`]
    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(DeError::Unsupported(
            "binary data content is not supported by XML format".into(),
        ))
    }

    /// Forwards deserialization to the [`deserialize_bytes`](#method.deserialize_bytes)
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
        deserialize_bool(self.escaped_value, self.decoder, visitor)
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
        visitor.visit_unit()
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

    fn deserialize_enum<V>(
        self,
        _name: &str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
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

    forward_to_deserialize_any! {
        unit_struct seq tuple tuple_struct map struct identifier ignored_any
    }
}

impl<'de, 'a> de::EnumAccess<'de> for VariantDeserializer<'a> {
    type Error = DeError;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let name = seed.deserialize(self.clone())?;
        Ok((name, self))
    }
}

impl<'de, 'a> de::VariantAccess<'de> for VariantDeserializer<'a> {
    type Error = DeError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(self)
    }

    fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
}

use crate::{
    de::key::QNameDeserializer,
    de::simple_type::SimpleTypeDeserializer,
    de::{DeEvent, Deserializer, XmlRead, TEXT_KEY},
    errors::serialize::DeError,
};
use serde::de::value::StrDeserializer;
use serde::de::{self, DeserializeSeed, Deserializer as _, Visitor};

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
                seed.deserialize(QNameDeserializer::from_elem(e.name(), decoder)?)?,
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

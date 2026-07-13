use crate::{
    de::key::QNameDeserializer,
    de::map::ElementMapAccess,
    de::simple_type::SimpleTypeDeserializer,
    de::{DeEvent, Deserializer, TEXT_KEY},
    errors::serialize::DeError,
    reader::EntityResolverFactory,
};
use serde::de::value::BorrowedStrDeserializer;
use serde::de::{self, DeserializeSeed, Deserializer as _, Visitor};

/// An enum access
pub struct EnumAccess<'de, 'e, 'd, EF>
where
    EF: EntityResolverFactory<'de>,
{
    de: &'d mut Deserializer<'de, 'e, EF>,
}

impl<'de, 'e, 'd, EF> EnumAccess<'de, 'e, 'd, EF>
where
    EF: EntityResolverFactory<'de>,
{
    pub fn new(de: &'d mut Deserializer<'de, 'e, EF>) -> Self {
        EnumAccess { de }
    }
}

impl<'de, 'e, 'd, EF> de::EnumAccess<'de> for EnumAccess<'de, 'e, 'd, EF>
where
    EF: EntityResolverFactory<'de>,
{
    type Error = DeError;
    type Variant = VariantAccess<'de, 'e, 'd, EF>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let (name, is_text) = match self.de.peek()? {
            DeEvent::Start(e) => (seed.deserialize(QNameDeserializer::from_elem(e)?)?, false),
            DeEvent::Text(_) => (
                seed.deserialize(BorrowedStrDeserializer::<DeError>::new(TEXT_KEY))?,
                true,
            ),
            // SAFETY: The reader is guaranteed that we don't have unmatched tags
            // If we here, then our deserializer has a bug
            DeEvent::End(e) => unreachable!("{:?}", e),
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

pub struct VariantAccess<'de, 'e, 'd, EF>
where
    EF: EntityResolverFactory<'de>,
{
    de: &'d mut Deserializer<'de, 'e, EF>,
    /// `true` if variant should be deserialized from a textual content
    /// and `false` if from tag
    is_text: bool,
}

impl<'de, 'e, 'd, EF> de::VariantAccess<'de> for VariantAccess<'de, 'e, 'd, EF>
where
    EF: EntityResolverFactory<'de>,
{
    type Error = DeError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        match self.de.next()? {
            // Consume subtree
            DeEvent::Start(e) => self.de.read_to_end(e.name()),
            // Does not needed to deserialize using SimpleTypeDeserializer, because
            // it returns `()` when `deserialize_unit()` is requested
            DeEvent::Text(_) => Ok(()),
            // SAFETY: the other events are filtered in `variant_seed()`
            _ => unreachable!("Only `Start` or `Text` events are possible here"),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        if self.is_text {
            match self.de.next()? {
                DeEvent::Text(e) => seed.deserialize(SimpleTypeDeserializer::from_text_content(e)),
                // SAFETY: the other events are filtered in `variant_seed()`
                _ => unreachable!("Only `Text` events are possible here"),
            }
        } else {
            seed.deserialize(self.de)
        }
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if self.is_text {
            match self.de.next()? {
                DeEvent::Text(e) => {
                    SimpleTypeDeserializer::from_text_content(e).deserialize_tuple(len, visitor)
                }
                // SAFETY: the other events are filtered in `variant_seed()`
                _ => unreachable!("Only `Text` events are possible here"),
            }
        } else {
            self.de.deserialize_tuple(len, visitor)
        }
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.de.next()? {
            DeEvent::Start(e) => visitor.visit_map(ElementMapAccess::new(self.de, e, fields)),
            DeEvent::Text(e) => {
                SimpleTypeDeserializer::from_text_content(e).deserialize_struct("", fields, visitor)
            }
            // SAFETY: the other events are filtered in `variant_seed()`
            _ => unreachable!("Only `Start` or `Text` events are possible here"),
        }
    }
}

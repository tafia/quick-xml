use crate::{
    de::{escape::EscapedDeserializer, BorrowingReader, DeEvent, Deserializer},
    errors::serialize::DeError,
};
use serde::de::{self, Deserializer as SerdeDeserializer};

/// An enum access
pub struct EnumAccess<'de, 'a, R: BorrowingReader<'de>> {
    de: &'a mut Deserializer<'de, R>,
}

impl<'de, 'a, R: BorrowingReader<'de>> EnumAccess<'de, 'a, R> {
    pub fn new(de: &'a mut Deserializer<'de, R>) -> Self {
        EnumAccess { de }
    }
}

impl<'de, 'a, R: BorrowingReader<'de>> de::EnumAccess<'de> for EnumAccess<'de, 'a, R> {
    type Error = DeError;
    type Variant = VariantAccess<'de, 'a, R>;

    fn variant_seed<V: de::DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, VariantAccess<'de, 'a, R>), DeError> {
        let decoder = self.de.reader.decoder();
        let de = match self.de.peek()? {
            Some(DeEvent::Text(t)) => EscapedDeserializer::new(t.to_vec(), decoder, true),
            Some(DeEvent::Start(e)) => EscapedDeserializer::new(e.name().to_vec(), decoder, false),
            Some(_) => {
                return Err(DeError::Unsupported(
                    "Invalid event for Enum, expecting `Text` or `Start`",
                ))
            }
            None => return Err(DeError::Eof),
        };
        let name = seed.deserialize(de)?;
        Ok((name, VariantAccess { de: self.de }))
    }
}

pub struct VariantAccess<'de, 'a, R: BorrowingReader<'de>> {
    de: &'a mut Deserializer<'de, R>,
}

impl<'de, 'a, R: BorrowingReader<'de>> de::VariantAccess<'de> for VariantAccess<'de, 'a, R> {
    type Error = DeError;

    fn unit_variant(self) -> Result<(), DeError> {
        match self.de.next()? {
            DeEvent::Start(e) => self.de.read_to_end(e.name()),
            DeEvent::Text(_) => Ok(()),
            _ => unreachable!(),
        }
    }

    fn newtype_variant_seed<T: de::DeserializeSeed<'de>>(
        self,
        seed: T,
    ) -> Result<T::Value, DeError> {
        seed.deserialize(&mut *self.de)
    }

    fn tuple_variant<V: de::Visitor<'de>>(
        self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, DeError> {
        self.de.deserialize_tuple(len, visitor)
    }

    fn struct_variant<V: de::Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, DeError> {
        self.de.deserialize_struct("", fields, visitor)
    }
}

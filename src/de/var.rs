use crate::{
    de::{escape::EscapedDeserializer, Deserializer},
    errors::serialize::DeError,
    events::Event,
};
use serde::de::{self, Deserializer as SerdeDeserializer};
use std::io::BufRead;

/// An enum access
pub struct EnumAccess<'a, R: BufRead> {
    de: &'a mut Deserializer<R>,
}

impl<'a, R: BufRead> EnumAccess<'a, R> {
    pub fn new(de: &'a mut Deserializer<R>) -> Self {
        EnumAccess { de }
    }
}

impl<'de, 'a, R: 'a + BufRead> de::EnumAccess<'de> for EnumAccess<'a, R> {
    type Error = DeError;
    type Variant = VariantAccess<'a, R>;

    fn variant_seed<V: de::DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, VariantAccess<'a, R>), DeError> {
        let decoder = self.de.reader.decoder();
        let de = match self.de.peek()? {
            Some(Event::Text(t)) => EscapedDeserializer::new(t.to_vec(), decoder, true),
            Some(Event::Start(e)) => EscapedDeserializer::new(e.name().to_vec(), decoder, false),
            Some(e) => return Err(DeError::InvalidEnum(e.to_owned())),
            None => return Err(DeError::Eof),
        };
        let name = seed.deserialize(de)?;
        Ok((name, VariantAccess { de: self.de }))
    }
}

pub struct VariantAccess<'a, R: BufRead> {
    de: &'a mut Deserializer<R>,
}

impl<'de, 'a, R: BufRead> de::VariantAccess<'de> for VariantAccess<'a, R> {
    type Error = DeError;

    fn unit_variant(self) -> Result<(), DeError> {
        match self.de.next(&mut Vec::new())? {
            Event::Start(e) => self.de.read_to_end(e.name()),
            Event::Text(_) => Ok(()),
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

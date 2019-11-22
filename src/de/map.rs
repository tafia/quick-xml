//! Serde `Deserializer` module

use crate::{
    de::{errors::DeError, escape::EscapedDeserializer, Deserializer},
    events::{attributes::Attribute, BytesStart, Event},
    reader::Decoder,
};
use serde::de::{self, DeserializeSeed};
use std::io::BufRead;

enum MapValue {
    Empty,
    Attribute { value: Vec<u8> },
    Nested { name: Vec<u8> },
}

/// A deserializer for `Attributes`
pub(crate) struct MapAccess<'a, R: BufRead> {
    start: BytesStart<'static>,
    de: &'a mut Deserializer<R>,
    decoder: Decoder,
    position: usize,
    value: MapValue,
}

impl<'a, R: BufRead> MapAccess<'a, R> {
    /// Create a new MapAccess
    pub fn new(
        de: &'a mut Deserializer<R>,
        start: BytesStart<'static>,
        decoder: Decoder,
    ) -> Result<Self, DeError> {
        let position = start.attributes().position;
        Ok(MapAccess {
            de,
            start,
            decoder,
            position,
            value: MapValue::Empty,
        })
    }

    fn next_attr(&mut self) -> Result<Option<Attribute>, DeError> {
        let mut attributes = self.start.attributes();
        attributes.position = self.position;
        let next_att = attributes.next();
        self.position = attributes.position;
        Ok(next_att.transpose()?)
    }
}

impl<'a, 'de, R: BufRead> de::MapAccess<'de> for MapAccess<'a, R> {
    type Error = DeError;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        let attr_key_val = self
            .next_attr()?
            .map(|a| (a.key.to_owned(), a.value.into_owned()));
        if let Some((key, value)) = attr_key_val {
            // try getting map from attributes (key= "value")
            self.value = MapValue::Attribute { value };
            seed.deserialize(EscapedDeserializer {
                decoder: self.decoder,
                escaped_value: key,
                escaped: false,
            })
            .map(Some)
        } else {
            // try getting from events (<key>value</key>)
            if let Some(Event::Start(e)) = self.de.peek()? {
                let name = e.name().to_owned();

                let _ = self.de.next(&mut Vec::new())?;
                self.value = MapValue::Nested { name: name.clone() };

                // return key
                seed.deserialize(EscapedDeserializer {
                    decoder: self.decoder,
                    escaped_value: name,
                    escaped: false,
                })
                .map(Some)
            } else {
                self.de.read_to_end(self.start.name())?;
                return Ok(None);
            }
        }
    }

    fn next_value_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<K::Value, Self::Error> {
        dbg!("val");
        match std::mem::replace(&mut self.value, MapValue::Empty) {
            MapValue::Attribute { value } => seed.deserialize(EscapedDeserializer {
                decoder: self.decoder,
                escaped_value: value,
                escaped: true,
            }),
            MapValue::Nested { name } => {
                let value = seed.deserialize(&mut *self.de)?;
                self.de.read_to_end(&name)?;
                Ok(value)
            }
            MapValue::Empty => Err(DeError::EndOfAttributes),
        }
    }
}

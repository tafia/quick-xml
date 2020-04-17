//! Serde `Deserializer` module

use crate::{
    de::{escape::EscapedDeserializer, Deserializer, BorrowingReader, INNER_VALUE},
    errors::serialize::DeError,
    events::{BytesStart, Event},
};
use serde::de::{self, DeserializeSeed, IntoDeserializer};

enum MapValue {
    Empty,
    Attribute { value: Vec<u8> },
    Nested,
    InnerValue,
}

/// A deserializer for `Attributes`
pub(crate) struct MapAccess<'de, 'a, R: BorrowingReader<'de> + 'a> {
    start: BytesStart<'de>,
    de: &'a mut Deserializer<'de, R>,
    position: usize,
    value: MapValue,
}

impl<'de, 'a, R: BorrowingReader<'de>> MapAccess<'de, 'a, R> {
    /// Create a new MapAccess
    pub fn new(de: &'a mut Deserializer<'de, R>, start: BytesStart<'de>) -> Result<Self, DeError> {
        let position = start.attributes().position;
        Ok(MapAccess {
            de,
            start,
            position,
            value: MapValue::Empty,
        })
    }

    fn next_attr(&mut self) -> Result<Option<(Vec<u8>, Vec<u8>)>, DeError> {
        let mut attributes = self.start.attributes();
        attributes.position = self.position;
        let next_att = attributes.next().transpose()?;
        self.position = attributes.position;
        Ok(next_att.map(|a| (a.key.to_owned(), a.value.into_owned())))
    }
}

impl<'de, 'a, R: BorrowingReader<'de> + 'a> de::MapAccess<'de> for MapAccess<'de, 'a, R> {
    type Error = DeError;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        let decoder = self.de.reader.decoder();
        let has_value_field = self.de.has_value_field;
        if let Some((key, value)) = self.next_attr()? {
            // try getting map from attributes (key= "value")
            self.value = MapValue::Attribute { value };
            seed.deserialize(EscapedDeserializer::new(key, decoder, false))
                .map(Some)
        } else {
            // try getting from events (<key>value</key>)
            match self.de.peek()? {
                Some(Event::Text(_)) | Some(Event::Start(_)) if has_value_field => {
                    self.value = MapValue::InnerValue;
                    seed.deserialize(INNER_VALUE.into_deserializer()).map(Some)
                }
                Some(Event::Start(e)) => {
                    let name = e.local_name().to_owned();
                    self.value = MapValue::Nested;
                    seed.deserialize(EscapedDeserializer::new(name, decoder, false))
                        .map(Some)
                }
                _ => Ok(None),
            }
        }
    }

    fn next_value_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<K::Value, Self::Error> {
        match std::mem::replace(&mut self.value, MapValue::Empty) {
            MapValue::Attribute { value } => seed.deserialize(EscapedDeserializer::new(
                value,
                self.de.reader.decoder(),
                true,
            )),
            MapValue::Nested | MapValue::InnerValue => seed.deserialize(&mut *self.de),
            MapValue::Empty => Err(DeError::EndOfAttributes),
        }
    }
}

//! Serde `Deserializer` module

use crate::{
    de::{escape::EscapedDeserializer, Deserializer, INNER_VALUE},
    errors::serialize::DeError,
    events::{attributes::Attribute, BytesStart, Event},
};
use serde::de::{self, DeserializeSeed, IntoDeserializer};
use std::io::BufRead;

enum MapValue {
    Empty,
    Attribute { value: Vec<u8> },
    Nested,
    InnerValue,
}

/// A deserializer for `Attributes`
pub(crate) struct MapAccess<'a, R: BufRead> {
    /// Tag -- owner of attributes
    start: BytesStart<'static>,
    de: &'a mut Deserializer<R>,
    /// Position in flat byte slice of all attributes from which next
    /// attribute should be parsed. This field is required because we
    /// do not store reference to `Attributes` itself but instead create
    /// a new object on each advance of `Attributes` iterator, so we need
    /// to restore last position before advance.
    position: usize,
    value: MapValue,
}

impl<'a, R: BufRead> MapAccess<'a, R> {
    /// Create a new MapAccess
    pub fn new(de: &'a mut Deserializer<R>, start: BytesStart<'static>) -> Result<Self, DeError> {
        let position = start.attributes().position;
        Ok(MapAccess {
            de,
            start,
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
        let decoder = self.de.reader.decoder();
        let has_value_field = self.de.has_value_field;
        if let Some((key, value)) = attr_key_val {
            // try getting map from attributes (key= "value")
            self.value = MapValue::Attribute { value };
            seed.deserialize(EscapedDeserializer::new(key, decoder, false))
                .map(Some)
        } else {
            // try getting from events (<key>value</key>)
            match self.de.peek()? {
                Some(Event::Text(_)) => {
                    self.value = MapValue::InnerValue;
                    seed.deserialize(INNER_VALUE.into_deserializer()).map(Some)
                }
                // Used to deserialize collections of enums, like:
                // <root>
                //   <A/>
                //   <B/>
                //   <C/>
                // </root>
                //
                // into
                //
                // enum Enum { A, B, С }
                // struct Root {
                //     #[serde(rename = "$value")]
                //     items: Vec<Enum>,
                // }
                // TODO: This should be handled by #[serde(flatten)]
                // See https://github.com/serde-rs/serde/issues/1905
                Some(Event::Start(_)) if has_value_field => {
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

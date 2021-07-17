//! Serde `Deserializer` module

use crate::{
    de::escape::EscapedDeserializer,
    de::simple_type::SimpleTypeDeserializer,
    de::{BorrowingReader, DeEvent, Deserializer, INNER_VALUE, UNFLATTEN_PREFIX},
    errors::serialize::DeError,
    events::attributes::Attribute,
    events::BytesStart,
};
use serde::de::{self, DeserializeSeed, IntoDeserializer};
use std::borrow::Cow;

/// Representing state of the `MapAccess` accessor.
enum State {
    /// `next_key_seed` not yet called. This is initial state and state after deserializing
    /// value (calling `next_value_seed`).
    Empty,
    /// `next_key_seed` checked the attributes list and find it is not exhausted yet.
    /// Next call to the `next_value_seed` will deserialize type from the attribute value
    Attribute,
    /// Next event returned will be a [`DeEvent::Start`], which represents a key.
    /// Value should be deserialized from that XML node:
    ///
    /// ```xml
    /// <any-tag>
    ///     <key>...</key>
    /// <!--^^^^^^^^^^^^^^ - this node will be used to deserialize map value -->
    /// </any-tag>
    /// ```
    Nested,
    /// Value should be deserialized from the text content of the XML node:
    ///
    /// ```xml
    /// <any-tag>
    ///     <key>text content</key>
    /// <!--     ^^^^^^^^^^^^ - this will be used to deserialize map value -->
    /// </any-tag>
    /// ```
    InnerValue,
}

/// A deserializer for `Attributes`
pub(crate) struct MapAccess<'de, 'a, R: BorrowingReader<'de>> {
    /// Tag -- owner of attributes
    start: BytesStart<'de>,
    de: &'a mut Deserializer<'de, R>,
    /// Position in flat byte slice of all attributes from which next
    /// attribute should be parsed. This field is required because we
    /// do not store reference to `Attributes` itself but instead create
    /// a new object on each advance of `Attributes` iterator, so we need
    /// to restore last position before advance.
    position: usize,
    /// Current state of the accessor that determines what next call to API
    /// methods should return.
    state: State,
    /// list of fields yet to unflatten (defined as starting with $unflatten=)
    unflatten_fields: Vec<&'static [u8]>,
}

impl<'de, 'a, R: BorrowingReader<'de>> MapAccess<'de, 'a, R> {
    /// Create a new MapAccess
    pub fn new(
        de: &'a mut Deserializer<'de, R>,
        start: BytesStart<'de>,
        fields: &[&'static str],
    ) -> Result<Self, DeError> {
        let position = start.attributes().position;
        Ok(MapAccess {
            de,
            start,
            position,
            state: State::Empty,
            unflatten_fields: fields
                .iter()
                .filter(|f| f.starts_with(UNFLATTEN_PREFIX))
                .map(|f| f.as_bytes())
                .collect(),
        })
    }

    fn next_attr(&mut self) -> Result<Option<Attribute>, DeError> {
        let mut attributes = self.start.attributes();
        attributes.position = self.position;
        let next_att = attributes.next().transpose()?;
        self.position = attributes.position;
        Ok(next_att)
    }
}

impl<'de, 'a, R: BorrowingReader<'de>> de::MapAccess<'de> for MapAccess<'de, 'a, R> {
    type Error = DeError;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        let decoder = self.de.reader.decoder();
        let has_value_field = self.de.has_value_field;

        let mut attributes = self.start.attributes();
        attributes.position = self.position;
        if let Some(a) = attributes.next().transpose()? {
            // try getting map from attributes (key= "value")
            self.state = State::Attribute;
            seed.deserialize(EscapedDeserializer::new(
                Cow::Borrowed(a.key),
                decoder,
                false,
            ))
            .map(Some)
        } else {
            // try getting from events (<key>value</key>)
            match self.de.peek()? {
                DeEvent::Text(_) | DeEvent::CData(_) => {
                    self.state = State::InnerValue;
                    // Deserialize `key` from special attribute name which means
                    // that value should be taken from the text content of the
                    // XML node
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
                DeEvent::Start(_) if has_value_field => {
                    self.state = State::Nested;
                    seed.deserialize(INNER_VALUE.into_deserializer()).map(Some)
                }
                DeEvent::Start(e) => {
                    let key = if let Some(p) = self
                        .unflatten_fields
                        .iter()
                        .position(|f| e.name() == &f[UNFLATTEN_PREFIX.len()..])
                    {
                        // Used to deserialize elements, like:
                        // <root>
                        //   <xxx>test</xxx>
                        // </root>
                        //
                        // into
                        //
                        // struct Root {
                        //     #[serde(rename = "$unflatten=xxx")]
                        //     xxx: String,
                        // }
                        self.state = State::Nested;
                        seed.deserialize(self.unflatten_fields.remove(p).into_deserializer())
                    } else {
                        let name = Cow::Borrowed(e.local_name());
                        self.state = State::Nested;
                        seed.deserialize(EscapedDeserializer::new(name, decoder, false))
                    };
                    key.map(Some)
                }
                _ => Ok(None),
            }
        }
    }

    fn next_value_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<K::Value, Self::Error> {
        match std::mem::replace(&mut self.state, State::Empty) {
            State::Attribute => {
                let decoder = self.de.reader.decoder();
                match self.next_attr()? {
                    Some(a) => {
                        //FIXME: we have to clone value because of wrong lifetimes on `a`
                        // It should be bound to the input lifetime, but it instead bound
                        // to a deserializer lifetime
                        let value: Vec<_> = a.value.into_owned();
                        seed.deserialize(SimpleTypeDeserializer::new(value.into(), true, decoder))
                    }
                    // We set `Attribute` state only when we are sure that `next_attr()` returns a value
                    None => unreachable!(),
                }
            }
            // This case are checked by "de::tests::xml_schema_lists::element" tests
            State::InnerValue => {
                let decoder = self.de.reader.decoder();
                match self.de.next()? {
                    DeEvent::Text(e) => {
                        //TODO: It is better to store event content as part of state
                        seed.deserialize(SimpleTypeDeserializer::new(e.into_inner(), true, decoder))
                    }
                    // It is better to format similar code similarly, but rustfmt disagree
                    #[rustfmt::skip]
                    DeEvent::CData(e) => {
                        //TODO: It is better to store event content as part of state
                        seed.deserialize(SimpleTypeDeserializer::new(e.into_inner(), false, decoder))
                    }
                    // SAFETY: We set `InnerValue` only when we seen `Text` or `CData`
                    _ => unreachable!(),
                }
            }
            State::Nested => seed.deserialize(&mut *self.de),
            State::Empty => Err(DeError::EndOfAttributes),
        }
    }
}

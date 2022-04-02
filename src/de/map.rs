//! Serde `Deserializer` module

use crate::{
    de::escape::EscapedDeserializer,
    de::{deserialize_bool, DeEvent, Deserializer, XmlRead, INNER_VALUE, UNFLATTEN_PREFIX},
    errors::serialize::DeError,
    events::attributes::IterState,
    events::{BytesCData, BytesStart},
    reader::Decoder,
};
use serde::de::{self, DeserializeSeed, IntoDeserializer, Visitor};
use serde::serde_if_integer128;
use std::borrow::Cow;
use std::ops::Range;

/// Defines a source that should be used to deserialize a value in the next call
/// to [`next_value_seed()`](de::MapAccess::next_value_seed)
#[derive(Debug, PartialEq)]
enum ValueSource {
    /// Source are not specified, because [`next_key_seed()`] not yet called.
    /// This is an initial state and state after deserializing value
    /// (after call of [`next_value_seed()`]).
    ///
    /// Attempt to call [`next_value_seed()`] while accessor in this state would
    /// return a [`DeError::KeyNotRead`] error.
    ///
    /// [`next_key_seed()`]: de::MapAccess::next_key_seed
    /// [`next_value_seed()`]: de::MapAccess::next_value_seed
    Unknown,
    /// Next value should be deserialized from an attribute value; value is located
    /// at specified span.
    Attribute(Range<usize>),
    /// Value should be deserialized from the text content of the XML node, which
    /// represented or by an ordinary text node, or by a CDATA node:
    ///
    /// ```xml
    /// <...>text content for field value<...>
    /// ```
    /// ```xml
    /// <any-tag>
    ///     <key><![CDATA[cdata content]]></key>
    /// <!--              ^^^^^^^^^^^^^ - this will be used to deserialize a map value -->
    /// </any-tag>
    /// ```
    Text,
    /// Next value should be deserialized from an element with an any name.
    /// Corresponding tag name will always be associated with a field with name
    /// [`INNER_VALUE`].
    ///
    /// That state is set when call to [`peek()`] returns a [`Start`] event
    /// _and_ struct has a field with a special name [`INNER_VALUE`].
    ///
    /// When in this state, next event, returned by [`next()`], will be a [`Start`],
    /// which represents both a key, and a value. Value would be deserialized from
    /// the whole element and how is will be done determined by the value deserializer.
    /// The [`MapAccess`] do not consume any events in that state.
    ///
    /// Because in that state any encountered `<tag>` is mapped to the [`INNER_VALUE`]
    /// field, it is possible to use tag name as an enum discriminator, so `enum`s
    /// can be deserialized from that XMLs:
    ///
    /// ```xml
    /// <any-tag>
    ///     <variant1>...</variant1>
    /// <!-- ~~~~~~~~               - this data will determine that this is Enum::variant1 -->
    /// <!--^^^^^^^^^^^^^^^^^^^^^^^ - this data will be used to deserialize a map value -->
    /// </any-tag>
    /// ```
    /// ```xml
    /// <any-tag>
    ///     <variant2>...</variant2>
    /// <!-- ~~~~~~~~               - this data will determine that this is Enum::variant2 -->
    /// <!--^^^^^^^^^^^^^^^^^^^^^^^ - this data will be used to deserialize a map value -->
    /// </any-tag>
    /// ```
    ///
    /// both can be deserialized into
    ///
    /// ```ignore
    /// enum Enum {
    ///   variant1,
    ///   variant2,
    /// }
    /// struct AnyName {
    ///   #[serde(rename = "$value")]
    ///   field: Enum,
    /// }
    /// ```
    ///
    /// That is possible, because value deserializer have access to the full content
    /// of a `<variant1>...</variant1>` or `<variant2>...</variant2>` node, including
    /// the tag name.
    ///
    /// Currently, processing of that enum variant is fully equivalent to the
    /// processing of a [`Text`] variant. Split of variants made for clarity.
    ///
    /// [`Start`]: DeEvent::Start
    /// [`peek()`]: Deserializer::peek()
    /// [`next()`]: Deserializer::next()
    /// [`Text`]: Self::Text
    Content,
    /// Next value should be deserialized from an element with a dedicated name.
    ///
    /// That state is set when call to [`peek()`] returns a [`Start`] event, which
    /// [`name()`] represents a field name. That name will be deserialized as a key.
    ///
    /// When in this state, next event, returned by [`next()`], will be a [`Start`],
    /// which represents both a key, and a value. Value would be deserialized from
    /// the whole element and how is will be done determined by the value deserializer.
    /// The [`MapAccess`] do not consume any events in that state.
    ///
    /// An illustration below shows, what data is used to deserialize key and value:
    /// ```xml
    /// <any-tag>
    ///     <key>...</key>
    /// <!-- ~~~           - this data will be used to deserialize a map key -->
    /// <!--^^^^^^^^^^^^^^ - this data will be used to deserialize a map value -->
    /// </any-tag>
    /// ```
    ///
    /// Although value deserializer will have access to the full content of a `<key>`
    /// node (including the tag name), it will not get much benefits from that,
    /// because tag name will always be fixed for a given map field (equal to a
    /// field name). So, if the field type is an `enum`, it cannot select its
    /// variant based on the tag name. If that is needed, then [`Content`] variant
    /// of this enum should be used. Such usage is enabled by annotating a struct
    /// field as "content" field, which implemented as given the field a special
    /// [`INNER_VALUE`] name.
    ///
    /// [`Start`]: DeEvent::Start
    /// [`peek()`]: Deserializer::peek()
    /// [`next()`]: Deserializer::next()
    /// [`name()`]: BytesStart::name()
    /// [`Content`]: Self::Content
    Nested,
}

/// A deserializer for `Attributes`
pub(crate) struct MapAccess<'de, 'a, R>
where
    R: XmlRead<'de>,
{
    /// Tag -- owner of attributes
    start: BytesStart<'de>,
    de: &'a mut Deserializer<'de, R>,
    /// State of the iterator over attributes. Contains the next position in the
    /// inner `start` slice, from which next attribute should be parsed.
    iter: IterState,
    /// Current state of the accessor that determines what next call to API
    /// methods should return.
    source: ValueSource,
    /// list of fields yet to unflatten (defined as starting with $unflatten=)
    unflatten_fields: Vec<&'static [u8]>,
}

impl<'de, 'a, R> MapAccess<'de, 'a, R>
where
    R: XmlRead<'de>,
{
    /// Create a new MapAccess
    pub fn new(
        de: &'a mut Deserializer<'de, R>,
        start: BytesStart<'de>,
        fields: &[&'static str],
    ) -> Result<Self, DeError> {
        Ok(MapAccess {
            de,
            start,
            iter: IterState::new(0, false),
            source: ValueSource::Unknown,
            unflatten_fields: fields
                .iter()
                .filter(|f| f.starts_with(UNFLATTEN_PREFIX))
                .map(|f| f.as_bytes())
                .collect(),
        })
    }
}

impl<'de, 'a, R> de::MapAccess<'de> for MapAccess<'de, 'a, R>
where
    R: XmlRead<'de>,
{
    type Error = DeError;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        debug_assert_eq!(self.source, ValueSource::Unknown);

        // FIXME: There error positions counted from end of tag name - need global position
        let slice = self.start.attributes_raw();
        let decoder = self.de.reader.decoder();
        let has_value_field = self.de.has_value_field;

        if let Some(a) = self.iter.next(slice).transpose()? {
            // try getting map from attributes (key= "value")
            let (key, value) = a.into();
            self.source = ValueSource::Attribute(value.unwrap_or_default());
            seed.deserialize(EscapedDeserializer::new(
                Cow::Borrowed(&slice[key]),
                decoder,
                false,
            ))
            .map(Some)
        } else {
            // try getting from events (<key>value</key>)
            match self.de.peek()? {
                DeEvent::Text(_) | DeEvent::CData(_) => {
                    self.source = ValueSource::Text;
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
                    self.source = ValueSource::Content;
                    seed.deserialize(INNER_VALUE.into_deserializer()).map(Some)
                }
                DeEvent::Start(e) => {
                    self.source = ValueSource::Nested;
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
                        seed.deserialize(self.unflatten_fields.remove(p).into_deserializer())
                    } else {
                        let name = Cow::Borrowed(e.local_name());
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
        match std::mem::replace(&mut self.source, ValueSource::Unknown) {
            ValueSource::Attribute(value) => {
                let slice = self.start.attributes_raw();
                let decoder = self.de.reader.decoder();

                seed.deserialize(EscapedDeserializer::new(
                    Cow::Borrowed(&slice[value]),
                    decoder,
                    true,
                ))
            }
            // This arm processes the following XML shape:
            // <any-tag>
            //   text value
            // </any-tag>
            // The whole map represented by an `<any-tag>` element, the map key
            // is implicit and equals to the `INNER_VALUE` constant, and the value
            // is a `Text` or a `CData` event (the value deserializer will see one
            // of that events)
            ValueSource::Text => seed.deserialize(MapValueDeserializer { map: self }),
            // This arm processes the following XML shape:
            // <any-tag>
            //   <any>...</any>
            // </any-tag>
            // The whole map represented by an `<any-tag>` element, the map key
            // is implicit and equals to the `INNER_VALUE` constant, and the value
            // is a `Start` event (the value deserializer will see that event)
            ValueSource::Content => seed.deserialize(MapValueDeserializer { map: self }),
            // This arm processes the following XML shape:
            // <any-tag>
            //   <tag>...</tag>
            // </any-tag>
            // The whole map represented by an `<any-tag>` element, the map key
            // is a `tag`, and the value is a `Start` event (the value deserializer
            // will see that event)
            ValueSource::Nested => seed.deserialize(&mut *self.de),
            ValueSource::Unknown => Err(DeError::KeyNotRead),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

macro_rules! forward {
    (
        $deserialize:ident
        $(
            ($($name:ident : $type:ty),*)
        )?
    ) => {
        #[inline]
        fn $deserialize<V: Visitor<'de>>(
            self,
            $($($name: $type,)*)?
            visitor: V
        ) -> Result<V::Value, Self::Error> {
            self.map.de.$deserialize($($($name,)*)? visitor)
        }
    };
}

/// A deserializer for a value of map or struct. That deserializer slightly
/// differently processes events for a primitive types and sequences than
/// a [`Deserializer`].
struct MapValueDeserializer<'de, 'a, 'm, R>
where
    R: XmlRead<'de>,
{
    /// Access to the map that created this deserializer. Gives access to the
    /// context, such as list of fields, that current map known about.
    map: &'m mut MapAccess<'de, 'a, R>,
}

impl<'de, 'a, 'm, R> MapValueDeserializer<'de, 'a, 'm, R>
where
    R: XmlRead<'de>,
{
    /// Returns a text event, used inside [`deserialize_primitives!()`]
    #[inline]
    fn next_text(&mut self, unescape: bool) -> Result<BytesCData<'de>, DeError> {
        self.map.de.next_text_impl(unescape, false)
    }

    /// Returns a decoder, used inside [`deserialize_primitives!()`]
    #[inline]
    fn decoder(&self) -> Decoder {
        self.map.de.reader.decoder()
    }
}

impl<'de, 'a, 'm, R> de::Deserializer<'de> for MapValueDeserializer<'de, 'a, 'm, R>
where
    R: XmlRead<'de>,
{
    type Error = DeError;

    deserialize_primitives!(mut);

    forward!(deserialize_option);
    forward!(deserialize_unit);
    forward!(deserialize_unit_struct(name: &'static str));
    forward!(deserialize_newtype_struct(name: &'static str));

    forward!(deserialize_seq);
    forward!(deserialize_tuple(len: usize));
    forward!(deserialize_tuple_struct(name: &'static str, len: usize));

    forward!(deserialize_map);
    forward!(deserialize_struct(
        name: &'static str,
        fields: &'static [&'static str]
    ));

    forward!(deserialize_enum(
        name: &'static str,
        variants: &'static [&'static str]
    ));

    forward!(deserialize_any);
    forward!(deserialize_ignored_any);

    #[inline]
    fn is_human_readable(&self) -> bool {
        self.map.de.is_human_readable()
    }
}

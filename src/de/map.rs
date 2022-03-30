//! Serde `Deserializer` module

use crate::{
    de::escape::EscapedDeserializer,
    de::seq::is_unknown,
    de::simple_type::SimpleTypeDeserializer,
    de::{deserialize_bool, BorrowingReader, DeEvent, Deserializer, INNER_VALUE, UNFLATTEN_PREFIX},
    errors::serialize::DeError,
    events::attributes::Attribute,
    events::{BytesCData, BytesStart},
    reader::Decoder,
};
use serde::de::{self, DeserializeSeed, IntoDeserializer, Visitor};
use serde::serde_if_integer128;
use std::borrow::Cow;

/// Defines a source that should be used to deserialize a value in next call of
/// [ `next_value_seed()`](de::MapAccess::next_value_seed)
#[derive(Debug, PartialEq)]
enum ValueSource {
    /// Source not specified, because `next_key_seed` not yet called.
    /// This is an initial state and state after deserializing value
    /// (after call of `next_value_seed`).
    ///
    /// Attempt to call `next_value_seed` while accessor in this state would
    /// return a [`DeError::KeyNotRead`] error.
    Unknown,
    /// Next value should be deserialized from an attribute value.
    Attribute,
    /// Next value should be deserialized from an element with a dedicated name.
    /// If deserialized type is a sequence, then that sequence will collect all
    /// elements with the same name until it will be filled. If not all elements
    /// would be consumed, the rest will be ignored.
    ///
    /// That state is set when call to [`peek()`] returns a [`Start`] event, which
    /// [`name()`] represents a tag name. That name will be deserialized as a key.
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
    /// <!--^^^^^^^^^^^^^^ - this node will be used to deserialize a map value -->
    /// </any-tag>
    /// ```
    ///
    /// Although value deserializer will have access to the full content of a `<key>`
    /// node, it should not use the name of the node (`key`) in their implementation.
    /// This is because this name always be fixed and equal to the map or struct key,
    /// therefore a little sense to use it.
    ///
    /// Similar [`Self::Content`] variant is used, when value deserializer wants to
    /// use `key` with benefit.
    ///
    /// [`peek()`]: Deserializer::peek()
    /// [`next()`]: Deserializer::next()
    /// [`Start`]: DeEvent::Start
    /// [`name()`]: BytesStart::name()
    Nested,
    /// Next value should be deserialized from an element with an any name, except
    /// names, listed individually. Corresponding tag name will always associated
    /// with a field with name [`INNER_VALUE`].
    ///
    /// That state is set when call to [`peek()`] returns a [`Start`] event, which
    /// [`name()`] represents a tag name _and_ that tag name is not listed in the
    /// list of known fields (which for a struct would be a list with field names).
    ///
    /// The behavior in that state is mostly identical to that in [`Self::Nested`],
    /// but because key is not strictly defined, it would be worth to use it when
    /// deserialize value, which means, that you can deserialize enums from it --
    /// `key` would be used as discriminator.
    ///
    /// [`peek()`]: Deserializer::peek()
    /// [`Start`]: DeEvent::Start
    /// [`name()`]: BytesStart::name()
    Content,
    /// Value should be deserialized from the text content of the XML node, which
    /// represented or by ordinary text node, or by CDATA node:
    ///
    /// ```xml
    /// <any-tag>
    ///     <key>text content</key>
    /// <!--     ^^^^^^^^^^^^ - this will be used to deserialize map value -->
    /// </any-tag>
    /// ```
    /// ```xml
    /// <any-tag>
    ///     <key><![CDATA[cdata content]]></key>
    /// <!--              ^^^^^^^^^^^^^ - this will be used to deserialize map value -->
    /// </any-tag>
    /// ```
    Text,
}

/// A deserializer that extracts map-like structures from an XML. This
/// deserializer represent a one XML tag:
///
/// ```xml
/// <tag>...</tag>
/// ```
///
/// Name of this tag is stored in a [`Self::start`] property.
///
/// Map keys could be deserialized from three places:
/// - attributes
/// - elements
/// - implicit `#text` node of the element
///
/// Deserialization from attributes is simple -- each attribute name mapped to
/// a map key name and each attribute value mapped to a primitive value
/// (numbers, boolean, strings or unit structs / enum variants) or to an
/// [`xs:list`]. The latter allows deserialize a sequence of primitive types
/// from an attribute value.
///
/// Deserialization from elements more complicated.
///
/// # Lifetimes
///
/// `'de` lifetime represents a buffer, from which deserialized values can
/// borrow their data. Depending on the underlying reader, there can be an
/// internal buffer of deserializer (i.e. deserializer itself) or an input
/// (in that case it is possible to approach zero-cost deserialization).
///
/// `'a` lifetime represents a parent deserializer, which could own the data
/// buffer.
///
/// [`xs:list`]: SimpleTypeDeserializer
pub(crate) struct MapAccess<'de, 'a, R>
where
    R: BorrowingReader<'de>,
{
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
    source: ValueSource,
    /// List of field names of the struct
    fields: &'static [&'static str],
    /// list of fields yet to unflatten (defined as starting with $unflatten=)
    unflatten_fields: Vec<&'static [u8]>,
}

impl<'de, 'a, R> MapAccess<'de, 'a, R>
where
    R: BorrowingReader<'de>,
{
    /// Create a new MapAccess
    pub fn new(
        de: &'a mut Deserializer<'de, R>,
        start: BytesStart<'de>,
        fields: &'static [&'static str],
    ) -> Result<Self, DeError> {
        let position = start.attributes().position;
        Ok(MapAccess {
            de,
            start,
            position,
            source: ValueSource::Unknown,
            fields,
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

impl<'de, 'a, R> de::MapAccess<'de> for MapAccess<'de, 'a, R>
where
    R: BorrowingReader<'de>,
{
    type Error = DeError;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        debug_assert_eq!(self.source, ValueSource::Unknown);

        let decoder = self.de.reader.decoder();
        let has_value_field = self.de.has_value_field;

        let mut attributes = self.start.attributes();
        attributes.position = self.position;
        if let Some(a) = attributes.next().transpose()? {
            // try getting map from attributes (key= "value")
            self.source = ValueSource::Attribute;
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
                DeEvent::Start(e) if has_value_field && is_unknown(self.fields, e, decoder)? => {
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
            ValueSource::Attribute => {
                let decoder = self.de.reader.decoder();
                match self.next_attr()? {
                    Some(a) => {
                        //FIXME: we have to clone value because of wrong lifetimes on `a`
                        // It should be bound to the input lifetime, but it instead bound
                        // to a deserializer lifetime
                        let value: Vec<_> = a.value.into_owned();
                        seed.deserialize(SimpleTypeDeserializer::new(value.into(), true, decoder))
                    }
                    // SAFETY: We set `Attribute` source only when we are sure that `next_attr()` returns a value
                    None => unreachable!(),
                }
            }
            // This case are checked by "de::tests::xml_schema_lists::element" tests
            ValueSource::Text => {
                let decoder = self.de.reader.decoder();
                match self.de.next()? {
                    DeEvent::Text(e) => {
                        //TODO: It is better to store event content as part of source
                        seed.deserialize(SimpleTypeDeserializer::new(e.into_inner(), true, decoder))
                    }
                    // It is better to format similar code similarly, but rustfmt disagree
                    #[rustfmt::skip]
                    DeEvent::CData(e) => {
                        //TODO: It is better to store event content as part of source
                        seed.deserialize(SimpleTypeDeserializer::new(e.into_inner(), false, decoder))
                    }
                    // SAFETY: We set `Text` only when we seen `Text` or `CData`
                    _ => unreachable!(),
                }
            }
            // In both cases we get to a deserializer full access to the data
            // The difference in the handling of sequences by SeqAccess
            ValueSource::Nested => seed.deserialize(MapValueDeserializer {
                map: self,
                content: false,
            }),
            ValueSource::Content => seed.deserialize(MapValueDeserializer {
                map: self,
                content: true,
            }),
            ValueSource::Unknown => Err(DeError::KeyNotRead),
        }
    }
}

//-------------------------------------------------------------------------------------------------

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
/// differently process events for a primitive types and sequences.
struct MapValueDeserializer<'de, 'a, 'm, R>
where
    R: BorrowingReader<'de>,
{
    /// Access to the map that created this deserializer. Gives access to the
    /// context, such as list of fields, that current map known about.
    map: &'m mut MapAccess<'de, 'a, R>,
    /// Determines if field with a dedicated name or a field  that accepts any
    /// content is deserialized. If `true`, the deserialized content is for
    /// [`INNER_VALUE`] field, otherwise for a field which name matched the tag
    /// name (`self.map.start.name()`).
    ///
    /// If `true`, then only text and CDATA events is allowed to return from
    /// `Self::next_text()`, otherwise they also can be returned from one nested
    /// level of tags. This allows deserialize primitives (such as numbers) from
    /// elements that represents structure fields, because all deserializers get
    /// the full data, including surrounding tag, not only tag content.
    content: bool,
}

impl<'de, 'a, 'm, R> MapValueDeserializer<'de, 'a, 'm, R>
where
    R: BorrowingReader<'de>,
{
    /// Returns a text event, used inside `deserialize_primitives!()`
    #[inline]
    fn next_text(&mut self) -> Result<BytesCData<'de>, DeError> {
        self.map.de.next_text_impl(!self.content)
    }

    /// Returns a decoder, used inside `deserialize_primitives!()`
    #[inline]
    fn decoder(&self) -> Decoder {
        self.map.de.reader.decoder()
    }
}

impl<'de, 'a, 'm, R> de::Deserializer<'de> for MapValueDeserializer<'de, 'a, 'm, R>
where
    R: BorrowingReader<'de>,
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

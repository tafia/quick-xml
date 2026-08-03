use crate::{
    de::key::QNameDeserializer,
    de::map::ElementMapAccess,
    de::resolver::EntityResolver,
    de::simple_type::SimpleTypeDeserializer,
    de::{DeEvent, Deserializer, XmlRead, TEXT_KEY},
    errors::{serialize::DeError, Error},
    events::BytesStart,
};
use serde::de::value::BorrowedStrDeserializer;
use serde::de::{self, DeserializeSeed, Deserializer as _, Visitor};
use std::borrow::Cow;

/// An enum access
pub struct EnumAccess<'de, 'd, R, E>
where
    R: XmlRead<'de>,
    E: EntityResolver,
{
    de: &'d mut Deserializer<'de, R, E>,
}

impl<'de, 'd, R, E> EnumAccess<'de, 'd, R, E>
where
    R: XmlRead<'de>,
    E: EntityResolver,
{
    pub fn new(de: &'d mut Deserializer<'de, R, E>) -> Self {
        EnumAccess { de }
    }
}

impl<'de, 'd, R, E> de::EnumAccess<'de> for EnumAccess<'de, 'd, R, E>
where
    R: XmlRead<'de>,
    E: EntityResolver,
{
    type Error = DeError;
    type Variant = VariantAccess<'de, 'd, R, E>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let name = match self.de.peek()? {
            DeEvent::Start(e) => seed.deserialize(QNameDeserializer::from_elem(e)?)?,
            DeEvent::Text(_) => {
                seed.deserialize(BorrowedStrDeserializer::<DeError>::new(TEXT_KEY))?
            }
            // SAFETY: The reader is guaranteed that we don't have unmatched tags
            // If we here, then our deserializer has a bug
            DeEvent::End(e) => unreachable!("{:?}", e),
            DeEvent::Eof => return Err(DeError::UnexpectedEof),
        };
        Ok((name, VariantAccess { de: self.de }))
    }
}

pub struct VariantAccess<'de, 'd, R, E>
where
    R: XmlRead<'de>,
    E: EntityResolver,
{
    de: &'d mut Deserializer<'de, R, E>,
}

impl<'de, 'd, R, E> de::VariantAccess<'de> for VariantAccess<'de, 'd, R, E>
where
    R: XmlRead<'de>,
    E: EntityResolver,
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
        match self.de.next()? {
            DeEvent::Start(e) => seed.deserialize(VariantContentDeserializer {
                start: e,
                de: self.de,
            }),
            DeEvent::Text(e) => seed.deserialize(SimpleTypeDeserializer::from_text_content(e)),
            // SAFETY: the other events are filtered in `variant_seed()`
            _ => unreachable!("Only `Start` or `Text` events are possible here"),
        }
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.de.peek()? {
            DeEvent::Start(_) => visitor.visit_seq(TupleVariantSeqAccess { de: self.de }),
            _ => match self.de.next()? {
                DeEvent::Text(e) => {
                    SimpleTypeDeserializer::from_text_content(e).deserialize_tuple(len, visitor)
                }
                // SAFETY: the other events are filtered in `variant_seed()`
                _ => unreachable!("Only `Start` or `Text` events are possible here"),
            },
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
            DeEvent::Start(e) => visitor.visit_map(ElementMapAccess::new(self.de, e, fields)?),
            DeEvent::Text(e) => {
                SimpleTypeDeserializer::from_text_content(e).deserialize_struct("", fields, visitor)
            }
            // SAFETY: the other events are filtered in `variant_seed()`
            _ => unreachable!("Only `Start` or `Text` events are possible here"),
        }
    }
}

/// [`SeqAccess`](de::SeqAccess) for tuple variants that consumes each repeated
/// element's Start event before deserializing the field, preventing re-entry for
/// recursive enum types.
///
/// Tuple variants use repeated sibling elements, one per field:
/// `Tuple(f64, String)` serializes as `<Tuple>42</Tuple><Tuple>answer</Tuple>`.
/// Each call to [`next_element_seed`] consumes the next Start event and creates
/// a [`VariantContentDeserializer`] to deserialize the field value within that
/// element (including its End tag).
///
/// [`next_element_seed`]: de::SeqAccess::next_element_seed
struct TupleVariantSeqAccess<'de, 'd, R, E>
where
    R: XmlRead<'de>,
    E: EntityResolver,
{
    de: &'d mut Deserializer<'de, R, E>,
}

impl<'de, 'd, R, E> de::SeqAccess<'de> for TupleVariantSeqAccess<'de, 'd, R, E>
where
    R: XmlRead<'de>,
    E: EntityResolver,
{
    type Error = DeError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        self.de.skip_whitespaces()?;
        match self.de.peek()? {
            DeEvent::Start(_) => match self.de.next()? {
                DeEvent::Start(e) => seed
                    .deserialize(VariantContentDeserializer {
                        start: e,
                        de: self.de,
                    })
                    .map(Some),
                _ => unreachable!(),
            },
            DeEvent::End(_) | DeEvent::Eof => Ok(None),
            _ => seed.deserialize(&mut *self.de).map(Some),
        }
    }
}

/// Deserializer for the content of a variant element whose Start event has
/// already been consumed.
///
/// This differs from [`ElementDeserializer`](super::map::ElementDeserializer)
/// in how it handles `deserialize_enum` and `deserialize_seq`: instead of
/// re-using `self.start` (which would cause infinite re-entry for recursive
/// enum types), it delegates to the stream-based [`Deserializer`] and then
/// consumes the variant element's End tag.
///
/// For `deserialize_struct` and primitives, it behaves identically to
/// `ElementDeserializer`: the consumed `start` is used as the struct root,
/// and text content is read via `read_text`.
struct VariantContentDeserializer<'de, 'd, R, E>
where
    R: XmlRead<'de>,
    E: EntityResolver,
{
    start: BytesStart<'de>,
    de: &'d mut Deserializer<'de, R, E>,
}

impl<'de, 'd, R, E> VariantContentDeserializer<'de, 'd, R, E>
where
    R: XmlRead<'de>,
    E: EntityResolver,
{
    #[inline]
    fn read_string(&mut self) -> Result<Cow<'de, str>, DeError> {
        self.de.read_text(self.start.name())
    }
}

impl<'de, 'd, R, E> de::Deserializer<'de> for VariantContentDeserializer<'de, 'd, R, E>
where
    R: XmlRead<'de>,
    E: EntityResolver,
{
    type Error = DeError;

    deserialize_primitives!(mut);

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.de.read_to_end(self.start.name())?;
        visitor.visit_unit()
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let result = visitor.visit_seq(&mut self)?;
        self.de.read_to_end(self.start.name())?;
        Ok(result)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(ElementMapAccess::new(self.de, self.start, fields)?)
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let VariantContentDeserializer { start, de } = self;
        let result = de.deserialize_enum(name, variants, visitor)?;
        de.read_to_end(start.name())?;
        Ok(result)
    }

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }
}

/// Unlike the top-level [`SeqAccess`](de::SeqAccess) impl on [`Deserializer`]
/// (which only stops on [`DeEvent::Eof`]), this stops on [`DeEvent::End`]
/// because the sequence is bounded by the enclosing variant element's end tag.
/// [`DeEvent::Eof`] is an error (truncated XML).
impl<'de, 'd, R, E> de::SeqAccess<'de> for &mut VariantContentDeserializer<'de, 'd, R, E>
where
    R: XmlRead<'de>,
    E: EntityResolver,
{
    type Error = DeError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        self.de.skip_whitespaces()?;
        match self.de.peek()? {
            DeEvent::End(_) => Ok(None),
            DeEvent::Eof => Err(Error::missed_end(self.start.name()).into()),
            _ => seed
                .deserialize(InTreeDeserializer { de: &mut *self.de })
                .map(Some),
        }
    }
}

/// A wrapper around [`Deserializer`] that rejects `Start` events when
/// deserializing primitive types. Used by [`VariantContentDeserializer`]'s
/// [`SeqAccess`](de::SeqAccess) impl so that `Variant(Vec<i32>)` cannot be
/// deserialized from child elements like `<Variant><x>1</x></Variant>` —
/// only from xs:list text content like `<Variant>1 2 3</Variant>`.
///
/// Non-primitive methods (enum, struct, seq, etc.) delegate directly to the
/// underlying [`Deserializer`], so `Vec<MathNode>` from child elements still
/// works: `<Apply><Ci>x</Ci><Cn>5</Cn></Apply>`.
pub(crate) struct InTreeDeserializer<'de, 'd, R, E>
where
    R: XmlRead<'de>,
    E: EntityResolver,
{
    de: &'d mut Deserializer<'de, R, E>,
}

impl<'de, 'd, R, E> InTreeDeserializer<'de, 'd, R, E>
where
    R: XmlRead<'de>,
    E: EntityResolver,
{
    #[inline]
    fn read_string(&mut self) -> Result<Cow<'de, str>, DeError> {
        self.de.read_string_in_tree()
    }
}

impl<'de, 'd, R, E> de::Deserializer<'de> for InTreeDeserializer<'de, 'd, R, E>
where
    R: XmlRead<'de>,
    E: EntityResolver,
{
    type Error = DeError;

    deserialize_primitives!(mut);

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        self.de.deserialize_unit(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        self.de.deserialize_option(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        self.de.deserialize_newtype_struct(name, visitor)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        self.de.deserialize_seq(visitor)
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        self.de.deserialize_struct(name, fields, visitor)
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        self.de.deserialize_enum(name, variants, visitor)
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        self.de.deserialize_any(visitor)
    }
}

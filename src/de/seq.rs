use crate::de::{DeError, DeEvent, Deserializer, XmlRead};
use crate::events::BytesStart;
use crate::reader::Decoder;
use serde::de::{self, DeserializeSeed};
#[cfg(not(feature = "encoding"))]
use std::borrow::Cow;

/// Check if tag `start` is included in the `fields` list. `decoder` is used to
/// get a string representation of a tag.
///
/// Returns `true`, if `start` is not in the `fields` list and `false` otherwise.
pub fn not_in(
    fields: &'static [&'static str],
    start: &BytesStart,
    decoder: Decoder,
) -> Result<bool, DeError> {
    #[cfg(not(feature = "encoding"))]
    let tag = Cow::Borrowed(decoder.decode(start.name())?);

    #[cfg(feature = "encoding")]
    let tag = decoder.decode(start.name());

    Ok(fields.iter().all(|&field| field != tag.as_ref()))
}

/// A filter that determines, what tags should form a sequence.
///
/// There are two types of sequences:
/// - sequence where each element represented by tags with the same name
/// - sequence where each element can have a different tag
///
/// The first variant could represent a collection of structs, the second --
/// a collection of enum variants.
///
/// In the second case we don't know what tag name should be expected as a
/// sequence element, so we accept any element. Since the sequence are flattened
/// into maps, we skip elements which have dedicated fields in a struct by using an
/// `Exclude` filter that filters out elements with names matching field names
/// from the struct.
///
/// # Lifetimes
///
/// `'de` represents a lifetime of the XML input, when filter stores the
/// dedicated tag name
#[derive(Debug)]
enum TagFilter<'de> {
    /// A `SeqAccess` interested only in tags with specified name to deserialize
    /// an XML like this:
    ///
    /// ```xml
    /// <...>
    ///   <tag/>
    ///   <tag/>
    ///   <tag/>
    ///   ...
    /// </...>
    /// ```
    ///
    /// The tag name is stored inside (`b"tag"` for that example)
    Include(BytesStart<'de>), //TODO: Need to store only name instead of a whole tag
    /// A `SeqAccess` interested in tags with any name, except explicitly listed.
    /// Excluded tags are used as struct field names and therefore should not
    /// fall into a `$value` category
    Exclude(&'static [&'static str]),
}

impl<'de> TagFilter<'de> {
    fn is_suitable(&self, start: &BytesStart, decoder: Decoder) -> Result<bool, DeError> {
        match self {
            Self::Include(n) => Ok(n.name() == start.name()),
            Self::Exclude(fields) => not_in(fields, start, decoder),
        }
    }
}

/// A SeqAccess
pub struct SeqAccess<'de, 'a, R>
where
    R: XmlRead<'de>,
{
    /// Deserializer used to deserialize sequence items
    de: &'a mut Deserializer<'de, R>,
    /// Filter that determines whether a tag is a part of this sequence.
    filter: TagFilter<'de>,
}

impl<'a, 'de, R> SeqAccess<'de, 'a, R>
where
    R: XmlRead<'de>,
{
    /// Get a new SeqAccess
    pub fn new(de: &'a mut Deserializer<'de, R>) -> Result<Self, DeError> {
        let filter = if de.has_value_field {
            TagFilter::Exclude(&[])
        } else {
            if let DeEvent::Start(e) = de.peek()? {
                // Clone is cheap if event borrows from the input
                TagFilter::Include(e.clone())
            } else {
                TagFilter::Exclude(&[])
            }
        };
        Ok(Self { de, filter })
    }
}

impl<'de, 'a, R> de::SeqAccess<'de> for SeqAccess<'de, 'a, R>
where
    R: XmlRead<'de>,
{
    type Error = DeError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, DeError>
    where
        T: DeserializeSeed<'de>,
    {
        let decoder = self.de.reader.decoder();
        match self.de.peek()? {
            DeEvent::Eof | DeEvent::End(_) => Ok(None),
            DeEvent::Start(e) if !self.filter.is_suitable(e, decoder)? => Ok(None),
            _ => seed.deserialize(&mut *self.de).map(Some),
        }
    }
}

#[test]
fn test_not_in() {
    let tag = BytesStart::borrowed_name(b"tag");

    assert_eq!(not_in(&[], &tag, Decoder::utf8()).unwrap(), true);
    assert_eq!(
        not_in(&["no", "such", "tags"], &tag, Decoder::utf8()).unwrap(),
        true
    );
    assert_eq!(
        not_in(&["some", "tag", "included"], &tag, Decoder::utf8()).unwrap(),
        false
    );
}

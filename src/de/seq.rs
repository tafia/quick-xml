use crate::de::{DeError, DeEvent, Deserializer, XmlRead};
use crate::encoding::Decoder;
use crate::events::BytesStart;
use serde::de::{DeserializeSeed, SeqAccess};

/// Check if tag `start` is included in the `fields` list. `decoder` is used to
/// get a string representation of a tag.
///
/// Returns `true`, if `start` is not in the `fields` list and `false` otherwise.
pub fn not_in(
    fields: &'static [&'static str],
    start: &BytesStart,
    decoder: Decoder,
) -> Result<bool, DeError> {
    let tag = decoder.decode(start.name().into_inner())?;

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
pub enum TagFilter<'de> {
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
    pub fn is_suitable(&self, start: &BytesStart, decoder: Decoder) -> Result<bool, DeError> {
        match self {
            Self::Include(n) => Ok(n.name() == start.name()),
            Self::Exclude(fields) => not_in(fields, start, decoder),
        }
    }
}

/// A SeqAccess
pub struct TopLevelSeqAccess<'de, 'a, R>
where
    R: XmlRead<'de>,
{
    /// Deserializer used to deserialize sequence items
    de: &'a mut Deserializer<'de, R>,
}

impl<'a, 'de, R> TopLevelSeqAccess<'de, 'a, R>
where
    R: XmlRead<'de>,
{
    /// Creates a new accessor to a top-level sequence of XML elements.
    pub fn new(de: &'a mut Deserializer<'de, R>) -> Result<Self, DeError> {
        Ok(Self { de })
    }
}

impl<'de, 'a, R> SeqAccess<'de> for TopLevelSeqAccess<'de, 'a, R>
where
    R: XmlRead<'de>,
{
    type Error = DeError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, DeError>
    where
        T: DeserializeSeed<'de>,
    {
        match self.de.peek()? {
            DeEvent::Eof => Ok(None),

            // Start(tag), End(tag), Text, CData
            _ => seed.deserialize(&mut *self.de).map(Some),
        }
    }
}

#[test]
fn test_not_in() {
    let tag = BytesStart::new("tag");

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

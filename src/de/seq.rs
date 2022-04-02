use crate::de::{BorrowingReader, DeError, DeEvent, Deserializer};
use crate::events::BytesStart;
use crate::reader::Decoder;
use serde::de::{DeserializeSeed, SeqAccess};
#[cfg(not(feature = "encoding"))]
use std::borrow::Cow;

/// Check if tag `start` is included in the `fields` list. `decoder` is used to
/// get a string representation of a tag.
///
/// Returns `true`, if `start` is not in the `fields` list and `false` otherwise.
pub fn is_unknown(
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
/// There is a two variant of sequences:
/// - sequence where each element represented by tags with the same name
/// - sequence where each element can have a different tag
///
/// The first variant could represent a collection of structs, the second --
/// a collection of enums.
///
/// In the second case we don't know what name sequence element will accept,
/// so we take an any element. But because in XML sequences a flattened into a
/// maps, then we could take an elements that have their own dedicated fields
/// in a struct. To prevent this we use an `Exclude` filter, that filters out
/// any known names of a struct fields.
#[derive(Debug)]
pub enum TagFilter {
    /// A `SeqAccess` interested only in tags with specified name to deserialize
    /// XML like this:
    ///
    /// ```xml
    /// <...>
    ///   <tag/>
    ///   <tag/>
    ///   <tag/>
    ///   ...
    /// </...>
    /// ```
    Include(Vec<u8>),
    /// A `SeqAccess` interested in tags with any name, except explicitly listed.
    /// Excluded tags are used as struct field names and therefore should not
    /// fall into `$value` category
    Exclude(&'static [&'static str]),
}

impl TagFilter {
    pub fn is_suitable(&self, start: &BytesStart, decoder: Decoder) -> Result<bool, DeError> {
        match self {
            Self::Include(n) => Ok(n == start.name()),
            Self::Exclude(fields) => is_unknown(fields, start, decoder),
        }
    }
}

/// A SeqAccess
pub struct TopLevelSeqAccess<'de, 'a, R>
where
    R: BorrowingReader<'de>,
{
    /// Deserializer used to deserialize sequence items
    de: &'a mut Deserializer<'de, R>,
    /// Tag name of elements that should be deserialized. All other tags will be
    /// skipped
    filter: TagFilter,
}

impl<'a, 'de, R> TopLevelSeqAccess<'de, 'a, R>
where
    R: BorrowingReader<'de>,
{
    /// Creates a new accessor to a top-level sequence of XML elements.
    pub fn new(de: &'a mut Deserializer<'de, R>) -> Result<Self, DeError> {
        let filter = if let DeEvent::Start(e) = de.peek()? {
            TagFilter::Include(e.name().to_vec())
        } else {
            TagFilter::Exclude(&[])
        };
        Ok(Self { de, filter })
    }
}

impl<'de, 'a, R> SeqAccess<'de> for TopLevelSeqAccess<'de, 'a, R>
where
    R: BorrowingReader<'de>,
{
    type Error = DeError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, DeError>
    where
        T: DeserializeSeed<'de>,
    {
        let decoder = self.de.reader.decoder();
        loop {
            break match self.de.peek()? {
                // If we see a tag that we not interested, skip it
                DeEvent::Start(e) if !self.filter.is_suitable(e, decoder)? => {
                    self.de.skip()?;
                    continue;
                }
                // This is unmatched End tag at top-level
                DeEvent::End(e) => Err(DeError::UnexpectedEnd(e.name().to_owned())),
                DeEvent::Eof => Ok(None),

                // Start(tag), Text, CData
                _ => seed.deserialize(&mut *self.de).map(Some),
            };
        }
    }
}

#[test]
fn test_is_unknown() {
    let tag = BytesStart::borrowed_name(b"tag");

    assert_eq!(is_unknown(&[], &tag, Decoder::utf8()).unwrap(), true);
    assert_eq!(
        is_unknown(&["no", "such", "tags"], &tag, Decoder::utf8()).unwrap(),
        true
    );
    assert_eq!(
        is_unknown(&["some", "tag", "included"], &tag, Decoder::utf8()).unwrap(),
        false
    );
}

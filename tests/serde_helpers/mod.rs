//! Utility functions for serde integration tests

use quick_xml::de::Deserializer;
use quick_xml::DeError;
use serde::Deserialize;

/// Deserialize an instance of type T from a string of XML text.
/// If deserialization was succeeded checks that all XML events was consumed
pub fn from_str<'de, T>(source: &'de str) -> Result<T, DeError>
where
    T: Deserialize<'de>,
{
    // Log XML that we try to deserialize to see it in the failed tests output
    dbg!(source);
    let mut de = Deserializer::from_str(source);
    let result = T::deserialize(&mut de);

    // If type was deserialized, the whole XML document should be consumed
    if let Ok(_) = result {
        assert!(de.is_empty(), "the whole XML document should be consumed");
    }

    result
}

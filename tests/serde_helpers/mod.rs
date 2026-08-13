//! Utility functions for serde integration tests

use pretty_assertions::assert_eq;
use quick_xml::de::Deserializer;
use quick_xml::DeError;
use serde::Deserialize;

/// Deserialize an instance of type T from a string of XML text.
/// If deserialization was succeeded checks that all XML events was consumed
#[track_caller]
pub fn from_str<'de, T>(source: &'de str) -> Result<T, DeError>
where
    T: Deserialize<'de>,
{
    // Log XML that we try to deserialize to see it in the failed tests output
    dbg!(source);
    let mut de = Deserializer::from_str(source);
    assert_eq!(
        de.resolver().level(),
        0,
        "no user namespace bindings expected just after creation: {:#?}",
        de.resolver()
    );

    let result = T::deserialize(&mut de);

    // If type was deserialized, the whole XML document should be consumed
    if result.is_ok() {
        de.check_eof_reached();

        let resolver = de.resolver();
        assert_eq!(
            resolver.level(),
            0,
            "all namespace bindings must be popped, level should be zero: {:#?}",
            resolver
        );
    }

    result
}

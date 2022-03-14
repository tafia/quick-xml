//! Helper types for tests

use serde::de::{self, Deserialize, Deserializer, Error};
use std::fmt;

/// Wrapper around `Vec<u8>` that deserialized using `deserialize_byte_buf`
/// instead of vector's generic `deserialize_seq`
#[derive(Debug, PartialEq)]
pub struct ByteBuf(pub Vec<u8>);

impl<'de> Deserialize<'de> for ByteBuf {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = ByteBuf;

            fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                fmt.write_str("byte data")
            }

            fn visit_bytes<E: Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                Ok(ByteBuf(v.to_vec()))
            }

            fn visit_byte_buf<E: Error>(self, v: Vec<u8>) -> Result<Self::Value, E> {
                Ok(ByteBuf(v))
            }
        }

        Ok(d.deserialize_byte_buf(Visitor)?)
    }
}

/// Wrapper around `&[u8]` that deserialized using `deserialize_bytes`
/// instead of vector's generic `deserialize_seq`
#[derive(Debug, PartialEq)]
pub struct Bytes<'de>(pub &'de [u8]);

impl<'de> Deserialize<'de> for Bytes<'de> {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Bytes<'de>;

            fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                fmt.write_str("borrowed bytes")
            }

            fn visit_borrowed_bytes<E: Error>(self, v: &'de [u8]) -> Result<Self::Value, E> {
                Ok(Bytes(v))
            }
        }

        Ok(d.deserialize_bytes(Visitor)?)
    }
}

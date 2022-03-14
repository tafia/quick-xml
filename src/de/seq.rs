use crate::de::{BorrowingReader, DeError, DeEvent, Deserializer};
use crate::{events::BytesStart, reader::Decoder};
use serde::de::{self, DeserializeSeed};

#[derive(Debug)]
enum Names {
    Unknown,
    Peek(String),
}

impl Names {
    fn is_valid(&self, decoder: Decoder, start: &BytesStart) -> Result<bool, DeError> {
        #[cfg(not(feature = "encoding"))]
        let name = decoder.decode(start.name())?;
        #[cfg(feature = "encoding")]
        let name = decoder.decode(start.name());
        let res = match self {
            Names::Unknown => true,
            Names::Peek(n) => &**n == &*name,
        };
        Ok(res)
    }
}

/// A SeqAccess
pub struct SeqAccess<'de, 'a, R>
where
    R: BorrowingReader<'de>,
{
    de: &'a mut Deserializer<'de, R>,
    names: Names,
}

impl<'a, 'de, R> SeqAccess<'de, 'a, R>
where
    R: BorrowingReader<'de>,
{
    /// Get a new SeqAccess
    pub fn new(de: &'a mut Deserializer<'de, R>) -> Result<Self, DeError> {
        let decoder = de.reader.decoder();
        let names = if de.has_value_field {
            Names::Unknown
        } else {
            if let DeEvent::Start(e) = de.peek()? {
                #[cfg(not(feature = "encoding"))]
                let name = decoder.decode(e.name())?.to_owned();
                #[cfg(feature = "encoding")]
                let name = decoder.decode(e.name()).into_owned();
                Names::Peek(name)
            } else {
                Names::Unknown
            }
        };
        Ok(SeqAccess { de, names })
    }
}

impl<'de, 'a, R> de::SeqAccess<'de> for SeqAccess<'de, 'a, R>
where
    R: BorrowingReader<'de>,
{
    type Error = DeError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, DeError>
    where
        T: DeserializeSeed<'de>,
    {
        let decoder = self.de.reader.decoder();
        match self.de.peek()? {
            DeEvent::Eof | DeEvent::End(_) => Ok(None),
            DeEvent::Start(e) if !self.names.is_valid(decoder, e)? => Ok(None),
            _ => seed.deserialize(&mut *self.de).map(Some),
        }
    }
}

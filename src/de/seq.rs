use crate::de::{BorrowingReader, DeError, DeEvent, Deserializer};
use crate::{events::BytesStart, reader::Decoder};
use serde::de;

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
pub struct SeqAccess<'de, 'a, R: BorrowingReader<'de>> {
    de: &'a mut Deserializer<'de, R>,
    max_size: Option<usize>,
    names: Names,
}

impl<'a, 'de, R: BorrowingReader<'de>> SeqAccess<'de, 'a, R> {
    /// Get a new SeqAccess
    pub fn new(de: &'a mut Deserializer<'de, R>, max_size: Option<usize>) -> Result<Self, DeError> {
        let decoder = de.reader.decoder();
        let names = if de.has_value_field {
            Names::Unknown
        } else {
            if let Some(DeEvent::Start(e)) = de.peek()? {
                #[cfg(not(feature = "encoding"))]
                let name = decoder.decode(e.name())?.to_owned();
                #[cfg(feature = "encoding")]
                let name = decoder.decode(e.name()).into_owned();
                Names::Peek(name)
            } else {
                Names::Unknown
            }
        };
        Ok(SeqAccess {
            de,
            max_size,
            names,
        })
    }
}

impl<'de, 'a, R: BorrowingReader<'de>> de::SeqAccess<'de> for SeqAccess<'de, 'a, R> {
    type Error = DeError;

    fn size_hint(&self) -> Option<usize> {
        self.max_size
    }

    fn next_element_seed<T: de::DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, DeError> {
        if let Some(s) = self.max_size.as_mut() {
            if *s == 0 {
                return Ok(None);
            }
            *s -= 1;
        }
        let decoder = self.de.reader.decoder();
        match self.de.peek()? {
            None | Some(DeEvent::Eof) | Some(DeEvent::End(_)) => Ok(None),
            Some(DeEvent::Start(e)) if !self.names.is_valid(decoder, e)? => Ok(None),
            _ => seed.deserialize(&mut *self.de).map(Some),
        }
    }
}

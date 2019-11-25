use crate::de::{DeError, Deserializer};
use crate::{
    events::{BytesStart, Event},
    reader::Decoder,
};
use serde::de;
use std::io::BufRead;

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
pub struct SeqAccess<'a, R: BufRead> {
    de: &'a mut Deserializer<R>,
    max_size: Option<usize>,
    names: Names,
}

impl<'a, R: BufRead> SeqAccess<'a, R> {
    /// Get a new SeqAccess
    pub fn new(de: &'a mut Deserializer<R>, max_size: Option<usize>) -> Result<Self, DeError> {
        let decoder = de.reader.decoder();
        let names = if de.has_value_field {
            Names::Unknown
        } else {
            if let Some(Event::Start(e)) = de.peek()? {
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

impl<'de, 'a, R: 'a + BufRead> de::SeqAccess<'de> for SeqAccess<'a, R> {
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
            None | Some(Event::Eof) | Some(Event::End(_)) => Ok(None),
            Some(Event::Start(e)) if !self.names.is_valid(decoder, e)? => Ok(None),
            _ => seed.deserialize(&mut *self.de).map(Some),
        }
    }
}

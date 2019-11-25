use crate::de::{DeError, Deserializer};
use crate::events::Event;
use serde::de;
use std::io::BufRead;

/// A SeqAccess
pub struct SeqAccess<'a, R: BufRead> {
    de: &'a mut Deserializer<R>,
    max_size: Option<usize>,
}

impl<'a, R: BufRead> SeqAccess<'a, R> {
    /// Get a new SeqAccess
    pub fn new(de: &'a mut Deserializer<R>, max_size: Option<usize>) -> Self {
        SeqAccess { de, max_size }
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
        match self.de.peek()? {
            None | Some(Event::Eof) | Some(Event::End(_)) => Ok(None),
            _ => seed.deserialize(&mut *self.de).map(Some),
        }
    }
}

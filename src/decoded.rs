use std::io::BufRead;
use encoding::types::EncodingRef;
use {XmlReader, Event, AsStr};
use error::ResultPos;

/// An iterator which decodes `Event::Text`
///
/// Behaves exactly the same as `XmlReader` except for
/// `Event::Text` replaced by `Event::DecodedText`, which provides a
/// decoded `String`
pub struct XmlDecoder<'a, B: BufRead + 'a> {
    reader: &'a mut XmlReader<B>,
    decoder: Option<EncodingRef>,
}

impl<'a, B: BufRead + 'a> XmlDecoder<'a, B> {
    /// Creates a new `XmlDecoder`
    pub fn new(r: &'a mut XmlReader<B>) -> XmlDecoder<'a, B> {
        XmlDecoder {
            reader: r,
            decoder: None,
        }
    }

    /// Sets a custom decoder
    pub fn with_decoder(&mut self, decoder: Option<EncodingRef>) -> &mut XmlDecoder<'a, B> {
        self.decoder = decoder;
        self
    }
}

impl<'a, B: BufRead + 'a> Iterator for XmlDecoder<'a, B> {
    type Item = ResultPos<Event>;
    fn next(&mut self) -> Option<ResultPos<Event>> {
        match self.reader.next() {
            None => None,
            Some(Err(e)) => Some(Err(e)),
            Some(Ok(e)) => Some(map_event(self, e)),
        }
    }
}

fn map_event<'a, B: BufRead + 'a>(r: &mut XmlDecoder<B>, e: Event) -> ResultPos<Event> {
    match e {
        Event::Decl(e) => {
            if r.decoder.is_none() {
                r.decoder = e.encoder()?;
            }
            Ok(Event::Decl(e))
        },
        Event::Text(e) => {
            let s = e
                .content()
                .as_string(r.decoder.as_ref())
                .map_err(|err| (err, r.reader.buf_position))?;
            Ok(Event::DecodedText(e, s))
        },
        e => Ok(e),
    }
}

//! This example demonstrate how custom entities can be extracted from the DOCTYPE,
//! and later use to:
//! - insert new pieces of document (particular case - insert only textual content)
//! - decode attribute values
//!
//! NB: this example is deliberately kept simple:
//! * it assumes that the XML file is UTF-8 encoded (custom_entities must only contain UTF-8 data)
//! * it only handles internal entities;
//! * the regex in this example is simple but brittle;
//! * it does not support the use of entities in entity declaration.

use std::borrow::Cow;
use std::collections::{HashMap, VecDeque};
use std::str::from_utf8;

use quick_xml::encoding::Decoder;
use quick_xml::errors::Error;
use quick_xml::escape::EscapeError;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::name::QName;
use quick_xml::reader::Reader;
use regex::bytes::Regex;

use pretty_assertions::assert_eq;

struct MyReader<'i> {
    /// Stack of readers, the first element is the initial reader, the other are
    /// readers created for each resolved entity
    readers: VecDeque<Reader<&'i [u8]>>,
    /// Map of captured internal _parsed general entities_. _Parsed_ means that
    /// value of the entity is parsed by XML reader
    entities: HashMap<&'i [u8], &'i [u8]>,
    /// In this example we use simple regular expression to capture entities from DTD.
    /// In real application you should use DTD parser.
    entity_re: Regex,
}
impl<'i> MyReader<'i> {
    fn new(input: &'i str) -> Result<Self, regex::Error> {
        let mut reader = Reader::from_str(input);
        reader.config_mut().trim_text(true);

        let mut readers = VecDeque::new();
        readers.push_back(reader);

        // Capture "name" and "content" from such string:
        // <!ENTITY name "content" >
        let entity_re = Regex::new(r#"<!ENTITY\s+([^ \t\r\n]+)\s+"([^"]*)"\s*>"#)?;
        Ok(Self {
            readers,
            entities: HashMap::new(),
            entity_re,
        })
    }
    fn read_event(&mut self) -> Result<Event<'i>, Error> {
        loop {
            if let Some(mut reader) = self.readers.pop_back() {
                match dbg!(reader.read_event())? {
                    // Capture defined entities from the DTD inside document and skip that event
                    Event::DocType(e) => {
                        self.readers.push_back(reader);
                        self.capture(e);
                        continue;
                    }
                    // When entity is referenced, create new reader with the same settings as
                    // the current reader have and push it to the top of stack. Then try to
                    // read next event from it (on next iteration)
                    Event::GeneralRef(e) => {
                        if let Some(ch) = e.resolve_char_ref()? {
                            self.readers.push_back(reader);
                            return Ok(Event::Text(BytesText::from_escaped(ch.to_string())));
                        }
                        let mut r = Reader::from_reader(self.resolve(&e)?);
                        *r.config_mut() = reader.config().clone();

                        self.readers.push_back(reader);
                        self.readers.push_back(r);
                        continue;
                    }
                    // When reader is exhausted, do not return it to the stack
                    Event::Eof => continue,

                    // Return all other events to caller
                    e => {
                        self.readers.push_back(reader);
                        return Ok(e);
                    }
                }
            }
            return Ok(Event::Eof);
        }
    }

    /// In this example we use simple regular expression to capture entities from DTD.
    /// In real application you should use DTD parser
    fn capture(&mut self, doctype: BytesText<'i>) {
        let doctype = match doctype.into_inner() {
            Cow::Borrowed(doctype) => doctype,
            Cow::Owned(_) => unreachable!("We are sure that event will be borrowed"),
        };
        for cap in self.entity_re.captures_iter(doctype) {
            self.entities.insert(
                cap.get(1).unwrap().as_bytes(),
                cap.get(2).unwrap().as_bytes(),
            );
        }
    }

    fn resolve(&self, entity: &[u8]) -> Result<&'i [u8], EscapeError> {
        match self.entities.get(entity) {
            Some(replacement) => Ok(replacement),
            None => Err(EscapeError::UnrecognizedEntity(
                0..0,
                String::from_utf8_lossy(entity).into_owned(),
            )),
        }
    }

    fn get_entity(&self, entity: &str) -> Option<&'i str> {
        self.entities
            .get(entity.as_bytes())
            // SAFETY: We are sure that slices are correct UTF-8 because we get
            // them from rust string
            .map(|value| from_utf8(value).unwrap())
    }

    fn decoder(&self) -> Decoder {
        self.readers.back().unwrap().decoder()
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = MyReader::new(
        r#"
        <!DOCTYPE test [
        <!ENTITY text "hello world" >
        <!ENTITY element1 "<dtd attr = 'Message: &text;'/>" >
        <!ENTITY element2 "<a>&element1;</a>" >
        ]>
        <test label="Message: &text;">&#39;&element2;&#x27;</test>
        "#,
    )?;

    let event = reader.read_event()?;
    assert_eq!(
        event,
        Event::Start(BytesStart::from_content(
            r#"test label="Message: &text;""#,
            4
        ))
    );
    if let Event::Start(e) = event {
        let mut attrs = e.attributes();

        let label = attrs.next().unwrap()?;
        assert_eq!(label.key, QName(b"label"));
        assert_eq!(
            label.decode_and_unescape_value_with(reader.decoder(), |ent| reader.get_entity(ent))?,
            "Message: hello world"
        );

        assert_eq!(attrs.next(), None);
    }

    // This is decoded decimal character reference &#39;
    assert_eq!(
        reader.read_event()?,
        Event::Text(BytesText::from_escaped("'"))
    );

    //--------------------------------------------------------------------------
    // This part was inserted into original document from entity defined in DTD

    assert_eq!(reader.read_event()?, Event::Start(BytesStart::new("a")));
    let event = reader.read_event()?;
    assert_eq!(
        event,
        Event::Empty(BytesStart::from_content(
            r#"dtd attr = 'Message: &text;'"#,
            3
        ))
    );
    if let Event::Start(e) = event {
        let mut attrs = e.attributes();

        let attr = attrs.next().unwrap()?;
        assert_eq!(attr.key, QName(b"attr"));
        assert_eq!(
            attr.decode_and_unescape_value_with(reader.decoder(), |ent| reader.get_entity(ent))?,
            "Message: hello world"
        );

        assert_eq!(attrs.next(), None);
    }
    assert_eq!(reader.read_event()?, Event::End(BytesEnd::new("a")));
    //--------------------------------------------------------------------------

    // This is decoded hexadecimal character reference &#x27;
    assert_eq!(
        reader.read_event()?,
        Event::Text(BytesText::from_escaped("'"))
    );

    assert_eq!(reader.read_event()?, Event::End(BytesEnd::new("test")));
    assert_eq!(reader.read_event()?, Event::Eof);

    Ok(())
}

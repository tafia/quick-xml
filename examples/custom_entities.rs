//! This example demonstrate how custom entities can be extracted from the DOCTYPE!,
//! and later use to decode text and attribute values.
//!
//! NB: this example is deliberately kept simple:
//! * it assumes that the XML file is UTF-8 encoded (custom_entities must only contain UTF-8 data)
//! * it only handles internal entities;
//! * the regex in this example is simple but brittle;
//! * it does not support the use of entities in entity declaration.

extern crate quick_xml;
extern crate regex;

use quick_xml::events::Event;
use quick_xml::Reader;
use regex::bytes::Regex;
use std::collections::HashMap;

const DATA: &str = r#"

    <?xml version="1.0"?>
    <!DOCTYPE test [
    <!ENTITY msg "hello world" >
    ]>
    <test label="&msg;">&msg;</test>

"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = Reader::from_str(DATA);
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut custom_entities = HashMap::new();
    let entity_re = Regex::new(r#"<!ENTITY\s+([^ \t\r\n]+)\s+"([^"]*)"\s*>"#)?;

    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::DocType(ref e)) => {
                for cap in entity_re.captures_iter(&e) {
                    custom_entities.insert(cap[1].to_vec(), cap[2].to_vec());
                }
            }
            Ok(Event::Start(ref e)) => match e.name() {
                b"test" => println!(
                    "attributes values: {:?}",
                    e.attributes()
                        .map(|a| a
                            .unwrap()
                            .unescape_and_decode_value_with_custom_entities(
                                &reader,
                                &custom_entities
                            )
                            .unwrap())
                        .collect::<Vec<_>>()
                ),
                _ => (),
            },
            Ok(Event::Text(ref e)) => {
                println!(
                    "text value: {}",
                    e.unescape_and_decode_with_custom_entities(&reader, &custom_entities)
                        .unwrap()
                );
            }
            Ok(Event::Eof) => break,
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            _ => (),
        }
    }
    Ok(())
}

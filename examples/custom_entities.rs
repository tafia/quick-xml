//! This example demonstrate how custom entities can be extracted from the DOCTYPE!,
//! and later use to decode text and attribute values.
//!
//! NB: this example is deliberately kept simple:
//! * it assumes that the XML file is UTF-8 encoded (custom_entities must only contain UTF-8 data)
//! * it only handles internal entities;
//! * the regex in this example is simple but brittle;
//! * it does not support the use of entities in entity declaration.

use std::collections::HashMap;

use quick_xml::events::Event;
use quick_xml::reader::Reader;
use regex::bytes::Regex;

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

    let mut custom_entities: HashMap<String, String> = HashMap::new();
    let entity_re = Regex::new(r#"<!ENTITY\s+([^ \t\r\n]+)\s+"([^"]*)"\s*>"#)?;

    loop {
        match reader.read_event() {
            Ok(Event::DocType(ref e)) => {
                for cap in entity_re.captures_iter(e) {
                    custom_entities.insert(
                        reader.decoder().decode(&cap[1])?.into_owned(),
                        reader.decoder().decode(&cap[2])?.into_owned(),
                    );
                }
            }
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"test" => {
                    let attributes = e
                        .attributes()
                        .map(|a| {
                            a.unwrap()
                                .decode_and_unescape_value_with(&reader, |ent| {
                                    custom_entities.get(ent).map(|s| s.as_str())
                                })
                                .unwrap()
                                .into_owned()
                        })
                        .collect::<Vec<_>>();
                    println!("attributes values: {:?}", attributes);
                }
                _ => (),
            },
            Ok(Event::Text(ref e)) => {
                println!(
                    "text value: {}",
                    e.unescape_with(|ent| custom_entities.get(ent).map(|s| s.as_str()))
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

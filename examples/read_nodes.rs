// example that separates logic for reading different top-level nodes of xml tree
// Note: for this specific data set using serde feature would simplify
//       this simple data is purely to make it easier to understand the code

use quick_xml::events::{BytesStart, Event};
use quick_xml::name::QName;
use quick_xml::reader::Reader;
use std::borrow::Cow;
use std::collections::HashMap;
use std::str;

const XML: &str = r#"
<?xml version="1.0" encoding="utf-8"?>
  <DefaultSettings Language="es" Greeting="HELLO"/>
  <Localization>
    <Translation Tag="HELLO" Language="ja">
      <Text>こんにちは</Text>
    </Translation>
    <Translation Tag="BYE" Language="ja">
      <Text>さようなら</Text>
    </Translation>
    <Translation Tag="HELLO" Language="es">
      <Text>Hola</Text>
    </Translation>
    <Translation Tag="BYE" Language="es">
      <Text>Adiós</Text>
    </Translation>
  </Localization>
"#;

#[derive(Debug)]
struct Translation {
    tag: String,
    lang: String,
    text: String,
}

impl Translation {
    fn new_from_element(
        reader: &mut Reader<&[u8]>,
        element: BytesStart,
    ) -> Result<Translation, quick_xml::Error> {
        let mut tag = Cow::Borrowed("");
        let mut lang = Cow::Borrowed("");

        for attr_result in element.attributes() {
            let a = attr_result?;
            match a.key.as_ref() {
                "Language" => lang = Cow::Owned(a.unescape_value()?.to_string()),
                "Tag" => tag = Cow::Owned(a.unescape_value()?.to_string()),
                _ => (),
            }
        }
        let mut element_buf = Vec::new();
        let event = reader.read_event_into(&mut element_buf)?;

        if let Event::Start(ref e) = event {
            let name = e.name();
            if name == QName("Text") {
                // note: `read_text` does not support content as CDATA
                let text_content = reader.read_text(e.name())?;
                Ok(Translation {
                    tag: tag.into(),
                    lang: lang.into(),
                    text: text_content.into(),
                })
            } else {
                dbg!("Expected Event::Start for Text, got: {:?}", &event);
                Err(quick_xml::Error::UnexpectedToken(name.as_ref().to_owned()))
            }
        } else {
            let event_string = format!("{:?}", event);
            Err(quick_xml::Error::UnexpectedToken(event_string))
        }
    }
}

fn main() -> Result<(), quick_xml::Error> {
    // In a real-world use case, Settings would likely be a struct
    // HashMap here is just to make the sample code short
    let mut settings: HashMap<String, String>;
    let mut translations: Vec<Translation> = Vec::new();

    let mut reader = Reader::from_str(XML);
    reader.trim_text(true);

    // == Handling empty elements ==
    // To simply our processing code
    // we want the same events for empty elements, like:
    //   <DefaultSettings Language="es" Greeting="HELLO"/>
    //   <Text/>
    reader.expand_empty_elements(true);
    let mut buf = Vec::new();

    loop {
        let event = reader.read_event_into(&mut buf)?;

        match event {
            Event::Start(element) => match element.name().as_ref() {
                "DefaultSettings" => {
                    // Note: real app would handle errors with good defaults or halt program with nice message
                    // This illustrates decoding an attribute's key and value with error handling
                    settings = element
                        .attributes()
                        .map(|attr_result| {
                            match attr_result {
                                Ok(a) => {
                                    let key = a.key.local_name().as_ref().to_string();
                                    let value = a.unescape_value().expect("failure to unescape").to_string();
                                    (key, value)
                                },
                                Err(err) => {
                                     dbg!("unable to read key in DefaultSettings, err = {:?}", err);
                                    (String::new(), String::new())
                                }
                            }
                        })
                        .collect();
                    assert_eq!(settings["Language"], "es");
                    assert_eq!(settings["Greeting"], "HELLO");
                    reader.read_to_end(element.name())?;
                }
                "Translation" => {
                    translations.push(Translation::new_from_element(&mut reader, element)?);
                }
                _ => (),
            },

            Event::Eof => break, // exits the loop when reaching end of file
            _ => (),             // There are `Event` types not considered here
        }
    }
    dbg!("{:?}", &translations);
    assert_eq!(translations.len(), 4);
    assert_eq!(translations[2].tag, "HELLO");
    assert_eq!(translations[2].text, "Hola");
    assert_eq!(translations[2].lang, "es");

    Ok(())
}

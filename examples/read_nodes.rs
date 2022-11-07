// example that separates logic for reading different top-level nodes of xml tree

use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use std::collections::HashMap;
use std::str;

// Code doesn't work if DefaultSettings node is constructed like this:
//   <DefaultSettings Language="es" Greeting="HELLO"/>

const XML: &str = r#"
<?xml version="1.0" encoding="utf-8"?>
  <DefaultSettings Language="es" Greeting="HELLO">
  </DefaultSettings>  
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
        use std::borrow::Cow;
        let mut tag = Cow::Borrowed("");
        let mut lang = Cow::Borrowed("");
        // let (tag, lang) =
        for attr_result in element.attributes() {
            let a = attr_result?;
            match a.key.as_ref() {
                b"Language" => lang = a.unescape_value()?,
                b"Tag" => tag = a.unescape_value()?,
                _ => (),
            }
        }
        let mut text = Cow::Borrowed("");
        let mut element_buf = Vec::new();
        let event = reader.read_event_into(&mut element_buf)?;
        match event {
            Event::Text(ref e) => {
                text = e.unescape()?;
                println!("text node content: {}", text);
            }
            _ => (),
        }

        Ok(Translation {
            tag: tag.into(),
            lang: lang.into(),
            text: text.into(),
        })
    }
}

fn main() -> Result<(), quick_xml::Error> {
    // In a real-world use case, Settings would likely be a struct
    // HashMap here is just to make the sample code short
    let mut settings: HashMap<String, String>;
    let mut translations: Vec<Translation> = Vec::new();

    let mut reader = Reader::from_str(XML);
    reader.trim_text(true);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(element) => match element.name().as_ref() {
                b"DefaultSettings" => {
                    settings = element
                        .attributes()
                        .map(|attr_result| {
                            let a = attr_result.unwrap();
                            (
                                str::from_utf8(a.key.as_ref()).unwrap().to_string(),
                                a.unescape_value().unwrap().to_string(),
                            )
                        })
                        .collect();
                    println!("settings: {:?}", settings);
                }
                b"Translation" => {
                    translations.push(Translation::new_from_element(&mut reader, element)?);
                }
                _ => (),
            },

            Event::Eof => break, // exits the loop when reaching end of file
            _ => (),             // There are `Event` types not considered here
        }
    }
    println!("translations...");
    for item in translations {
        println!("{} {} {}", item.lang, item.tag, item.text);
    }
    Ok(())
}

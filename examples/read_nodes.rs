// example that separates logic for reading different top-level nodes of xml tree

use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::Infallible;
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
                dbg!("text node content: {}", &text);
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
                    // note: real app would handle errors with good defaults or halt program with nice message
                    // This example illustrates transforming an attribute's key and value into a String
                    settings = element
                        .attributes()
                        .map(|attr_result| {
                            match attr_result {
                                Ok(a) => {
                                    let key = String::from_utf8(a.key.local_name().as_ref().into())
                                        .or_else(|err| {
                                            dbg!("unable to read key in DefaultSettings attribute {:?}, utf8 error {:?}", &a, err);
                                            Ok::<String, Infallible>(String::new())
                                        })
                                        .unwrap();
                                    let value = a.decode_and_unescape_value(&reader).or_else(|err| {
                                            dbg!("unable to read key in DefaultSettings attribute {:?}, utf8 error {:?}", &a, err);
                                            Ok::<Cow<'_, str>, Infallible>(std::borrow::Cow::from(""))
                                    }).unwrap().to_string();
                                    (key, value)
                                },
                                Err(err) => {
                                     dbg!("unable to read key in DefaultSettings, err = {:?}", err);
                                    (String::new(), String::new())
                                }
                            }
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

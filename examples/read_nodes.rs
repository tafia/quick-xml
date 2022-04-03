// example that separates logic for reading different top-level nodes of xml tree
// Note: for this specific data set using serde feature would simplify
//       this simple data is purely to make it easier to understand the code

use quick_xml::events::attributes::AttrError;
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::QName;
use quick_xml::reader::Reader;
use quick_xml::XmlVersion;
use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::Infallible;
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

// Enum variants is not read in example, so suppress the warning
#[allow(dead_code)]
#[derive(Debug)]
enum AppError {
    /// XML parsing error
    Xml(quick_xml::Error),
    /// The `Translation/Text` node is missed
    NoText(String),
}

impl From<quick_xml::Error> for AppError {
    fn from(error: quick_xml::Error) -> Self {
        Self::Xml(error)
    }
}

impl From<AttrError> for AppError {
    fn from(error: AttrError) -> Self {
        Self::Xml(quick_xml::Error::InvalidAttr(error))
    }
}

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
    ) -> Result<Translation, AppError> {
        let mut tag = Cow::Borrowed("");
        let mut lang = Cow::Borrowed("");

        for attr_result in element.attributes() {
            let a = attr_result?;
            match a.key.as_ref() {
                b"Language" => {
                    lang = a.decoded_and_normalized_value(XmlVersion::V1_0, reader.decoder())?
                }
                b"Tag" => {
                    tag = a.decoded_and_normalized_value(XmlVersion::V1_0, reader.decoder())?
                }
                _ => (),
            }
        }
        let mut element_buf = Vec::new();
        let event = reader.read_event_into(&mut element_buf)?;

        if let Event::Start(ref e) = event {
            let name = e.name();
            if name == QName(b"Text") {
                // note: `read_text` does not support content as CDATA
                let text_content = reader.read_text(e.name())?;
                Ok(Translation {
                    tag: tag.into(),
                    lang: lang.into(),
                    text: text_content.into(),
                })
            } else {
                dbg!("Expected Event::Start for Text, got: {:?}", &event);
                let name_string = reader
                    .decoder()
                    .decode(name.as_ref())
                    .map_err(quick_xml::Error::Encoding)?;
                Err(AppError::NoText(name_string.into()))
            }
        } else {
            let event_string = format!("{:?}", event);
            Err(AppError::NoText(event_string))
        }
    }
}

fn main() -> Result<(), AppError> {
    // In a real-world use case, Settings would likely be a struct
    // HashMap here is just to make the sample code short
    let mut settings: HashMap<String, String>;
    let mut translations: Vec<Translation> = Vec::new();

    let mut reader = Reader::from_str(XML);
    let config = reader.config_mut();

    config.trim_text(true);
    // == Handling empty elements ==
    // To simply our processing code
    // we want the same events for empty elements, like:
    //   <DefaultSettings Language="es" Greeting="HELLO"/>
    //   <Text/>
    config.expand_empty_elements = true;

    let mut buf = Vec::new();

    loop {
        let event = reader.read_event_into(&mut buf)?;

        match event {
            Event::Start(element) => match element.name().as_ref() {
                b"DefaultSettings" => {
                    // Note: real app would handle errors with good defaults or halt program with nice message
                    // This illustrates decoding an attribute's key and value with error handling
                    settings = element
                        .attributes()
                        .map(|attr_result| {
                            match attr_result {
                                Ok(a) => {
                                    let key = reader.decoder().decode(a.key.local_name().as_ref())
                                        .or_else(|err| {
                                            dbg!("unable to read key in DefaultSettings attribute {:?}, utf8 error {:?}", &a, err);
                                            Ok::<Cow<'_, str>, Infallible>(std::borrow::Cow::from(""))
                                        })
                                        .unwrap().to_string();
                                    let value = a.decoded_and_normalized_value(XmlVersion::V1_0, reader.decoder()).or_else(|err| {
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
                    assert_eq!(settings["Language"], "es");
                    assert_eq!(settings["Greeting"], "HELLO");
                    reader.read_to_end(element.name())?;
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
    dbg!("{:?}", &translations);
    assert_eq!(translations.len(), 4);
    assert_eq!(translations[2].tag, "HELLO");
    assert_eq!(translations[2].text, "Hola");
    assert_eq!(translations[2].lang, "es");

    Ok(())
}

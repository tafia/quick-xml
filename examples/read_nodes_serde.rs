// note: to use serde, the feature needs to be enabled
// run example with:
//    cargo run --example read_nodes_serde --features="serialize"

use quick_xml::de::DeError;
use quick_xml::de::Deserializer;
use serde::Deserialize;

#[derive(Debug, PartialEq, Default, Deserialize)]
#[serde(default)]
struct Translation {
    #[serde(rename = "@Tag")]
    tag: String,
    #[serde(rename = "@Language")]
    lang: String,
    #[serde(rename = "Text")]
    text: String,
}

#[derive(Debug, PartialEq, Default, Deserialize)]
#[serde(default)]
struct DefaultSettings {
    #[serde(rename = "@Language")]
    language: String,
    #[serde(rename = "@Greeting")]
    greeting: String,
}

#[derive(Debug, PartialEq, Default, Deserialize)]
#[serde(default)]
struct Config {
    #[serde(rename = "DefaultSettings")]
    settings: DefaultSettings,
    #[serde(rename = "Localization")]
    translation: Vec<Translation>,
}

const XML: &str = r#"
<?xml version="1.0" encoding="utf-8"?>
<Config>
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
</Config>
"#;

const ONE_TRANSLATION_XML: &str = r#"
    <Translation Tag="HELLO" Language="ja">
      <Text>こんにちは</Text>
    </Translation>
"#;

/// Deserialize an instance of type T from a string of XML text.
/// If deserialization was succeeded checks that all XML events were consumed
fn from_str<'de, T>(s: &'de str) -> Result<T, DeError>
where
    T: Deserialize<'de>,
{
    let mut de = Deserializer::from_str(s);
    let result = T::deserialize(&mut de);

    // If type was deserialized, the whole XML document should be consumed
    if let Ok(_) = result {
        match <()>::deserialize(&mut de) {
            Err(DeError::UnexpectedEof) => (),
            e => panic!("Expected end `UnexpectedEof`, but got {:?}", e),
        }
    }

    result
}

fn main() -> anyhow::Result<()> {
    let t: Translation = from_str(ONE_TRANSLATION_XML)?;
    assert_eq!(t.tag, "HELLO");
    assert_eq!(t.lang, "ja");
    assert_eq!(t.text, "こんにちは");

    let config: Config = from_str(XML)?;
    dbg!("{:?}", &config);

    assert_eq!(config.settings.language, "es");
    assert_eq!(config.settings.greeting, "HELLO");

    assert_eq!(config.translation.len(), 4);
    assert_eq!(config.translation[2].tag, "HELLO");
    assert_eq!(config.translation[2].text, "Hola");
    assert_eq!(config.translation[2].lang, "es");
    Ok(())
}

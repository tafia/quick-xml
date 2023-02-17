// note: to use serde, the feature needs to be enabled
// run example with:
//    cargo run --example read_nodes_serde --features="serialize"

use quick_xml::de::from_str;
use serde::Deserialize;

#[derive(Debug, PartialEq, Default, Deserialize)]
#[serde(default)]
struct Translation {
    #[serde(rename = "@Tag")]
    tag: String,
    #[serde(rename = "@Language")]
    lang: String,
    #[serde(rename = "$text")]
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
#[serde(default, rename_all = "PascalCase")]
struct Config {
    #[serde(rename = "DefaultSettings")]
    settings: DefaultSettings,
    localization: Localization,
}
#[derive(Debug, PartialEq, Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Localization {
    translation: Vec<Translation>,
}

const XML: &str = r#"
<?xml version="1.0" encoding="utf-8"?>
<Config>
  <DefaultSettings Language="es" Greeting="HELLO"/>
  <Localization>
    <Translation Tag="HELLO" Language="ja">
      こんにちは
    </Translation>
    <Translation Tag="BYE" Language="ja">
      さようなら
    </Translation>
    <Translation Tag="HELLO" Language="es">
      Hola
    </Translation>
    <Translation Tag="BYE" Language="es">
      Adiós
    </Translation>
  </Localization>
</Config>
"#;

const ONE_TRANSLATION_XML: &str = r#"
    <Translation Tag="HELLO" Language="ja">
      こんにちは
    </Translation>
"#;

fn main() -> Result<(), quick_xml::DeError> {
    let t: Translation = from_str(ONE_TRANSLATION_XML)?;
    assert_eq!(t.tag, "HELLO");
    assert_eq!(t.lang, "ja");
    assert_eq!(t.text, "こんにちは");

    let config: Config = from_str(XML)?;
    dbg!("{:?}", &config);

    assert_eq!(config.settings.language, "es");
    assert_eq!(config.settings.greeting, "HELLO");

    let translations = config.localization.translation;
    assert_eq!(translations.len(), 4);
    assert_eq!(translations[2].tag, "HELLO");
    assert_eq!(translations[2].text, "Hola");
    assert_eq!(translations[2].lang, "es");
    Ok(())
}

#![cfg(feature = "serialize")]

use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::writer::PrettyWriter;
use serde::Serialize;

#[derive(Serialize)]
struct Item<'a> {
    #[serde(rename = "@name")]
    name: &'a str,
    value: &'a str,
}

#[derive(Serialize)]
enum Content<'a> {
    #[serde(rename = "$text")]
    Text(&'a str),
}

#[test]
fn serializable_root_is_pretty_printed() {
    let mut writer = PrettyWriter::new(Vec::new());
    writer
        .write_serializable(
            "item",
            &Item {
                name: "a & b",
                value: "<hello>",
            },
        )
        .unwrap();
    assert_eq!(
        String::from_utf8(writer.into_inner().unwrap()).unwrap(),
        "<item name=\"a &amp; b\">\n  <value>&lt;hello&gt;</value>\n</item>"
    );
}

#[test]
fn serializable_joins_mixed_content() {
    let mut writer = PrettyWriter::new(Vec::new());
    writer
        .write_event(Event::Start(BytesStart::new("p")))
        .unwrap();
    writer
        .write_event(Event::Text(BytesText::new("before ")))
        .unwrap();
    writer.write_serializable("n", &42).unwrap();
    writer
        .write_event(Event::Text(BytesText::new(" after")))
        .unwrap();
    writer.write_event(Event::End(BytesEnd::new("p"))).unwrap();
    assert_eq!(
        String::from_utf8(writer.into_inner().unwrap()).unwrap(),
        "<p>before <n>42</n> after</p>"
    );
}

#[test]
fn serialized_text_and_empty_values_preserve_content() {
    let mut writer = PrettyWriter::new(Vec::new());
    writer
        .write_event(Event::Start(BytesStart::new("p")))
        .unwrap();
    writer
        .write_serializable("ignored", &Content::Text(" hello & <world> "))
        .unwrap();
    writer.write_serializable("ignored", &None::<u8>).unwrap();
    writer
        .write_serializable("ignored", &Vec::<u8>::new())
        .unwrap();
    writer.write_event(Event::End(BytesEnd::new("p"))).unwrap();
    assert_eq!(
        String::from_utf8(writer.into_inner().unwrap()).unwrap(),
        "<p> hello &amp; &lt;world&gt; </p>"
    );
}

#[test]
fn multiple_elements_and_empty_element_config() {
    let mut writer = PrettyWriter::new(Vec::new());
    writer.config_mut().add_space_before_slash_in_empty_elements = true;
    writer
        .write_event(Event::Start(BytesStart::new("root")))
        .unwrap();
    writer.write_serializable("item", &vec![(), ()]).unwrap();
    writer
        .write_event(Event::End(BytesEnd::new("root")))
        .unwrap();
    assert_eq!(
        String::from_utf8(writer.into_inner().unwrap()).unwrap(),
        "<root>\n  <item />\n  <item />\n</root>"
    );
}

#[test]
fn failed_serialization_leaves_buffer_unchanged() {
    use serde::ser::Error;
    struct Fail;
    impl Serialize for Fail {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            use serde::ser::SerializeSeq;
            let mut seq = serializer.serialize_seq(Some(2))?;
            seq.serialize_element(&1)?;
            Err(S::Error::custom("failure"))
        }
    }
    let mut writer = PrettyWriter::new(Vec::new());
    writer
        .write_event(Event::Start(BytesStart::new("root")))
        .unwrap();
    assert!(writer.write_serializable("item", &Fail).is_err());
    writer
        .write_event(Event::Text(BytesText::new("ok")))
        .unwrap();
    writer
        .write_event(Event::End(BytesEnd::new("root")))
        .unwrap();
    assert_eq!(
        String::from_utf8(writer.into_inner().unwrap()).unwrap(),
        "<root>ok</root>"
    );
}

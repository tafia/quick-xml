use quick_xml::events::{BytesCData, BytesDecl, BytesEnd, BytesPI, BytesStart, BytesText, Event};
use quick_xml::writer::PrettyWriter;

#[test]
fn element_builder_supports_nested_content_and_attributes() {
    let mut writer = PrettyWriter::new(Vec::new());
    writer
        .create_element("root")
        .write_inner_content(|writer| {
            writer
                .create_element("child")
                .new_line()
                .with_attribute(("name", "value"))
                .new_line()
                .with_attributes([("one", "1"), ("two", "2")])
                .write_empty()?;
            writer
                .create_element("text")
                .write_text_content(BytesText::new("a & b"))?;
            writer
                .create_element("cdata")
                .write_cdata_content(BytesCData::new("x < y"))?;
            writer
                .create_element("pi")
                .write_pi_content(BytesPI::new("target"))?;
            Ok(())
        })
        .unwrap();
    assert_eq!(
        String::from_utf8(writer.into_inner().unwrap()).unwrap(),
        "<root>\n  <child\n    name=\"value\"\n    one=\"1\" two=\"2\"/>\n  <text>a &amp; b</text>\n  <cdata><![CDATA[x < y]]></cdata>\n  <pi>\n    <?target?>\n  </pi>\n</root>"
    );
}

#[test]
fn bom_and_declaration_preserve_order() {
    let mut writer = PrettyWriter::new(Vec::new());
    writer.write_bom().unwrap();
    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", None, None)))
        .unwrap();
    writer.create_element("root").write_empty().unwrap();
    assert_eq!(
        String::from_utf8(writer.into_inner().unwrap()).unwrap(),
        "\u{feff}<?xml version=\"1.0\"?><root/>"
    );
}

#[test]
fn manual_indent_is_preserved_inside_mixed_content() {
    let mut writer = PrettyWriter::new_with_indent(Vec::new(), '·', 2);
    writer
        .write_event(Event::Start(BytesStart::new("p")))
        .unwrap();
    writer
        .write_event(Event::Text(BytesText::new("before")))
        .unwrap();
    writer.write_indent().unwrap();
    writer.create_element("br").write_empty().unwrap();
    writer
        .write_event(Event::Text(BytesText::new("after")))
        .unwrap();
    writer.write_event(Event::End(BytesEnd::new("p"))).unwrap();
    assert_eq!(
        String::from_utf8(writer.into_inner().unwrap()).unwrap(),
        "<p>before\n··<br/>after</p>"
    );
}

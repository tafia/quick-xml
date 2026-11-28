#![cfg(feature = "serialize")]

use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::writer::Writer;

#[test]
#[ignore = "an empty nested sequence incorrectly changes the serializer's indentation state"]
fn empty_sequence_before_element_keeps_text_inline() {
    let mut writer = Writer::new_with_indent(Vec::new(), ' ', 2);
    writer
        .write_event(Event::Start(BytesStart::new("root")))
        .unwrap();
    writer
        .write_event(Event::Text(BytesText::new("text")))
        .unwrap();
    writer
        .write_serializable("item", &vec![vec![], vec![42]])
        .unwrap();
    writer
        .write_event(Event::End(BytesEnd::new("root")))
        .unwrap();

    assert_eq!(
        String::from_utf8(writer.into_inner()).unwrap(),
        "<root>text<item>42</item>\n</root>"
    );
}

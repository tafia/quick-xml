use quick_xml::XmlVersion;
use quick_xml::escape::unescape;
use quick_xml::events::{BytesCData, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;

fn write_text_element(value: &str) -> String {
    let mut writer = Writer::new(Vec::new());
    writer
        .write_event(Event::Start(BytesStart::new("value")))
        .unwrap();
    writer
        .write_event(Event::Text(BytesText::new(value)))
        .unwrap();
    writer
        .write_event(Event::End(BytesEnd::new("value")))
        .unwrap();
    String::from_utf8(writer.into_inner()).unwrap()
}

fn write_attr_element(value: &str) -> String {
    let mut writer = Writer::new(Vec::new());
    let mut start = BytesStart::new("root");
    start.push_attribute(("value", value));
    writer.write_event(Event::Empty(start)).unwrap();
    String::from_utf8(writer.into_inner()).unwrap()
}

fn write_cdata_element(value: &str) -> String {
    let mut writer = Writer::new(Vec::new());
    writer
        .write_event(Event::Start(BytesStart::new("value")))
        .unwrap();
    writer
        .write_event(Event::CData(BytesCData::new(value)))
        .unwrap();
    writer
        .write_event(Event::End(BytesEnd::new("value")))
        .unwrap();
    String::from_utf8(writer.into_inner()).unwrap()
}

fn parse_text(xml: &str) -> String {
    let mut reader = Reader::from_str(xml);
    loop {
        match reader.read_event().unwrap() {
            Event::Start(e) if e.name().as_ref() == "value" => {
                let text = reader.read_text(e.name()).unwrap();
                let normalized = text.xml10_content();
                return unescape(&normalized).unwrap().into_owned();
            }
            Event::Eof => panic!("unexpected EOF"),
            _ => {}
        }
    }
}

fn parse_attr(xml: &str) -> String {
    let mut reader = Reader::from_str(xml);
    loop {
        match reader.read_event().unwrap() {
            Event::Start(e) | Event::Empty(e) => {
                let attr = e.try_get_attribute("value").unwrap().unwrap();
                return attr
                    .normalized_value(XmlVersion::Implicit1_0)
                    .unwrap()
                    .into_owned();
            }
            Event::Eof => panic!("unexpected EOF"),
            _ => {}
        }
    }
}

fn parse_cdata_text(xml: &str) -> String {
    let mut reader = Reader::from_str(xml);
    loop {
        match reader.read_event().unwrap() {
            Event::Start(e) if e.name().as_ref() == "value" => {
                let text = reader.read_text(e.name()).unwrap();
                let normalized = text.xml10_content();
                return unescape(&normalized).unwrap().into_owned();
            }
            Event::Eof => panic!("unexpected EOF"),
            _ => {}
        }
    }
}

fn main() {
    println!("=== TEXT CONTENT ===\n");

    for (label, value) in [
        ("CR", "hello\rworld"),
        ("CRLF", "hello\r\nworld"),
        ("LF", "hello\nworld"),
        ("TAB", "col1\tcol2"),
    ] {
        let xml = write_text_element(value);
        println!("{} in text - serialized: {:?}", label, xml);
        let parsed = parse_text(&xml);
        println!("{} in text - parsed:     {:?}", label, parsed);
        println!("{} in text - roundtrip:  {}\n", label, parsed == value);
    }

    println!("=== ATTRIBUTE VALUES ===\n");

    for (label, value) in [
        ("CR", "new\rline"),
        ("CRLF", "new\r\nline"),
        ("LF", "new\nline"),
        ("TAB", "col1\tcol2"),
    ] {
        let xml = write_attr_element(value);
        println!("{} in attr - serialized: {:?}", label, xml);
        let parsed = parse_attr(&xml);
        println!("{} in attr - parsed:     {:?}", label, parsed);
        println!("{} in attr - roundtrip:  {}\n", label, parsed == value);
    }

    println!("=== CDATA ===\n");

    for (label, value) in [
        ("CR", "hello\rworld"),
        ("CRLF", "hello\r\nworld"),
        ("LF", "hello\nworld"),
    ] {
        let xml = write_cdata_element(value);
        println!("{} in CDATA - serialized: {:?}", label, xml);
        let parsed = parse_cdata_text(&xml);
        println!("{} in CDATA - parsed:     {:?}\n", label, parsed);
    }

    // Space in text
    let xml = write_text_element("with spaces");
    println!("Space in text - serialized: {:?}", xml);

    // Space in attr
    let xml = write_attr_element("with spaces");
    println!("Space in attr - serialized: {:?}", xml);
    let parsed = parse_attr(&xml);
    println!("Space in attr - parsed:     {:?}", parsed);
    println!("Space in attr - roundtrip:  {}", parsed == "with spaces");
}

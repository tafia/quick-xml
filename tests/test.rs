use quick_xml::events::attributes::Attribute;
use quick_xml::events::Event::*;
use quick_xml::name::QName;
use quick_xml::reader::Reader;
use std::borrow::Cow;

use pretty_assertions::assert_eq;

#[test]
fn test_sample() {
    let src = include_str!("documents/sample_rss.xml");
    let mut r = Reader::from_str(src);
    let mut count = 0;
    loop {
        match r.read_event().unwrap() {
            Start(_) => count += 1,
            Decl(e) => println!("{:?}", e.version()),
            Eof => break,
            _ => (),
        }
    }
    println!("{}", count);
}

#[test]
fn test_attributes_empty() {
    let src = "<a att1='a' att2='b'/>";
    let mut r = Reader::from_str(src);
    r.trim_text(true);
    match r.read_event() {
        Ok(Empty(e)) => {
            let mut attrs = e.attributes();
            assert_eq!(
                attrs.next(),
                Some(Ok(Attribute {
                    key: QName(b"att1"),
                    value: Cow::Borrowed(b"a"),
                }))
            );
            assert_eq!(
                attrs.next(),
                Some(Ok(Attribute {
                    key: QName(b"att2"),
                    value: Cow::Borrowed(b"b"),
                }))
            );
            assert_eq!(attrs.next(), None);
        }
        e => panic!("Expecting Empty event, got {:?}", e),
    }
}

#[test]
fn test_attribute_equal() {
    let src = "<a att1=\"a=b\"/>";
    let mut r = Reader::from_str(src);
    r.trim_text(true);
    match r.read_event() {
        Ok(Empty(e)) => {
            let mut attrs = e.attributes();
            assert_eq!(
                attrs.next(),
                Some(Ok(Attribute {
                    key: QName(b"att1"),
                    value: Cow::Borrowed(b"a=b"),
                }))
            );
            assert_eq!(attrs.next(), None);
        }
        e => panic!("Expecting Empty event, got {:?}", e),
    }
}

#[test]
fn test_clone_reader() {
    let mut reader = Reader::from_str("<tag>text</tag>");
    reader.trim_text(true);

    assert!(matches!(reader.read_event().unwrap(), Start(_)));

    let mut cloned = reader.clone();

    assert!(matches!(reader.read_event().unwrap(), Text(_)));
    assert!(matches!(reader.read_event().unwrap(), End(_)));

    assert!(matches!(cloned.read_event().unwrap(), Text(_)));
    assert!(matches!(cloned.read_event().unwrap(), End(_)));
}

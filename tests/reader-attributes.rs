use std::borrow::Cow;

use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesEnd, Event::*};
use quick_xml::name::QName;
use quick_xml::reader::Reader;

use pretty_assertions::assert_eq;

#[test]
fn single_gt() {
    let mut reader = Reader::from_str("<a attr='>' check='2'></a>");
    match reader.read_event() {
        Ok(Start(e)) => {
            let mut attrs = e.attributes();
            assert_eq!(
                attrs.next(),
                Some(Ok(Attribute {
                    key: QName(b"attr"),
                    value: Cow::Borrowed(b">"),
                }))
            );
            assert_eq!(
                attrs.next(),
                Some(Ok(Attribute {
                    key: QName(b"check"),
                    value: Cow::Borrowed(b"2"),
                }))
            );
            assert_eq!(attrs.next(), None);
        }
        x => panic!("expected <a attr='>'>, got {:?}", x),
    }
    assert_eq!(reader.read_event().unwrap(), End(BytesEnd::new("a")));
}

#[test]
fn single_gt_quot() {
    let mut reader = Reader::from_str(r#"<a attr='">"' check='"2"'></a>"#);
    match reader.read_event() {
        Ok(Start(e)) => {
            let mut attrs = e.attributes();
            assert_eq!(
                attrs.next(),
                Some(Ok(Attribute {
                    key: QName(b"attr"),
                    value: Cow::Borrowed(br#"">""#),
                }))
            );
            assert_eq!(
                attrs.next(),
                Some(Ok(Attribute {
                    key: QName(b"check"),
                    value: Cow::Borrowed(br#""2""#),
                }))
            );
            assert_eq!(attrs.next(), None);
        }
        x => panic!("expected <a attr='>'>, got {:?}", x),
    }
    assert_eq!(reader.read_event().unwrap(), End(BytesEnd::new("a")));
}

#[test]
fn double_gt() {
    let mut reader = Reader::from_str(r#"<a attr=">" check="2"></a>"#);
    match reader.read_event() {
        Ok(Start(e)) => {
            let mut attrs = e.attributes();
            assert_eq!(
                attrs.next(),
                Some(Ok(Attribute {
                    key: QName(b"attr"),
                    value: Cow::Borrowed(b">"),
                }))
            );
            assert_eq!(
                attrs.next(),
                Some(Ok(Attribute {
                    key: QName(b"check"),
                    value: Cow::Borrowed(b"2"),
                }))
            );
            assert_eq!(attrs.next(), None);
        }
        x => panic!("expected <a attr='>'>, got {:?}", x),
    }
    assert_eq!(reader.read_event().unwrap(), End(BytesEnd::new("a")));
}

#[test]
fn double_gt_apos() {
    let mut reader = Reader::from_str(r#"<a attr="'>'" check="'2'"></a>"#);
    match reader.read_event() {
        Ok(Start(e)) => {
            let mut attrs = e.attributes();
            assert_eq!(
                attrs.next(),
                Some(Ok(Attribute {
                    key: QName(b"attr"),
                    value: Cow::Borrowed(b"'>'"),
                }))
            );
            assert_eq!(
                attrs.next(),
                Some(Ok(Attribute {
                    key: QName(b"check"),
                    value: Cow::Borrowed(b"'2'"),
                }))
            );
            assert_eq!(attrs.next(), None);
        }
        x => panic!("expected <a attr='>'>, got {:?}", x),
    }
    assert_eq!(reader.read_event().unwrap(), End(BytesEnd::new("a")));
}

#[test]
fn empty_tag() {
    let mut reader = Reader::from_str("<a att1='a' att2='b'/>");
    match reader.read_event() {
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
fn equal_sign_in_value() {
    let mut reader = Reader::from_str("<a att1=\"a=b\"/>");
    match reader.read_event() {
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

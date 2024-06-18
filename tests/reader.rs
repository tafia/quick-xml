use std::str::from_utf8;

use quick_xml::events::{BytesCData, BytesEnd, BytesStart, BytesText, Event::*};
use quick_xml::reader::Reader;

use pretty_assertions::assert_eq;

#[test]
fn test_start_end() {
    let mut r = Reader::from_str("<a></a>");

    assert_eq!(r.read_event().unwrap(), Start(BytesStart::new("a")));
    assert_eq!(r.read_event().unwrap(), End(BytesEnd::new("a")));
}

#[test]
fn test_start_end_with_ws() {
    let mut r = Reader::from_str("<a></a >");

    assert_eq!(r.read_event().unwrap(), Start(BytesStart::new("a")));
    assert_eq!(r.read_event().unwrap(), End(BytesEnd::new("a")));
}

#[test]
fn test_start_end_attr() {
    let mut r = Reader::from_str("<a b=\"test\"></a>");

    assert_eq!(
        r.read_event().unwrap(),
        Start(BytesStart::from_content("a b=\"test\"", 1))
    );
    assert_eq!(r.read_event().unwrap(), End(BytesEnd::new("a")));
}

#[test]
fn test_empty_attr() {
    let mut r = Reader::from_str("<a b=\"test\" />");

    assert_eq!(
        r.read_event().unwrap(),
        Empty(BytesStart::from_content("a b=\"test\" ", 1))
    );
}

#[test]
fn test_start_end_comment() {
    let mut r = Reader::from_str("<b><a b=\"test\" c=\"test\"/> <a  /><!--t--></b>");
    r.config_mut().trim_text(true);

    assert_eq!(r.read_event().unwrap(), Start(BytesStart::new("b")));
    assert_eq!(
        r.read_event().unwrap(),
        Empty(BytesStart::from_content("a b=\"test\" c=\"test\"", 1))
    );
    assert_eq!(
        r.read_event().unwrap(),
        Empty(BytesStart::from_content("a  ", 1))
    );
    assert_eq!(r.read_event().unwrap(), Comment(BytesText::new("t")));
    assert_eq!(r.read_event().unwrap(), End(BytesEnd::new("b")));
}

#[test]
fn test_start_txt_end() {
    let mut r = Reader::from_str("<a>test</a>");

    assert_eq!(r.read_event().unwrap(), Start(BytesStart::new("a")));
    assert_eq!(r.read_event().unwrap(), Text(BytesText::new("test")));
    assert_eq!(r.read_event().unwrap(), End(BytesEnd::new("a")));
}

#[test]
fn test_comment() {
    let mut r = Reader::from_str("<!--test-->");

    assert_eq!(r.read_event().unwrap(), Comment(BytesText::new("test")));
}

#[test]
fn test_xml_decl() {
    let mut r = Reader::from_str("<?xml version=\"1.0\" encoding='utf-8'?>");
    match r.read_event().unwrap() {
        Decl(ref e) => {
            match e.version() {
                Ok(v) => assert_eq!(
                    &*v,
                    b"1.0",
                    "expecting version '1.0', got '{:?}",
                    from_utf8(&v)
                ),
                Err(e) => panic!("{:?}", e),
            }
            match e.encoding() {
                Some(Ok(v)) => assert_eq!(
                    &*v,
                    b"utf-8",
                    "expecting encoding 'utf-8', got '{:?}",
                    from_utf8(&v)
                ),
                Some(Err(e)) => panic!("{:?}", e),
                None => panic!("cannot find encoding"),
            }
            match e.standalone() {
                None => (),
                e => panic!("doesn't expect standalone, got {:?}", e),
            }
        }
        _ => panic!("unable to parse XmlDecl"),
    }
}

#[test]
fn test_cdata() {
    let mut r = Reader::from_str("<![CDATA[test]]>");

    assert_eq!(r.read_event().unwrap(), CData(BytesCData::new("test")));
}

#[test]
fn test_cdata_open_close() {
    let mut r = Reader::from_str("<![CDATA[test <> test]]>");

    assert_eq!(
        r.read_event().unwrap(),
        CData(BytesCData::new("test <> test"))
    );
}

#[test]
fn test_start_attr() {
    let mut r = Reader::from_str("<a b=\"c\">");

    assert_eq!(
        r.read_event().unwrap(),
        Start(BytesStart::from_content("a b=\"c\"", 1))
    );
}

#[test]
fn test_nested() {
    let mut r = Reader::from_str("<a><b>test</b><c/></a>");

    assert_eq!(r.read_event().unwrap(), Start(BytesStart::new("a")));
    assert_eq!(r.read_event().unwrap(), Start(BytesStart::new("b")));
    assert_eq!(r.read_event().unwrap(), Text(BytesText::new("test")));
    assert_eq!(r.read_event().unwrap(), End(BytesEnd::new("b")));
    assert_eq!(r.read_event().unwrap(), Empty(BytesStart::new("c")));
    assert_eq!(r.read_event().unwrap(), End(BytesEnd::new("a")));
}

#[test]
fn test_escaped_content() {
    let mut r = Reader::from_str("<a>&lt;test&gt;</a>");

    assert_eq!(r.read_event().unwrap(), Start(BytesStart::new("a")));
    match r.read_event() {
        Ok(Text(e)) => {
            assert_eq!(
                &*e,
                b"&lt;test&gt;",
                "content unexpected: expecting '&lt;test&gt;', got '{:?}'",
                from_utf8(&e)
            );
            match e.unescape() {
                Ok(c) => assert_eq!(c, "<test>"),
                Err(e) => panic!(
                    "cannot escape content at position {}: {:?}",
                    r.buffer_position(),
                    e
                ),
            }
        }
        Ok(e) => panic!("Expecting text event, got {:?}", e),
        Err(e) => panic!(
            "Cannot get next event at position {}: {:?}",
            r.buffer_position(),
            e
        ),
    }
    assert_eq!(r.read_event().unwrap(), End(BytesEnd::new("a")));
}

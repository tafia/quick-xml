use std::str::from_utf8;

use quick_xml::events::{BytesCData, BytesEnd, BytesRef, BytesStart, BytesText, Event::*};
use quick_xml::name::QName;
use quick_xml::reader::Reader;

use pretty_assertions::assert_eq;

// Import `small_buffers_tests!`
#[macro_use]
mod helpers;

small_buffers_tests!(
    #[test]
    read_event_into: std::io::BufReader<_>
);

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
    assert_eq!(r.read_event().unwrap(), GeneralRef(BytesRef::new("lt")));
    match r.read_event() {
        Ok(Text(e)) => {
            assert_eq!(
                &*e,
                b"test",
                "content unexpected: expecting 'test', got '{:?}'",
                from_utf8(&e)
            );
            match e.unescape() {
                Ok(c) => assert_eq!(c, "test"),
                Err(e) => panic!(
                    "cannot escape content at position {}: {:?}",
                    r.error_position(),
                    e
                ),
            }
        }
        Ok(e) => panic!("Expecting text event, got {:?}", e),
        Err(e) => panic!(
            "Cannot get next event at position {}: {:?}",
            r.error_position(),
            e
        ),
    }
    assert_eq!(r.read_event().unwrap(), GeneralRef(BytesRef::new("gt")));
    assert_eq!(r.read_event().unwrap(), End(BytesEnd::new("a")));
}

#[test]
fn it_works() {
    let src = include_str!("documents/sample_rss.xml");
    let mut reader = Reader::from_str(src);
    let mut count = 0;
    loop {
        match reader.read_event().unwrap() {
            Start(_) => count += 1,
            Decl(e) => println!("{:?}", e.version()),
            Eof => break,
            _ => (),
        }
    }
    println!("{}", count);
}

/// Checks that after cloning reader the parse state is independent in each copy
#[test]
fn clone_state() {
    let mut reader = Reader::from_str("<tag>text</tag>");

    assert!(matches!(reader.read_event().unwrap(), Start(_)));

    let mut cloned = reader.clone();

    assert!(matches!(reader.read_event().unwrap(), Text(_)));
    assert!(matches!(reader.read_event().unwrap(), End(_)));

    assert!(matches!(cloned.read_event().unwrap(), Text(_)));
    assert!(matches!(cloned.read_event().unwrap(), End(_)));
}

/// Ported tests from xml-rs crate from function `issue_105_unexpected_double_dash`
mod double_dash {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn text1() {
        let mut r = Reader::from_str("<hello>-- </hello>");

        assert_eq!(r.read_event().unwrap(), Start(BytesStart::new("hello")));
        assert_eq!(r.read_event().unwrap(), Text(BytesText::new("-- ")));
        assert_eq!(r.read_event().unwrap(), End(BytesEnd::new("hello")));
    }

    #[test]
    fn text2() {
        let mut r = Reader::from_str("<hello>--</hello>");

        assert_eq!(r.read_event().unwrap(), Start(BytesStart::new("hello")));
        assert_eq!(r.read_event().unwrap(), Text(BytesText::new("--")));
        assert_eq!(r.read_event().unwrap(), End(BytesEnd::new("hello")));
    }

    #[test]
    fn text3() {
        let mut r = Reader::from_str("<hello>--></hello>");

        assert_eq!(r.read_event().unwrap(), Start(BytesStart::new("hello")));
        assert_eq!(
            r.read_event().unwrap(),
            Text(BytesText::from_escaped("-->"))
        );
        assert_eq!(r.read_event().unwrap(), End(BytesEnd::new("hello")));
    }

    #[test]
    fn cdata() {
        let mut r = Reader::from_str("<hello><![CDATA[--]]></hello>");

        assert_eq!(r.read_event().unwrap(), Start(BytesStart::new("hello")));
        assert_eq!(r.read_event().unwrap(), CData(BytesCData::new("--")));
        assert_eq!(r.read_event().unwrap(), End(BytesEnd::new("hello")));
    }
}

/// This tests checks that read_to_end() correctly returns span even when
/// text is trimmed from both sides
mod read_to_end {
    use super::*;

    mod borrowed {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn text() {
            let mut r = Reader::from_str("<tag> text </tag>");
            //                            ^0   ^5    ^11
            r.config_mut().trim_text(true);

            assert_eq!(r.read_event().unwrap(), Start(BytesStart::new("tag")));
            assert_eq!(r.read_to_end(QName(b"tag")).unwrap(), 5..11);
            assert_eq!(r.read_event().unwrap(), Eof);
        }

        #[test]
        fn tag() {
            let mut r = Reader::from_str("<tag> <nested/> </tag>");
            //                            ^0   ^5         ^16
            r.config_mut().trim_text(true);

            assert_eq!(r.read_event().unwrap(), Start(BytesStart::new("tag")));
            assert_eq!(r.read_to_end(QName(b"tag")).unwrap(), 5..16);
            assert_eq!(r.read_event().unwrap(), Eof);
        }
    }

    mod buffered {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn text() {
            let mut r = Reader::from_str("<tag> text </tag>");
            //                            ^0   ^5    ^11
            r.config_mut().trim_text(true);

            let mut buf = Vec::new();
            assert_eq!(
                r.read_event_into(&mut buf).unwrap(),
                Start(BytesStart::new("tag"))
            );
            assert_eq!(r.read_to_end_into(QName(b"tag"), &mut buf).unwrap(), 5..11);
            assert_eq!(r.read_event_into(&mut buf).unwrap(), Eof);
        }

        #[test]
        fn tag() {
            let mut r = Reader::from_str("<tag> <nested/> </tag>");
            //                            ^0   ^5         ^16
            r.config_mut().trim_text(true);

            let mut buf = Vec::new();
            assert_eq!(
                r.read_event_into(&mut buf).unwrap(),
                Start(BytesStart::new("tag"))
            );
            assert_eq!(r.read_to_end_into(QName(b"tag"), &mut buf).unwrap(), 5..16);
            assert_eq!(r.read_event_into(&mut buf).unwrap(), Eof);
        }
    }
}

/// This tests checks that read_text() correctly returns text even when
/// text is trimmed from both sides
mod read_text {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn text() {
        let mut r = Reader::from_str("<tag> text </tag>");
        r.config_mut().trim_text(true);

        assert_eq!(r.read_event().unwrap(), Start(BytesStart::new("tag")));
        assert_eq!(r.read_text(QName(b"tag")).unwrap(), " text ");
        assert_eq!(r.read_event().unwrap(), Eof);
    }

    #[test]
    fn tag() {
        let mut r = Reader::from_str("<tag> <nested/> </tag>");
        r.config_mut().trim_text(true);

        assert_eq!(r.read_event().unwrap(), Start(BytesStart::new("tag")));
        assert_eq!(r.read_text(QName(b"tag")).unwrap(), " <nested/> ");
        assert_eq!(r.read_event().unwrap(), Eof);
    }
}

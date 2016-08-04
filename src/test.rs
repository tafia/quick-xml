use super::{AsStr, Element, XmlReader, XmlWriter};
use super::Event::*;
use super::error::ResultPos;
use std::str::from_utf8;
use std::io::Cursor;

macro_rules! next_eq {
    ($r: expr, $($t:path, $bytes:expr),*) => {
        $(
            match $r.next() {
                Some(Ok($t(ref e))) => {
                    assert!(e.name() == $bytes, "expecting {:?}, found {:?}",
                            from_utf8($bytes), e.content().as_str());
                },
                Some(Ok(e)) => {
                    assert!(false, "expecting {:?}, found {:?}",
                            $t(Element::from_buffer($bytes.to_vec(), 
                                                    0, 
                                                    $bytes.len(), 
                                                    $bytes.len())), 
                            e);
                },
                Some(Err((e, pos))) => {
                    assert!(false, "{:?} at buffer position {}", e, pos);
                },
                p => {
                    assert!(false, "expecting {:?}, found {:?}",
                            $t(Element::from_buffer($bytes.to_vec(), 
                                                    0, 
                                                    $bytes.len(), 
                                                    $bytes.len())), 
                            p);
                }
            }
        )*
    }
}

#[test]
fn test_start() {
    let mut r = XmlReader::from("<a>").trim_text(true);
    next_eq!(r, Start, b"a");
}

#[test]
fn test_start_end() {
    let mut r = XmlReader::from("<a></a>").trim_text(true);
    next_eq!(r, Start, b"a", End, b"a");
}

#[test]
fn test_start_end_attr() {
    let mut r = XmlReader::from("<a b=\"test\"></a>").trim_text(true);
    next_eq!(r, Start, b"a", End, b"a");
}

#[test]
fn test_empty() {
    let mut r = XmlReader::from("<a />")
        .trim_text(true)
        .expand_empty_elements(false);
    next_eq!(r, Empty, b"a");
}

#[test]
fn test_empty_can_be_expanded() {
    let mut r = XmlReader::from("<a />")
        .trim_text(true)
        .expand_empty_elements(true);
    next_eq!(r, Start, b"a", End, b"a");
}

#[test]
fn test_empty_attr() {
    let mut r = XmlReader::from("<a b=\"test\" />")
        .trim_text(true)
        .expand_empty_elements(false);
    next_eq!(r, Empty, b"a");
}

#[test]
fn test_start_end_comment() {
    let mut r = XmlReader::from("<b><a b=\"test\" c=\"test\"/> <a  /><!--t--></b>")
        .trim_text(true)
        .expand_empty_elements(false);
    next_eq!(r,
             Start,
             b"b",
             Empty,
             b"a",
             Empty,
             b"a",
             Comment,
             b"t",
             End,
             b"b");
}

#[test]
fn test_start_txt_end() {
    let mut r = XmlReader::from("<a>test</a>").trim_text(true);
    next_eq!(r, Start, b"a", Text, b"test", End, b"a");
}

#[test]
fn test_comment() {
    let mut r = XmlReader::from("<!--test-->").trim_text(true);
    next_eq!(r, Comment, b"test");
}

#[test]
fn test_xml_decl() {
    let mut r = XmlReader::from("<?xml version=\"1.0\" encoding='utf-8'?>")
        .trim_text(true);
    match r.next() {
        Some(Ok(Decl(ref e))) => {
            match e.version() {
                Ok(v) => {
                    assert!(v == b"1.0",
                            "expecting version '1.0', got '{:?}",
                            v.as_str())
                }
                Err(e) => assert!(false, "{:?}", e),
            }
            match e.encoding() {
                Some(Ok(v)) => {
                    assert!(v == b"utf-8",
                            "expecting encoding 'utf-8', got '{:?}",
                            v.as_str())
                }
                Some(Err(e)) => assert!(false, "{:?}", e),
                None => assert!(false, "cannot find encoding"),
            }
            match e.standalone() {
                None => assert!(true),
                e => assert!(false, "doesn't expect standalone, got {:?}", e),
            }
        }
        Some(Err((e, pos))) => {
            assert!(false, "{:?} at buffer position {}", e, pos);
        }
        _ => assert!(false, "unable to parse XmlDecl"),
    }
}

#[test]
fn test_trim_test() {
    let txt = "<a><b>  </b></a>";
    let mut r = XmlReader::from(txt).trim_text(true);
    next_eq!(r, Start, b"a", Start, b"b", End, b"b", End, b"a");

    let mut r = XmlReader::from(txt).trim_text(false);
    next_eq!(r,
             Text,
             b"",
             Start,
             b"a",
             Text,
             b"",
             Start,
             b"b",
             Text,
             b"  ",
             End,
             b"b",
             Text,
             b"",
             End,
             b"a");
}

#[test]
fn test_cdata() {
    let mut r = XmlReader::from("<![CDATA[test]]>").trim_text(true);
    next_eq!(r, CData, b"test");
}

#[test]
fn test_cdata_open_close() {
    let mut r = XmlReader::from("<![CDATA[test <> test]]>").trim_text(true);
    next_eq!(r, CData, b"test <> test");
}

#[test]
fn test_start_attr() {
    let mut r = XmlReader::from("<a b=\"c\">").trim_text(true);
    next_eq!(r, Start, b"a");
}

#[test]
fn test_nested() {
    let mut r = XmlReader::from("<a><b>test</b><c/></a>")
        .trim_text(true)
        .expand_empty_elements(false);
    next_eq!(r,
             Start,
             b"a",
             Start,
             b"b",
             Text,
             b"test",
             End,
             b"b",
             Empty,
             b"c",
             End,
             b"a");
}

#[test]
fn test_writer() {
    let txt = r#"<?xml version="1.0" encoding="utf-8"?><manifest xmlns:android="http://schemas.android.com/apk/res/android" package="com.github.sample" android:versionName="Lollipop" android:versionCode="5.1"><application android:label="SampleApplication"></application></manifest>"#;
    let reader = XmlReader::from(txt).trim_text(true);
    let mut writer = XmlWriter::new(Cursor::new(Vec::new()));
    for event in reader {
        assert!(writer.write(event.unwrap()).is_ok());
    }

    let result = writer.into_inner().into_inner();
    assert_eq!(result, txt.as_bytes());
}

#[test]
fn test_write_empty_element_attrs() {
    let str_from = r#"<source attr="val"/>"#;
    let expected = r#"<source attr="val"/>"#;
    let reader = XmlReader::from(str_from).expand_empty_elements(false);
    let mut writer = XmlWriter::new(Cursor::new(Vec::new()));
    for event in reader {
        assert!(writer.write(event.unwrap()).is_ok());
    }

    let result = writer.into_inner().into_inner();
    assert_eq!(String::from_utf8(result).unwrap(), expected);
}

#[test]
fn test_write_attrs() {
    let str_from = r#"<source attr="val"></source>"#;
    let expected = r#"<copy attr="val" a="b" c="d" x="y"></copy>"#;
    let reader = XmlReader::from(str_from).trim_text(true);
    let mut writer = XmlWriter::new(Cursor::new(Vec::new()));
    for event in reader {
        let event = event.unwrap();
        let event = match event {
            Start(elem) => {
                let mut attrs = elem.attributes()
                    .collect::<ResultPos<Vec<_>>>().unwrap();
                attrs.extend_from_slice(&[(b"a", b"b"), (b"c", b"d")]);
                let mut elem = Element::new("copy").with_attributes(attrs);
                elem.push_attribute("x", "y");
                Start(elem)
            }
            End(_elem) => End(Element::new("copy")),
            _ => event,
        };
        assert!(writer.write(event).is_ok());
    }

    let result = writer.into_inner().into_inner();
    assert_eq!(result, expected.as_bytes());
}

#[test]
fn test_buf_position() {
    let mut r = XmlReader::from("</a>")
        .trim_text(true)
        .with_check(true);

    match r.next() {
        Some(Err((_, 2))) => assert!(true), // error at char 2: no opening tag
        Some(Err((e, n))) => assert!(false, 
                                     "expecting buf_pos = 2, found {}, err: {:?}", n, e),
        e => assert!(false, "expecting error, found {:?}", e),
    }

    r = XmlReader::from("<a><!--b>")
        .trim_text(true)
        .with_check(true);

    next_eq!(r, Start, b"a");

    match r.next() {
        Some(Err((_, 5))) => assert!(true), // error at char 5: no closing --> tag found
        Some(Err((e, n))) => assert!(false, 
                                     "expecting buf_pos = 2, found {}, err: {:?}", n, e),
        e => assert!(false, "expecting error, found {:?}", e),
    }

}

#[test]
fn test_namespace() {
    let mut r = XmlReader::from("<a xmlns:myns='www1'><myns:b>in namespace!</myns:b></a>")
        .trim_text(true)
        .namespaced();;

    if let Some(Ok((None, Start(_)))) = r.next() {
    } else {
        assert!(false, "expecting start element with no namespace");
    }

    if let Some(Ok((Some(a), Start(_)))) = r.next() {
        if &*a == b"www1" {
            assert!(true);
        } else {
            assert!(false, "expecting namespace to resolve to 'www1'");
        }
    } else {
        assert!(false, "expecting namespace resolution");
    }
}

#[test]
fn test_escaped_content() {
    let mut r = XmlReader::from("<a>&lt;test&gt;</a>").trim_text(true);
    next_eq!(r, Start, b"a");
    match r.next() {
        Some(Ok(Text(ref e))) => {
            if e.content() != b"&lt;test&gt;" {
                panic!("content unexpected: expecting '&lt;test&gt;', got '{:?}'",
                       e.content().as_str());
            }
            match e.unescaped_content() {
                Ok(ref c) => {
                    if &**c != b"<test>" {
                        panic!("unescaped content unexpected: expecting '&lt;test&lt;', got '{:?}'",
                               c.as_str())
                    }
                }
                Err((e, i)) => panic!("cannot escape content at position {}: {:?}", i, e),
            }
        }
        Some(Ok(ref e)) => panic!("Expecting text event, got {:?}", e),
        Some(Err((e, i))) => panic!("Cannot get next event at position {}: {:?}", i, e),
        None => panic!("Expecting text event, got None"),
    }
    next_eq!(r, End, b"a");
}

#[test]
fn test_read_write_roundtrip_results_in_identity() {
    let input = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <section ns:label="header">
            <section ns:label="empty element section" />
            <section ns:label="start/end section"></section>
            <section ns:label="with text">data</section>
            </section>
    "#;

    let reader = XmlReader::from(input)
        .trim_text(false)
        .expand_empty_elements(false);
    let mut writer = XmlWriter::new(Cursor::new(Vec::new()));
    for event in reader {
        assert!(writer.write(event.unwrap()).is_ok());
    }

    let result = writer.into_inner().into_inner();
    assert_eq!(result, input.as_bytes());
}

use super::{AsStr, Element, XmlReader, XmlWriter};
use super::Event::*;
use super::error::Result;
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
                            $t(Element::from_buffer($bytes.to_vec(), 0, $bytes.len(), $bytes.len())), e);
                },
                Some(Err((e, pos))) => {
                    assert!(false, "{:?} at buffer position {}", e, pos);
                },
                p => {
                    assert!(false, "expecting {:?}, found {:?}", 
                            $t(Element::from_buffer($bytes.to_vec(), 0, $bytes.len(), $bytes.len())), p);
                }
            }
        )*
    }
}

#[test]
fn test_start() {
    let mut r = XmlReader::from_str("<a>").trim_text(true);
    next_eq!(r, Start, b"a");
}
   
#[test]
fn test_start_end() {
    let mut r = XmlReader::from_str("<a/>").trim_text(true);
    next_eq!(r, Start, b"a", End, b"a");
}
   
#[test]
fn test_start_end_attr() {
    let mut r = XmlReader::from_str("<a b=\"test\" />").trim_text(true);
    next_eq!(r, Start, b"a", End, b"a");
}
   
#[test]
fn test_start_end_comment() {
    let mut r = XmlReader::from_str("<b><a b=\"test\" c=\"test\" /> <a  /><!--t--></b>").trim_text(true);
    next_eq!(r, 
             Start, b"b",
             Start, b"a", 
             End, b"a",
             Start, b"a", 
             End, b"a",
             Comment, b"t",
             End, b"b"
            );
}

#[test]
fn test_start_txt_end() {
    let mut r = XmlReader::from_str("<a>test</a>").trim_text(true);
    next_eq!(r, Start, b"a", Text, b"test", End, b"a");
}

#[test]
fn test_comment() {
    let mut r = XmlReader::from_str("<!--test-->").trim_text(true);
    next_eq!(r, Comment, b"test");
}

#[test]
fn test_xml_decl() {
    let mut r = XmlReader::from_str("<?xml version=\"1.0\" encoding='utf-8'?>").trim_text(true);
    match r.next() {
        Some(Ok(Decl(ref e))) => {
            match e.version() {
                Ok(v) => assert!(v == b"1.0", "expecting version '1.0', got '{:?}", v.as_str()),
                Err(e) => assert!(false, "{:?}", e),
            }
            match e.encoding() {
                Some(Ok(v)) => assert!(v == b"utf-8", "expecting encoding 'utf-8', got '{:?}",
                                       v.as_str()),
                Some(Err(e)) => assert!(false, "{:?}", e),
                None => assert!(false, "cannot find encoding"),
            }
            match e.standalone() {
                None => assert!(true),
                e => assert!(false, "doesn't expect standalone, got {:?}", e),
            }
        },
        Some(Err((e, pos))) => {
            assert!(false, "{:?} at buffer position {}", e, pos);
        },
        _ => assert!(false, "unable to parse XmlDecl"),
    }
}

#[test]
fn test_trim_test() {
    let txt = "<a><b>  </b></a>";
    let mut r = XmlReader::from_str(&txt).trim_text(true);
    next_eq!(r, Start, b"a",
                Start, b"b",
                End, b"b",
                End, b"a");

    let mut r = XmlReader::from_str(&txt).trim_text(false);
    next_eq!(r, Text, b"",
                Start, b"a",
                Text, b"",
                Start, b"b",
                Text, b"  ",
                End, b"b",
                Text, b"",
                End, b"a");
}

#[test]
fn test_cdata() {
    let mut r = XmlReader::from_str("<![CDATA[test]]>").trim_text(true);
    next_eq!(r, CData, b"test");
}

#[test]
fn test_cdata_open_close() {
    let mut r = XmlReader::from_str("<![CDATA[test <> test]]>").trim_text(true);
    next_eq!(r, CData, b"test <> test");
}

#[test]
fn test_start_attr() {
    let mut r = XmlReader::from_str("<a b=\"c\">").trim_text(true);
    next_eq!(r, Start, b"a");
}

#[test]
fn test_nested() {
    let mut r = XmlReader::from_str("<a><b>test</b><c/></a>").trim_text(true);
    next_eq!(r, 
             Start, b"a", 
             Start, b"b", 
             Text, b"test", 
             End, b"b",
             Start, b"c", 
             End, b"c",
             End, b"a"
            );
}

#[test]
fn test_writer() {
    let str = r#"<?xml version="1.0" encoding="utf-8"?><manifest xmlns:android="http://schemas.android.com/apk/res/android" package="com.github.sample" android:versionName="Lollipop" android:versionCode="5.1"><application android:label="SampleApplication"></application></manifest>"#;
    let reader = XmlReader::from_str(&str).trim_text(true);
    let mut writer = XmlWriter::new(Cursor::new(Vec::new()));
    for event in reader {
        assert!(writer.write(event.unwrap()).is_ok());
    }

    let result = writer.into_inner().into_inner();
    assert_eq!(result, str.as_bytes());
}

#[test]
fn test_write_attrs() {
    let str_from = r#"<source attr="val"></source>"#;
    let expected = r#"<copy attr="val" a="b" c="d" x="y"></copy>"#;
    let reader = XmlReader::from_str(&str_from).trim_text(true);
    let mut writer = XmlWriter::new(Cursor::new(Vec::new()));
    for event in reader {
        let event = event.unwrap();
        let event = match event {
            Start(elem) => {
                let mut attrs = elem.attributes().collect::<Result<Vec<_>>>().unwrap();
                attrs.extend_from_slice(&[(b"a", b"b"), (b"c", b"d")]);
                let mut elem = Element::new("copy").with_attributes(attrs);
                elem.push_attribute("x", "y");
                Start(elem)
            },
            End(_elem) => End(Element::new("copy")),
            _ => event
        };
        assert!(writer.write(event).is_ok());
    }

    let result = writer.into_inner().into_inner();
    assert_eq!(result, expected.as_bytes());
}

#[test]
fn test_buf_position() {
    let mut r = XmlReader::from_str("</a>")
        .trim_text(true).with_check(true);

    match r.next() {
        Some(Err((_, 4))) => assert!(true),
        Some(Err((_, n))) => assert!(false, "expecting buf_pos = 4, found {}", n),
        e => assert!(false, "expecting error, found {:?}", e),
    }

}
   

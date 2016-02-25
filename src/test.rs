use super::{Element, XmlReader, XmlWriter};
use super::Event::*;
use std::str::from_utf8;
use std::io::Cursor;

macro_rules! next_eq {
    ($r: expr, $($t:path, $bytes:expr),*) => {
        $(
            match $r.next() {
                Some(Ok($t(ref e))) => {
                    assert!(e.as_bytes() == $bytes, "expecting {:?}, found {:?}", 
                            from_utf8($bytes), e.as_str());
                },
                Some(Ok(e)) => {
                    assert!(false, "expecting {:?}, found {:?}", 
                            $t(Element::from_buffer($bytes.to_vec(), 0, $bytes.len(), $bytes.len())), e);
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
fn test_header() {
    let mut r = XmlReader::from_str("<?header?>").trim_text(true);
    next_eq!(r, Header, b"header");
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
    let str_from = r#"<elem attr="val"></elem>"#;
    let expected = r#"<elem attr="val" another="one"></elem>"#;
    let reader = XmlReader::from_str(&str_from).trim_text(true);
    let mut writer = XmlWriter::new(Cursor::new(Vec::new()));
    for event in reader {
        let event = event.unwrap();
        let event = match event {
            Start(elem) => {
                let mut attrs = elem.attributes().map(|e| {
                    let (k,v) = e.unwrap();
                    (from_utf8(k).unwrap(), v)
                }).collect::<Vec<_>>();
                attrs.push(("another", "one"));
                let elem = Element::new("elem", attrs.into_iter());
                Start(elem)
            },
            _ => event
        };
        assert!(writer.write(event).is_ok());
    }

    let result = writer.into_inner().into_inner();
    assert_eq!(result, expected.as_bytes());
}

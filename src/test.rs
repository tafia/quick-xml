use super::{XmlReader, Event};

macro_rules! next_eq {
    ($r: expr, $($t:path, $bytes:expr),*) => {
        $(
            match $r.next() {
                Some(Ok($t(ref e))) => {
                    if e.as_bytes() == $bytes {
                        assert!(true)
                    } else {
                        println!("expecting {:?}, found {:?}", 
                                 ::std::str::from_utf8($bytes), e.as_str());
                        assert!(false)
                    }
                },
                Some(Ok(e)) => {
                    println!("found {:?}, {:?}", e, e.element().as_str());
                    assert!(false)
                },
                p => {
                    println!("found {:?}", p);
                    assert!(false)
                }
            }
        )*
    }
}

#[test]
fn test_start() {
    let mut r = XmlReader::from_str("<a>").trim_text(true);
    next_eq!(r, Event::Start, b"a");
}
   
#[test]
fn test_start_end() {
    let mut r = XmlReader::from_str("<a/>").trim_text(true);
    next_eq!(r, Event::Start, b"a", Event::End, b"a");
}

#[test]
fn test_start_txt_end() {
    let mut r = XmlReader::from_str("<a>test</a>").trim_text(true);
    next_eq!(r, Event::Start, b"a", Event::Text, b"test", Event::End, b"a");
}

#[test]
fn test_comment() {
    let mut r = XmlReader::from_str("<!--test-->").trim_text(true);
    next_eq!(r, Event::Comment, b"test");
}

#[test]
fn test_cdata() {
    let mut r = XmlReader::from_str("<![CDATA[test]]>").trim_text(true);
    next_eq!(r, Event::CData, b"test");
}

#[test]
fn test_cdata_open_close() {
    let mut r = XmlReader::from_str("<![CDATA[test <> test]]>").trim_text(true);
    next_eq!(r, Event::CData, b"test <> test");
}

#[test]
fn test_start_attr() {
    let mut r = XmlReader::from_str("<a b=\"c\">").trim_text(true);
    next_eq!(r, Event::Start, b"a");
}

#[test]
fn test_nested() {
    let mut r = XmlReader::from_str("<a><b>test</b><c/></a>").trim_text(true);
    next_eq!(r, 
             Event::Start, b"a", 
             Event::Start, b"b", 
             Event::Text, b"test", 
             Event::End, b"b",
             Event::Start, b"c", 
             Event::End, b"c",
             Event::End, b"a"
            );
}


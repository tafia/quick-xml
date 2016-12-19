extern crate quick_xml;

use quick_xml::XmlReader;
use quick_xml::Event::*;
use quick_xml::AsStr;
   
#[test]
fn test_sample() {
    let src: &[u8] = include_bytes!("sample_rss.xml");
    let r = XmlReader::from_reader(src);
    let mut count = 0;
    for e in r {
        match e.unwrap() {
            Start(_) => count += 1,
            Decl(e) => println!("{:?}", e.version()),
            _ => (),
        }
    }
    println!("{}", count);
}

#[test]
fn test_attributes_empty() {
    let src = b"<a att1='a' att2='b'/>";
    let mut r = XmlReader::from_reader(src as &[u8])
        .trim_text(true)
        .expand_empty_elements(false);
    match r.next() {
        Some(Ok(Empty(e))) => {
            let mut atts = e.attributes();
            match atts.next() {
                Some(Ok((b"att1", b"a"))) => (),
                e => panic!("Expecting att1='a' attribute, found {:?}", e),
            }
            match atts.next() {
                Some(Ok((b"att2", b"b"))) => (),
                e => panic!("Expecting att2='b' attribute, found {:?}", e),
            }
            match atts.next() {
                None => (),
                e => panic!("Expecting None, found {:?}", e),
            }
        },
        e => panic!("Expecting Empty event, got {:?}", e),
    }
}

#[test]
fn test_attribute_equal() {
    let src = b"<a att1=\"a=b\"/>";
    let mut r = XmlReader::from_reader(src as &[u8])
        .trim_text(true)
        .expand_empty_elements(false);
    match r.next() {
        Some(Ok(Empty(e))) => {
            let mut atts = e.attributes();
            match atts.next() {
                Some(Ok((b"att1", b"a=b"))) => (),
                e => panic!("Expecting att1=\"a=b\" attribute, found {:?}", e),
            }
            match atts.next() {
                None => (),
                e => panic!("Expecting None, found {:?}", e),
            }
        },
        e => panic!("Expecting Empty event, got {:?}", e),
    }
}

#[test]
fn test_koi8_r_encoding() {
    let src: &[u8] = include_bytes!("documents/opennews_all.rss");
    let mut r = XmlReader::from_reader(src as &[u8])
        .trim_text(true)
        .expand_empty_elements(false);
    let mut decoder = None;
    for e in &mut r {
        match e.unwrap() {
            Decl(decl) => decoder = decl.encoder().unwrap(),
            Text(e) => {
                if let Err(e) = e.content().as_string(decoder.as_ref()) {
                    panic!("{:?}", e);
                }
            },
            _ => (),
        }
    }
}

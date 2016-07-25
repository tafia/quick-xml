extern crate quick_xml;

use quick_xml::XmlReader;
use quick_xml::Event::*;
   
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
    let mut r = XmlReader::from_reader(src as &[u8]).trim_text(true);
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

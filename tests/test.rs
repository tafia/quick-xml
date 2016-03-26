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

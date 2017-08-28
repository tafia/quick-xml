#![feature(test)]

extern crate test;
extern crate xml;

use test::Bencher;
use xml::reader::{EventReader, XmlEvent};

#[bench]
fn bench_xml_rs(b: &mut Bencher) {
    let src: &[u8] = include_bytes!("../tests/sample_rss.xml");
    b.iter(|| {
        let r = EventReader::new(src);
        let mut count = test::black_box(0);
        for e in r {
            if let Ok(XmlEvent::StartElement { .. }) = e {
                count += 1;
            }
        }
        assert_eq!(count, 1550);
    });
}

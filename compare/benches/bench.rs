#![feature(test)]

extern crate fast_xml;
extern crate test;
extern crate xml;

use fast_xml::{events::Event, Reader};
use test::Bencher;
use xml::reader::{EventReader, XmlEvent};

#[bench]
fn bench_fast_xml(b: &mut Bencher) {
    let src: &[u8] = include_bytes!("../../tests/sample_rss.xml");
    b.iter(|| {
        let mut r = Reader::from_reader(src);
        r.check_end_names(false).check_comments(false);
        let mut count = test::black_box(0);
        let mut buf = Vec::new();
        loop {
            match r.read_event(&mut buf) {
                Ok(Event::Start(_)) | Ok(Event::Empty(_)) => count += 1,
                Ok(Event::Eof) => break,
                _ => (),
            }
            buf.clear();
        }
        assert_eq!(count, 1550);
    });
}

#[bench]
fn bench_xml_rs(b: &mut Bencher) {
    let src: &[u8] = include_bytes!("../../tests/sample_rss.xml");
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

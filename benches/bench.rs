#![feature(test)]

extern crate quick_xml;
extern crate test;

use test::Bencher;
use quick_xml::events::BytesEvent;
use quick_xml::reader::Reader;

#[bench]
fn bench_quick_xml(b: &mut Bencher) {
    let src: &[u8] = include_bytes!("../tests/sample_rss.xml");
    b.iter(|| {
        let mut r = Reader::from_reader(src);
        r.check_end_names(false).check_comments(false);
        let mut count = test::black_box(0);
        let mut buf = Vec::new();
        loop {
            match r.read_event(&mut buf) {
                Ok(BytesEvent::Start(_)) | Ok(BytesEvent::Empty(_)) => count += 1,
                Ok(BytesEvent::Eof) => break,
                _ => (),
            }
            buf.clear();
        }
        assert_eq!(count, 1550);
    });
}

#[bench]
fn bench_quick_xml_namespaced(b: &mut Bencher) {
    let src: &[u8] = include_bytes!("../tests/sample_rss.xml");
    b.iter(|| {
        let mut r = Reader::from_reader(src);
        r.check_end_names(false).check_comments(false);
        let mut count = test::black_box(0);
        let mut buf = Vec::new();
        loop {
            match r.read_namespaced_event(&mut buf) {
                Ok((_, BytesEvent::Start(_))) | Ok((_, BytesEvent::Empty(_))) => count += 1,
                Ok((_, BytesEvent::Eof)) => break,
                _ => ()
            }
            buf.clear();
        }
        assert_eq!(count, 1550);
    });
}

#[bench]
fn bench_quick_xml_escaped(b: &mut Bencher) {
    let src: &[u8] = include_bytes!("../tests/sample_rss.xml");
    b.iter(|| {
        let mut buf = Vec::new();
        let mut r = Reader::from_reader(src);
        r.check_end_names(false).check_comments(false);
        let mut count = test::black_box(0);
        let mut nbtxt = test::black_box(0);
        loop {
            match r.read_event(&mut buf) {
                Ok(BytesEvent::Start(_)) | Ok(BytesEvent::Empty(_)) => count += 1,
                Ok(BytesEvent::Text(ref e)) => nbtxt += e.unescaped_content().unwrap().len(),
                Ok(BytesEvent::Eof) => break,
                _ => (),
            }
            buf.clear();
        }
        assert_eq!(count, 1550);
        assert_eq!(nbtxt, 66277);
    });
}

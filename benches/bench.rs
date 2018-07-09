#![feature(test)]

extern crate quick_xml;
extern crate test;

use quick_xml::events::Event;
use quick_xml::Reader;
use test::Bencher;

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
fn bench_quick_xml_namespaced(b: &mut Bencher) {
    let src: &[u8] = include_bytes!("../tests/sample_rss.xml");
    b.iter(|| {
        let mut r = Reader::from_reader(src);
        r.check_end_names(false).check_comments(false);
        let mut count = test::black_box(0);
        let mut buf = Vec::new();
        let mut ns_buf = Vec::new();
        loop {
            match r.read_namespaced_event(&mut buf, &mut ns_buf) {
                Ok((_, Event::Start(_))) | Ok((_, Event::Empty(_))) => count += 1,
                Ok((_, Event::Eof)) => break,
                _ => (),
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
                Ok(Event::Start(_)) | Ok(Event::Empty(_)) => count += 1,
                Ok(Event::Text(ref e)) => nbtxt += e.unescaped().unwrap().len(),
                Ok(Event::Eof) => break,
                _ => (),
            }
            buf.clear();
        }
        assert_eq!(count, 1550);
        assert_eq!(nbtxt, 66277);
    });
}

#![feature(test)]

extern crate quick_xml;
extern crate test;

use test::{Bencher};
use quick_xml::{XmlReader, Event};

#[bench]
fn bench_quick_xml(b: &mut Bencher) {
    let src: &[u8] = include_bytes!("../tests/sample_rss.xml");
    b.iter(|| {
        let r = XmlReader::from_reader(src);
        let mut count = test::black_box(0);
        for e in r {
            match e {
                Ok(Event::Start(_)) | Ok(Event::Empty(_)) => count += 1,
                _ => (),
            }
        }
        assert_eq!(count, 1550);
    });
}

#[bench]
fn bench_quick_xml_namespaced(b: &mut Bencher) {
    let src: &[u8] = include_bytes!("../tests/sample_rss.xml");
    b.iter(|| {
        let r = XmlReader::from_reader(src).namespaced();
        let mut count = test::black_box(0);
        for e in r {
            if let Ok((_, Event::Start(_))) = e {
                count += 1;
            }
        }
        assert_eq!(count, 1247);
    });
}

#[bench]
fn bench_quick_xml_escaped(b: &mut Bencher) {
    let src: &[u8] = include_bytes!("../tests/sample_rss.xml");
    b.iter(|| {
        let r = XmlReader::from_reader(src);
        let mut count = test::black_box(0);
        let mut nbtxt = test::black_box(0);
        for e in r {
            match e {
                Ok(Event::Start(_)) => count += 1,
                Ok(Event::Text(ref e)) => nbtxt += e.unescaped_content().unwrap().len(),
                _ => (),
            }
        }
        assert_eq!(count, 1247);
        assert_eq!(nbtxt, 66277);
    });
}


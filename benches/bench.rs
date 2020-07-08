#![feature(test)]

extern crate quick_xml;
extern crate test;

use quick_xml::events::Event;
use quick_xml::Reader;
use test::Bencher;

#[bench]
fn bench_quick_xml_normal(b: &mut Bencher) {
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

        // Windows has \r\n instead of \n
        #[cfg(windows)]
        assert_eq!(nbtxt, 67661);

        #[cfg(not(windows))]
        assert_eq!(nbtxt, 66277);
    });
}

#[bench]
fn bench_quick_xml_normal_trimmed(b: &mut Bencher) {
    let src: &[u8] = include_bytes!("../tests/sample_rss.xml");
    b.iter(|| {
        let mut r = Reader::from_reader(src);
        r.check_end_names(false)
            .check_comments(false)
            .trim_text(true);
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
fn bench_quick_xml_namespaced_trimmed(b: &mut Bencher) {
    let src: &[u8] = include_bytes!("../tests/sample_rss.xml");
    b.iter(|| {
        let mut r = Reader::from_reader(src);
        r.check_end_names(false)
            .check_comments(false)
            .trim_text(true);
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
fn bench_quick_xml_escaped_trimmed(b: &mut Bencher) {
    let src: &[u8] = include_bytes!("../tests/sample_rss.xml");
    b.iter(|| {
        let mut buf = Vec::new();
        let mut r = Reader::from_reader(src);
        r.check_end_names(false)
            .check_comments(false)
            .trim_text(true);
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

        // Windows has \r\n instead of \n
        #[cfg(windows)]
        assert_eq!(nbtxt, 50334);

        #[cfg(not(windows))]
        assert_eq!(nbtxt, 50261);
    });
}

#[bench]
fn bench_quick_xml_one_text_event(b: &mut Bencher) {
    let src = "Hello world!".repeat(512 / 12).into_bytes();
    let mut buf = Vec::with_capacity(1024);
    b.iter(|| {
        let mut r = Reader::from_reader(src.as_ref());
        let mut nbtxt = test::black_box(0);
        r.check_end_names(false).check_comments(false);
        match r.read_event(&mut buf) {
            Ok(Event::Text(ref e)) => nbtxt += e.unescaped().unwrap().len(),
            something_else => panic!("Did not expect {:?}", something_else),
        };

        buf.clear();

        assert_eq!(nbtxt, 504);
    })
}

#[bench]
fn bench_quick_xml_one_start_event_trimmed(b: &mut Bencher) {
    let src = format!(r#"<hello target="{}">"#, "world".repeat(512 / 5)).into_bytes();
    let mut buf = Vec::with_capacity(1024);
    b.iter(|| {
        let mut r = Reader::from_reader(src.as_ref());
        let mut nbtxt = test::black_box(0);
        r.check_end_names(false)
            .check_comments(false)
            .trim_text(true);
        match r.read_event(&mut buf) {
            Ok(Event::Start(ref e)) => nbtxt += e.unescaped().unwrap().len(),
            something_else => panic!("Did not expect {:?}", something_else),
        };

        buf.clear();

        assert_eq!(nbtxt, 525);
    })
}

#[bench]
fn bench_quick_xml_one_comment_event_trimmed(b: &mut Bencher) {
    let src = format!(r#"<!-- hello "{}" -->"#, "world".repeat(512 / 5)).into_bytes();
    let mut buf = Vec::with_capacity(1024);
    b.iter(|| {
        let mut r = Reader::from_reader(src.as_ref());
        let mut nbtxt = test::black_box(0);
        r.check_end_names(false)
            .check_comments(false)
            .trim_text(true);
        match r.read_event(&mut buf) {
            Ok(Event::Comment(ref e)) => nbtxt += e.unescaped().unwrap().len(),
            something_else => panic!("Did not expect {:?}", something_else),
        };

        buf.clear();

        assert_eq!(nbtxt, 520);
    })
}

#[bench]
fn bench_quick_xml_one_cdata_event_trimmed(b: &mut Bencher) {
    let src = format!(r#"<![CDATA[hello "{}"]]>"#, "world".repeat(512 / 5)).into_bytes();
    let mut buf = Vec::with_capacity(1024);
    b.iter(|| {
        let mut r = Reader::from_reader(src.as_ref());
        let mut nbtxt = test::black_box(0);
        r.check_end_names(false)
            .check_comments(false)
            .trim_text(true);
        match r.read_event(&mut buf) {
            Ok(Event::CData(ref e)) => nbtxt += e.unescaped().unwrap().len(),
            something_else => panic!("Did not expect {:?}", something_else),
        };

        buf.clear();

        assert_eq!(nbtxt, 518);
    })
}

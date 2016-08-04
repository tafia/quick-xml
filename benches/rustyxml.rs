#![feature(test)]
#![cfg(feature = "bench-rusty-xml")]

extern crate xml;
extern crate test;

use test::{Bencher, black_box};
use xml::{Event, Parser};

#[bench]
fn bench_rusty_xml(b: &mut Bencher) {
    let src: &[u8] = include_bytes!("../tests/sample_rss.xml");
    let src = ::std::str::from_utf8(src).unwrap();
    b.iter(|| {
        let mut r = Parser::new();
        r.feed_str(src);
        let mut count = black_box(0);
        for e in r {
            match e {
                Ok(Event::ElementStart(_)) => count += 1,
                Ok(Event::ElementEmpty(_)) => count += 1,
            }
        }
        assert_eq!(count, 1550);
    });
}



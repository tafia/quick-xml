#![feature(test)]

extern crate pretty_assertions;
extern crate quick_xml;
extern crate serde;
extern crate serde_xml_rs;
extern crate test;

use pretty_assertions::assert_eq;
use serde::Deserialize;
use test::Bencher;

const SOURCE: &str = include_str!("../../tests/sample_rss.xml");

#[derive(Debug, Deserialize)]
struct Rss {
    channel: Channel,
}

#[derive(Debug, Deserialize)]
struct Channel {
    title: String,
    #[serde(rename = "item", default)]
    items: Vec<Item>,
}

#[derive(Debug, Deserialize)]
struct Item {
    title: String,
    link: String,
    #[serde(rename = "pubDate")]
    pub_date: String,
    enclosure: Option<Enclosure>,
}

#[derive(Debug, Deserialize)]
struct Enclosure {
    url: String,
    length: String,
    #[serde(rename = "type")]
    typ: String,
}

#[bench]
fn bench_serde_quick_xml(b: &mut Bencher) {
    b.iter(|| {
        let rss: Rss = quick_xml::de::from_str(SOURCE).unwrap();
        assert_eq!(rss.channel.items.len(), 99);
    });
}

#[bench]
fn bench_serde_xml_rs(b: &mut Bencher) {
    b.iter(|| {
        let rss: Rss = serde_xml_rs::from_str(SOURCE).unwrap();
        assert_eq!(rss.channel.items.len(), 99);
    });
}

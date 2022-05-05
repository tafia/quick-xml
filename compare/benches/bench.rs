use criterion::{self, criterion_group, criterion_main, Criterion};
use pretty_assertions::assert_eq;
use fast_xml::{self, events::Event, Reader};
use serde::Deserialize;
use serde_xml_rs;
use xml::reader::{EventReader, XmlEvent};

static SOURCE: &str = include_str!("../../tests/sample_rss.xml");

fn raw_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("raw_parse_comparison");

    group.bench_function("fast_xml_normal", |b| {
        b.iter(|| {
            let mut r = Reader::from_reader(SOURCE.as_bytes());
            r.check_end_names(false).check_comments(false);
            let mut count = criterion::black_box(0);
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
        })
    });

    group.bench_function("xml_rs_normal", |b| {
        b.iter(|| {
            let r = EventReader::new(SOURCE.as_bytes());
            let mut count = criterion::black_box(0);
            for e in r {
                if let Ok(XmlEvent::StartElement { .. }) = e {
                    count += 1;
                }
            }
            assert_eq!(count, 1550);
        })
    });
    group.finish();
}

fn serde_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("serde_parse_comparison");
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

    group.bench_function("fast_xml_serde", |b| {
        b.iter(|| {
            let rss: Rss = fast_xml::de::from_str(SOURCE).unwrap();
            assert_eq!(rss.channel.items.len(), 99);
        })
    });

    group.bench_function("xml_rs_serde", |b| {
        b.iter(|| {
            let rss: Rss = serde_xml_rs::from_str(SOURCE).unwrap();
            assert_eq!(rss.channel.items.len(), 99);
        });
    });
    group.finish();
}

criterion_group!(benches, raw_comparison, serde_comparison);
criterion_main!(benches);

use criterion::{self, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use pretty_assertions::assert_eq;
use serde::Deserialize;
use serde_xml_rs;
use std::hint::black_box;

static SAMPLE_RSS: &str = include_str!("../../tests/documents/sample_rss.xml");

/// Runs benchmarks for several XML libraries using serde deserialization
#[allow(dead_code)] // We do not use structs
fn serde_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("serde");

    #[derive(Debug, Deserialize)]
    struct Rss<E> {
        channel: Channel<E>,
    }

    #[derive(Debug, Deserialize)]
    struct Channel<E> {
        title: String,
        #[serde(rename = "item", default = "Vec::new")]
        items: Vec<Item<E>>,
    }

    #[derive(Debug, Deserialize)]
    struct Item<E> {
        title: String,
        link: String,
        #[serde(rename = "pubDate")]
        pub_date: String,
        enclosure: Option<E>,
    }

    #[derive(Debug, Deserialize)]
    struct Enclosure {
        #[serde(rename = "@url")]
        url: String,

        #[serde(rename = "@length")]
        length: String,

        #[serde(rename = "@type")]
        typ: String,
    }

    group.throughput(Throughput::Bytes(SAMPLE_RSS.len() as u64));

    group.bench_with_input(
        BenchmarkId::new("quick_xml", "sample_rss.xml"),
        SAMPLE_RSS,
        |b, input| {
            b.iter(|| {
                let rss: Rss<Enclosure> = black_box(quick_xml::de::from_str(input).unwrap());
                assert_eq!(rss.channel.items.len(), 99);
            })
        },
    );

    /* NOTE: Most parts of deserializer are not implemented yet, so benchmark failed
    group.bench_with_input(BenchmarkId::new("rapid-xml", "sample_rss.xml"), SAMPLE_RSS, |b, input| {
        use rapid_xml::de::Deserializer;
        use rapid_xml::parser::Parser;

        b.iter(|| {
            let mut r = Parser::new(input.as_bytes());
            let mut de = Deserializer::new(&mut r).unwrap();
            let rss = black_box(Rss::deserialize(&mut de).unwrap());
            assert_eq!(rss.channel.items.len(), 99);
        });
    });*/

    group.bench_with_input(
        BenchmarkId::new("xml_rs", "sample_rss.xml"),
        SAMPLE_RSS,
        |b, input| {
            b.iter(|| {
                let rss: Rss<Enclosure> = black_box(serde_xml_rs::from_str(input).unwrap());
                assert_eq!(rss.channel.items.len(), 99);
            })
        },
    );

    group.finish();
}

criterion_group!(benches, serde_comparison);
criterion_main!(benches);

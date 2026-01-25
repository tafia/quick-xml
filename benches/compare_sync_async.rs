// std::hint::black_box stable since 1.66, but our MSRV = 1.56.
// criterion::black_box is deprecated in since criterion 0.7.
// Running benchmarks assumed on current Rust version, so this should be fine
#![allow(clippy::incompatible_msrv)]
use criterion::{self, criterion_group, criterion_main, Criterion, Throughput};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::hint::black_box;

static SAMPLE_RSS: &[u8] = include_bytes!("../tests/documents/sample_rss.xml");

pub fn bench_sync(c: &mut Criterion) {
    let mut group = c.benchmark_group("compare_sync");

    group.throughput(Throughput::Bytes(SAMPLE_RSS.len() as u64));
    group.bench_function("sample_rss.xml", |b| {
        b.iter(|| {
            let mut r = Reader::from_reader(SAMPLE_RSS);
            let mut buf = Vec::new();
            while !matches!(black_box(r.read_event_into(&mut buf).unwrap()), Event::Eof) {
                buf.clear();
            }
        })
    });

    group.finish();
}

pub fn bench_async(c: &mut Criterion) {
    let mut group = c.benchmark_group("compare_async");

    group.throughput(Throughput::Bytes(SAMPLE_RSS.len() as u64));
    group.bench_function("sample_rss.xml", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let mut r = Reader::from_reader(SAMPLE_RSS);
                let mut buf = Vec::new();
                while !matches!(
                    black_box(r.read_event_into_async(&mut buf).await.unwrap()),
                    Event::Eof
                ) {
                    buf.clear();
                }
            })
    });

    group.finish();
}

criterion_group!(benches, bench_sync, bench_async,);
criterion_main!(benches);

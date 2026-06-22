//! Regression benchmark for [#969] / [#971]: iterating the attributes of a
//! single start tag must stay ~O(N) in the number of attributes.
//!
//! `BytesStart::attributes()` defaults to `with_checks(true)`, which rejects
//! duplicate attribute names. That check used to be a linear scan of every
//! previously seen name, making a tag with N distinct attributes cost O(N²) byte
//! comparisons -- a CPU-exhaustion vector on untrusted XML. This benchmark times
//! the check across a range of attribute counts; if the O(N²) behaviour ever
//! returns, the per-element time for the larger inputs will blow up.
//!
//! Run with `cargo bench --bench issue971`. It is also executed once per case as
//! a smoke test by `cargo test --benches` on CI.
//!
//! [#969]: https://github.com/tafia/quick-xml/issues/969
//! [#971]: https://github.com/tafia/quick-xml/pull/971

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use quick_xml::events::Event;
use quick_xml::reader::Reader;

/// Builds a single empty-element tag carrying `n` distinct attributes, as in the
/// [#969] proof of concept (`<e a00000000="" a00000001="" ... />`).
fn tag_with_attributes(n: usize) -> String {
    let mut xml = String::with_capacity(n * 13 + 8);
    xml.push_str("<e");
    for i in 0..n {
        xml.push_str(&format!(" a{:08}=\"\"", i));
    }
    xml.push_str("/>");
    xml
}

/// Iterates every attribute of the single start tag in `xml`, returning the
/// count. `checks` toggles the duplicate-name detection under test.
fn count_attributes(xml: &str, checks: bool) -> usize {
    let mut reader = Reader::from_str(xml);
    match reader.read_event() {
        Ok(Event::Empty(e)) => {
            let mut count = 0;
            for attr in e.attributes().with_checks(checks) {
                attr.expect("valid attribute");
                count += 1;
            }
            count
        }
        other => panic!("expected an empty element, got {:?}", other),
    }
}

fn duplicate_check(c: &mut Criterion) {
    let mut group = c.benchmark_group("issue971_attributes");
    for n in [16usize, 64, 256, 1024, 4096] {
        let xml = tag_with_attributes(n);
        assert_eq!(count_attributes(&xml, true), n);
        group.throughput(Throughput::Elements(n as u64));

        group.bench_with_input(BenchmarkId::new("with_checks(true)", n), &xml, |b, xml| {
            b.iter(|| count_attributes(xml, true))
        });
        group.bench_with_input(BenchmarkId::new("with_checks(false)", n), &xml, |b, xml| {
            b.iter(|| count_attributes(xml, false))
        });
    }
    group.finish();
}

criterion_group!(benches, duplicate_check);
criterion_main!(benches);

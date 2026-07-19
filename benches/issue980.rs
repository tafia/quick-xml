//! Regression benchmark for [#980]: resolving a namespace prefix (or the
//! default namespace) for an element name must stay ~O(1) amortized per
//! event, regardless of how deeply the document is nested.
//!
//! `NamespaceResolver::resolve_prefix` used to scan the whole in-scope
//! `bindings` stack on every call. That stack grows by one entry per
//! `xmlns[:prefix]` declaration on every open ancestor, so its length tracks
//! nesting depth: a lookup that misses cost O(depth), and a full document
//! O(depth²) -- a CPU-exhaustion vector on untrusted XML. This benchmark
//! resolves one element name per nesting level across a range of depths; if
//! the O(depth²) behaviour ever returns, the per-level cost for the deeper
//! inputs will blow up.
//!
//! Run with `cargo bench --bench issue980`. It is also executed once per
//! case as a smoke test by `cargo test --benches` on CI.
//!
//! [#980]: https://github.com/tafia/quick-xml/issues/980

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use quick_xml::events::Event;
use quick_xml::reader::NsReader;

/// Builds a document nested `n` levels deep with one `xmlns:a` declaration per
/// level, as in the [#980] proof of concept: `<e xmlns:a='x'>...</e>`. No
/// `xmlns=` is declared, so every element name lookup is a miss.
///
/// [#980]: https://github.com/tafia/quick-xml/issues/980
fn nested_document(n: usize) -> String {
    let mut xml = String::with_capacity(n * 32);
    xml.push_str(&"<e xmlns:a='x'>".repeat(n));
    xml.push_str(&"</e>".repeat(n));
    xml
}

/// Reads every event of `xml` through [`NsReader::read_resolved_event`],
/// resolving the namespace of each `Start`/`End` element name, and returns
/// the number of elements seen.
fn read_resolved(xml: &str) -> usize {
    let mut reader = NsReader::from_str(xml);
    let mut count = 0;
    loop {
        match reader.read_resolved_event() {
            Ok((_, Event::Start(_))) | Ok((_, Event::End(_))) => count += 1,
            Ok((_, Event::Eof)) => break,
            Ok(_) => {}
            Err(e) => panic!("unexpected parse error: {e:?}"),
        }
    }
    count
}

fn resolve_prefix_by_depth(c: &mut Criterion) {
    let mut group = c.benchmark_group("issue980_resolve_prefix");
    for n in [64usize, 256, 1024, 4096, 16384] {
        let xml = nested_document(n);
        assert_eq!(read_resolved(&xml), 2 * n);
        // One resolve_prefix call per Start/End event.
        group.throughput(Throughput::Elements((2 * n) as u64));

        group.bench_with_input(BenchmarkId::new("depth", n), &xml, |b, xml| {
            b.iter(|| read_resolved(xml))
        });
    }
    group.finish();
}

criterion_group!(benches, resolve_prefix_by_depth);
criterion_main!(benches);

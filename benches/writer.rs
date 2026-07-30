use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::writer::{PrettyWriter, Writer};
use std::hint::black_box;

/// Benchmark element-only content (pure structure, no text)
fn bench_element_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("element_only");

    // Small document: 10 nested elements
    group.bench_function("Writer (no indent) - small", |b| {
        b.iter(|| {
            let mut writer = Writer::new(Vec::new());
            writer
                .write_event(Event::Start(BytesStart::new("root")))
                .unwrap();
            for i in 0..10 {
                writer
                    .write_event(Event::Empty(
                        BytesStart::new(format!("item{}", i)).with_attributes(vec![
                            ("id", format!("{}", i).as_str()),
                            ("value", "test"),
                        ]),
                    ))
                    .unwrap();
            }
            writer
                .write_event(Event::End(BytesEnd::new("root")))
                .unwrap();
            black_box(writer.into_inner())
        })
    });

    group.bench_function("Writer (with indent) - small", |b| {
        b.iter(|| {
            let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);
            writer
                .write_event(Event::Start(BytesStart::new("root")))
                .unwrap();
            for i in 0..10 {
                writer
                    .write_event(Event::Empty(
                        BytesStart::new(format!("item{}", i)).with_attributes(vec![
                            ("id", format!("{}", i).as_str()),
                            ("value", "test"),
                        ]),
                    ))
                    .unwrap();
            }
            writer
                .write_event(Event::End(BytesEnd::new("root")))
                .unwrap();
            black_box(writer.into_inner())
        })
    });

    group.bench_function("PrettyWriter - small", |b| {
        b.iter(|| {
            let mut writer = PrettyWriter::new(Vec::new());
            writer
                .write_event(Event::Start(BytesStart::new("root")))
                .unwrap();
            for i in 0..10 {
                writer
                    .write_event(Event::Empty(
                        BytesStart::new(format!("item{}", i)).with_attributes(vec![
                            ("id", format!("{}", i).as_str()),
                            ("value", "test"),
                        ]),
                    ))
                    .unwrap();
            }
            writer
                .write_event(Event::End(BytesEnd::new("root")))
                .unwrap();
            black_box(writer.into_inner().unwrap())
        })
    });

    // Medium document: 100 nested elements
    group.bench_function("Writer (no indent) - medium", |b| {
        b.iter(|| {
            let mut writer = Writer::new(Vec::new());
            writer
                .write_event(Event::Start(BytesStart::new("root")))
                .unwrap();
            for i in 0..100 {
                writer
                    .write_event(Event::Empty(
                        BytesStart::new(format!("item{}", i)).with_attributes(vec![
                            ("id", format!("{}", i).as_str()),
                            ("value", "test"),
                        ]),
                    ))
                    .unwrap();
            }
            writer
                .write_event(Event::End(BytesEnd::new("root")))
                .unwrap();
            black_box(writer.into_inner())
        })
    });

    group.bench_function("Writer (with indent) - medium", |b| {
        b.iter(|| {
            let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);
            writer
                .write_event(Event::Start(BytesStart::new("root")))
                .unwrap();
            for i in 0..100 {
                writer
                    .write_event(Event::Empty(
                        BytesStart::new(format!("item{}", i)).with_attributes(vec![
                            ("id", format!("{}", i).as_str()),
                            ("value", "test"),
                        ]),
                    ))
                    .unwrap();
            }
            writer
                .write_event(Event::End(BytesEnd::new("root")))
                .unwrap();
            black_box(writer.into_inner())
        })
    });

    group.bench_function("PrettyWriter - medium", |b| {
        b.iter(|| {
            let mut writer = PrettyWriter::new(Vec::new());
            writer
                .write_event(Event::Start(BytesStart::new("root")))
                .unwrap();
            for i in 0..100 {
                writer
                    .write_event(Event::Empty(
                        BytesStart::new(format!("item{}", i)).with_attributes(vec![
                            ("id", format!("{}", i).as_str()),
                            ("value", "test"),
                        ]),
                    ))
                    .unwrap();
            }
            writer
                .write_event(Event::End(BytesEnd::new("root")))
                .unwrap();
            black_box(writer.into_inner().unwrap())
        })
    });

    group.finish();
}

/// Benchmark mixed content (text + elements)
fn bench_mixed_content(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_content");

    group.bench_function("Writer (no indent)", |b| {
        b.iter(|| {
            let mut writer = Writer::new(Vec::new());
            writer
                .write_event(Event::Start(BytesStart::new("doc")))
                .unwrap();
            for i in 0..20 {
                writer
                    .write_event(Event::Start(BytesStart::new("p")))
                    .unwrap();
                writer
                    .write_event(Event::Text(BytesText::new("Some text ")))
                    .unwrap();
                writer
                    .write_event(Event::Start(BytesStart::new("strong")))
                    .unwrap();
                writer
                    .write_event(Event::Text(BytesText::new(format!("bold{}", i).as_str())))
                    .unwrap();
                writer
                    .write_event(Event::End(BytesEnd::new("strong")))
                    .unwrap();
                writer
                    .write_event(Event::Text(BytesText::new(" more text")))
                    .unwrap();
                writer.write_event(Event::End(BytesEnd::new("p"))).unwrap();
            }
            writer
                .write_event(Event::End(BytesEnd::new("doc")))
                .unwrap();
            black_box(writer.into_inner())
        })
    });

    group.bench_function("Writer (with indent)", |b| {
        b.iter(|| {
            let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);
            writer
                .write_event(Event::Start(BytesStart::new("doc")))
                .unwrap();
            for i in 0..20 {
                writer
                    .write_event(Event::Start(BytesStart::new("p")))
                    .unwrap();
                writer
                    .write_event(Event::Text(BytesText::new("Some text ")))
                    .unwrap();
                writer
                    .write_event(Event::Start(BytesStart::new("strong")))
                    .unwrap();
                writer
                    .write_event(Event::Text(BytesText::new(format!("bold{}", i).as_str())))
                    .unwrap();
                writer
                    .write_event(Event::End(BytesEnd::new("strong")))
                    .unwrap();
                writer
                    .write_event(Event::Text(BytesText::new(" more text")))
                    .unwrap();
                writer.write_event(Event::End(BytesEnd::new("p"))).unwrap();
            }
            writer
                .write_event(Event::End(BytesEnd::new("doc")))
                .unwrap();
            black_box(writer.into_inner())
        })
    });

    group.bench_function("PrettyWriter", |b| {
        b.iter(|| {
            let mut writer = PrettyWriter::new(Vec::new());
            writer
                .write_event(Event::Start(BytesStart::new("doc")))
                .unwrap();
            for i in 0..20 {
                writer
                    .write_event(Event::Start(BytesStart::new("p")))
                    .unwrap();
                writer
                    .write_event(Event::Text(BytesText::new("Some text ")))
                    .unwrap();
                writer
                    .write_event(Event::Start(BytesStart::new("strong")))
                    .unwrap();
                writer
                    .write_event(Event::Text(BytesText::new(format!("bold{}", i).as_str())))
                    .unwrap();
                writer
                    .write_event(Event::End(BytesEnd::new("strong")))
                    .unwrap();
                writer
                    .write_event(Event::Text(BytesText::new(" more text")))
                    .unwrap();
                writer.write_event(Event::End(BytesEnd::new("p"))).unwrap();
            }
            writer
                .write_event(Event::End(BytesEnd::new("doc")))
                .unwrap();
            black_box(writer.into_inner().unwrap())
        })
    });

    group.finish();
}

/// Benchmark deeply nested structures
fn bench_deep_nesting(c: &mut Criterion) {
    let mut group = c.benchmark_group("deep_nesting");

    for depth in [5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::new("Writer (no indent)", depth),
            depth,
            |b, &depth| {
                b.iter(|| {
                    let mut writer = Writer::new(Vec::new());
                    for i in 0..depth {
                        writer
                            .write_event(Event::Start(BytesStart::new(format!("level{}", i))))
                            .unwrap();
                    }
                    for i in (0..depth).rev() {
                        writer
                            .write_event(Event::End(BytesEnd::new(format!("level{}", i))))
                            .unwrap();
                    }
                    black_box(writer.into_inner())
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("Writer (with indent)", depth),
            depth,
            |b, &depth| {
                b.iter(|| {
                    let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);
                    for i in 0..depth {
                        writer
                            .write_event(Event::Start(BytesStart::new(format!("level{}", i))))
                            .unwrap();
                    }
                    for i in (0..depth).rev() {
                        writer
                            .write_event(Event::End(BytesEnd::new(format!("level{}", i))))
                            .unwrap();
                    }
                    black_box(writer.into_inner())
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("PrettyWriter", depth),
            depth,
            |b, &depth| {
                b.iter(|| {
                    let mut writer = PrettyWriter::new(Vec::new());
                    for i in 0..depth {
                        writer
                            .write_event(Event::Start(BytesStart::new(format!("level{}", i))))
                            .unwrap();
                    }
                    for i in (0..depth).rev() {
                        writer
                            .write_event(Event::End(BytesEnd::new(format!("level{}", i))))
                            .unwrap();
                    }
                    black_box(writer.into_inner().unwrap())
                })
            },
        );
    }

    group.finish();
}

/// Benchmark large documents with varied content
fn bench_large_document(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_document");

    // 1000-element document with mixed content types
    group.bench_function("Writer (no indent)", |b| {
        b.iter(|| {
            let mut writer = Writer::new(Vec::new());
            writer
                .write_event(Event::Start(BytesStart::new("catalog")))
                .unwrap();
            for i in 0..1000 {
                writer
                    .write_event(Event::Start(BytesStart::new("item")))
                    .unwrap();
                writer
                    .write_event(Event::Empty(
                        BytesStart::new("id")
                            .with_attributes(vec![("value", format!("{}", i).as_str())]),
                    ))
                    .unwrap();
                writer
                    .write_event(Event::Start(BytesStart::new("name")))
                    .unwrap();
                writer
                    .write_event(Event::Text(BytesText::new(format!("Item {}", i).as_str())))
                    .unwrap();
                writer
                    .write_event(Event::End(BytesEnd::new("name")))
                    .unwrap();
                writer
                    .write_event(Event::End(BytesEnd::new("item")))
                    .unwrap();
            }
            writer
                .write_event(Event::End(BytesEnd::new("catalog")))
                .unwrap();
            black_box(writer.into_inner())
        })
    });

    group.bench_function("Writer (with indent)", |b| {
        b.iter(|| {
            let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);
            writer
                .write_event(Event::Start(BytesStart::new("catalog")))
                .unwrap();
            for i in 0..1000 {
                writer
                    .write_event(Event::Start(BytesStart::new("item")))
                    .unwrap();
                writer
                    .write_event(Event::Empty(
                        BytesStart::new("id")
                            .with_attributes(vec![("value", format!("{}", i).as_str())]),
                    ))
                    .unwrap();
                writer
                    .write_event(Event::Start(BytesStart::new("name")))
                    .unwrap();
                writer
                    .write_event(Event::Text(BytesText::new(format!("Item {}", i).as_str())))
                    .unwrap();
                writer
                    .write_event(Event::End(BytesEnd::new("name")))
                    .unwrap();
                writer
                    .write_event(Event::End(BytesEnd::new("item")))
                    .unwrap();
            }
            writer
                .write_event(Event::End(BytesEnd::new("catalog")))
                .unwrap();
            black_box(writer.into_inner())
        })
    });

    group.bench_function("PrettyWriter", |b| {
        b.iter(|| {
            let mut writer = PrettyWriter::new(Vec::new());
            writer
                .write_event(Event::Start(BytesStart::new("catalog")))
                .unwrap();
            for i in 0..1000 {
                writer
                    .write_event(Event::Start(BytesStart::new("item")))
                    .unwrap();
                writer
                    .write_event(Event::Empty(
                        BytesStart::new("id")
                            .with_attributes(vec![("value", format!("{}", i).as_str())]),
                    ))
                    .unwrap();
                writer
                    .write_event(Event::Start(BytesStart::new("name")))
                    .unwrap();
                writer
                    .write_event(Event::Text(BytesText::new(format!("Item {}", i).as_str())))
                    .unwrap();
                writer
                    .write_event(Event::End(BytesEnd::new("name")))
                    .unwrap();
                writer
                    .write_event(Event::End(BytesEnd::new("item")))
                    .unwrap();
            }
            writer
                .write_event(Event::End(BytesEnd::new("catalog")))
                .unwrap();
            black_box(writer.into_inner().unwrap())
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_element_only,
    bench_mixed_content,
    bench_deep_nesting,
    bench_large_document
);
criterion_main!(benches);

// std::hint::black_box stable since 1.66, but our MSRV = 1.56.
// criterion::black_box is deprecated in since criterion 0.7.
// Running benchmarks assumed on current Rust version, so this should be fine
#![allow(clippy::incompatible_msrv)]
use criterion::{self, criterion_group, criterion_main, Criterion};
use pretty_assertions::assert_eq;
use quick_xml::escape::{escape, unescape};
use quick_xml::events::Event;
use quick_xml::name::QName;
use quick_xml::reader::{NsReader, Reader};
use std::hint::black_box;

static SAMPLE: &str = include_str!("../tests/documents/sample_rss.xml");
static PLAYERS: &str = include_str!("../tests/documents/players.xml");

static LOREM_IPSUM_TEXT: &str =
"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt
ut labore et dolore magna aliqua. Hac habitasse platea dictumst vestibulum rhoncus est pellentesque.
Risus ultricies tristique nulla aliquet enim tortor at. Fermentum odio eu feugiat pretium nibh ipsum.
Volutpat sed cras ornare arcu dui. Scelerisque fermentum dui faucibus in ornare quam. Arcu cursus
euismod quis viverra nibh cras pulvinar mattis. Sed viverra tellus in hac habitasse platea. Quis
commodo odio aenean sed. Cursus in hac habitasse platea dictumst quisque sagittis purus.

Neque convallis a cras semper auctor. Sit amet mauris commodo quis imperdiet massa. Ac ut consequat
semper viverra nam libero justo laoreet sit. Adipiscing commodo elit at imperdiet dui accumsan.
Enim lobortis scelerisque fermentum dui faucibus in ornare. Natoque penatibus et magnis dis parturient
montes nascetur ridiculus mus. At lectus urna duis convallis convallis tellus id interdum. Libero
volutpat sed cras ornare arcu dui vivamus arcu. Cursus in hac habitasse platea dictumst quisque sagittis
purus. Consequat id porta nibh venenatis cras sed felis.";

/// Benchmarks the `Reader::read_event` function with all XML well-formless
/// checks disabled (with and without trimming content of $text nodes)
fn read_event(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_event");
    group.bench_function("trim_text = false", |b| {
        b.iter(|| {
            let mut r = Reader::from_str(SAMPLE);
            r.config_mut().check_end_names = false;
            let mut count = black_box(0);
            loop {
                match r.read_event() {
                    Ok(Event::Start(_)) | Ok(Event::Empty(_)) => count += 1,
                    Ok(Event::Eof) => break,
                    _ => (),
                }
            }
            assert_eq!(
                count, 1550,
                "Overall tag count in ./tests/documents/sample_rss.xml"
            );
        })
    });

    group.bench_function("trim_text = true", |b| {
        b.iter(|| {
            let mut r = Reader::from_str(SAMPLE);
            let config = r.config_mut();
            config.trim_text(true);
            config.check_end_names = false;
            let mut count = black_box(0);
            loop {
                match r.read_event() {
                    Ok(Event::Start(_)) | Ok(Event::Empty(_)) => count += 1,
                    Ok(Event::Eof) => break,
                    _ => (),
                }
            }
            assert_eq!(
                count, 1550,
                "Overall tag count in ./tests/documents/sample_rss.xml"
            );
        });
    });
    group.finish();
}

/// Benchmarks the `NsReader::read_resolved_event_into` function with all XML well-formless
/// checks disabled (with and without trimming content of $text nodes)
fn read_resolved_event_into(c: &mut Criterion) {
    let mut group = c.benchmark_group("NsReader::read_resolved_event_into");
    group.bench_function("trim_text = false", |b| {
        b.iter(|| {
            let mut r = NsReader::from_str(SAMPLE);
            r.config_mut().check_end_names = false;
            let mut count = black_box(0);
            loop {
                match r.read_resolved_event() {
                    Ok((_, Event::Start(_))) | Ok((_, Event::Empty(_))) => count += 1,
                    Ok((_, Event::Eof)) => break,
                    _ => (),
                }
            }
            assert_eq!(
                count, 1550,
                "Overall tag count in ./tests/documents/sample_rss.xml"
            );
        });
    });

    group.bench_function("trim_text = true", |b| {
        b.iter(|| {
            let mut r = NsReader::from_str(SAMPLE);
            let config = r.config_mut();
            config.trim_text(true);
            config.check_end_names = false;
            let mut count = black_box(0);
            loop {
                match r.read_resolved_event() {
                    Ok((_, Event::Start(_))) | Ok((_, Event::Empty(_))) => count += 1,
                    Ok((_, Event::Eof)) => break,
                    _ => (),
                }
            }
            assert_eq!(
                count, 1550,
                "Overall tag count in ./tests/documents/sample_rss.xml"
            );
        });
    });
    group.finish();
}

/// Benchmarks, how fast individual event parsed
fn one_event(c: &mut Criterion) {
    let mut group = c.benchmark_group("One event");

    group.bench_function("Start", |b| {
        let src = format!(r#"<hello target="{}">"#, "world".repeat(512 / 5));
        b.iter(|| {
            let mut r = Reader::from_str(&src);
            let mut nbtxt = black_box(0);
            let config = r.config_mut();
            config.trim_text(true);
            config.check_end_names = false;
            match r.read_event() {
                Ok(Event::Start(ref e)) => nbtxt += e.len(),
                something_else => panic!("Did not expect {:?}", something_else),
            };

            assert_eq!(nbtxt, 525);
        })
    });

    group.bench_function("Comment", |b| {
        let src = format!(r#"<!-- hello "{}" -->"#, "world".repeat(512 / 5));
        b.iter(|| {
            let mut r = Reader::from_str(&src);
            let mut nbtxt = black_box(0);
            let config = r.config_mut();
            config.trim_text(true);
            config.check_end_names = false;
            match r.read_event() {
                Ok(Event::Comment(e)) => nbtxt += e.xml10_content().unwrap().len(),
                something_else => panic!("Did not expect {:?}", something_else),
            };

            assert_eq!(nbtxt, 520);
        })
    });

    group.bench_function("CData", |b| {
        let src = format!(r#"<![CDATA[hello "{}"]]>"#, "world".repeat(512 / 5));
        b.iter(|| {
            let mut r = Reader::from_str(&src);
            let mut nbtxt = black_box(0);
            let config = r.config_mut();
            config.trim_text(true);
            config.check_end_names = false;
            match r.read_event() {
                Ok(Event::CData(ref e)) => nbtxt += e.len(),
                something_else => panic!("Did not expect {:?}", something_else),
            };

            assert_eq!(nbtxt, 518);
        })
    });
    group.finish();
}

/// Benchmarks parsing attributes from events
fn attributes(c: &mut Criterion) {
    let mut group = c.benchmark_group("attributes");
    group.bench_function("with_checks = true", |b| {
        b.iter(|| {
            let mut r = Reader::from_str(PLAYERS);
            r.config_mut().check_end_names = false;
            let mut count = black_box(0);
            loop {
                match r.read_event() {
                    Ok(Event::Empty(e)) => {
                        for attr in e.attributes() {
                            let _attr = attr.unwrap();
                            count += 1
                        }
                    }
                    Ok(Event::Eof) => break,
                    _ => (),
                }
            }
            assert_eq!(count, 1041);
        })
    });

    group.bench_function("with_checks = false", |b| {
        b.iter(|| {
            let mut r = Reader::from_str(PLAYERS);
            r.config_mut().check_end_names = false;
            let mut count = black_box(0);
            loop {
                match r.read_event() {
                    Ok(Event::Empty(e)) => {
                        for attr in e.attributes().with_checks(false) {
                            let _attr = attr.unwrap();
                            count += 1
                        }
                    }
                    Ok(Event::Eof) => break,
                    _ => (),
                }
            }
            assert_eq!(count, 1041);
        })
    });

    group.bench_function("try_get_attribute", |b| {
        b.iter(|| {
            let mut r = Reader::from_str(PLAYERS);
            r.config_mut().check_end_names = false;
            let mut count = black_box(0);
            loop {
                match r.read_event() {
                    Ok(Event::Empty(e)) if e.name() == QName(b"player") => {
                        for name in ["num", "status", "avg"] {
                            if let Some(_attr) = e.try_get_attribute(name).unwrap() {
                                count += 1
                            }
                        }
                        assert!(e
                            .try_get_attribute("attribute-that-doesn't-exist")
                            .unwrap()
                            .is_none());
                    }
                    Ok(Event::Eof) => break,
                    _ => (),
                }
            }
            assert_eq!(count, 150);
        })
    });

    group.finish();
}

/// Benchmarks normalizing attribute values
fn attribute_value_normalization(c: &mut Criterion) {
    let mut group = c.benchmark_group("attribute_value_normalization");

    group.bench_function("noop_short", |b| {
        b.iter(|| {
            black_box(unescape("foobar")).unwrap();
        })
    });

    group.bench_function("noop_long", |b| {
        b.iter(|| {
            black_box(unescape("just a bit of text without any entities")).unwrap();
        })
    });

    group.bench_function("replacement_chars", |b| {
        b.iter(|| {
            black_box(unescape("just a bit\n of text without\tany entities")).unwrap();
        })
    });

    group.bench_function("char_reference", |b| {
        b.iter(|| {
            let text = "prefix &#34;some stuff&#34;,&#x22;more stuff&#x22;";
            black_box(unescape(text)).unwrap();
            let text = "&#38;&#60;";
            black_box(unescape(text)).unwrap();
        })
    });

    group.bench_function("entity_reference", |b| {
        b.iter(|| {
            let text = "age &gt; 72 &amp;&amp; age &lt; 21";
            black_box(unescape(text)).unwrap();
            let text = "&quot;what&apos;s that?&quot;";
            black_box(unescape(text)).unwrap();
        })
    });

    group.finish();
}

/// Benchmarks escaping text using XML rules
fn escaping(c: &mut Criterion) {
    let mut group = c.benchmark_group("escape_text");

    group.bench_function("no_chars_to_escape_long", |b| {
        b.iter(|| {
            black_box(escape(LOREM_IPSUM_TEXT));
        })
    });

    group.bench_function("no_chars_to_escape_short", |b| {
        b.iter(|| {
            black_box(escape("just bit of text"));
        })
    });

    group.bench_function("escaped_chars_short", |b| {
        b.iter(|| {
            black_box(escape("age > 72 && age < 21"));
            black_box(escape("\"what's that?\""));
        })
    });

    group.bench_function("escaped_chars_long", |b| {
        let lorem_ipsum_with_escape_chars =
"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt
ut labore et dolore magna aliqua. & Hac habitasse platea dictumst vestibulum rhoncus est pellentesque.
Risus ultricies tristique nulla aliquet enim tortor at. Fermentum odio eu feugiat pretium nibh ipsum.
Volutpat sed cras ornare arcu dui. Scelerisque fermentum dui faucibus in ornare quam. Arcu cursus
euismod quis< viverra nibh cras pulvinar mattis. Sed viverra tellus in hac habitasse platea. Quis
commodo odio aenean sed. Cursus in hac habitasse platea dictumst quisque sagittis purus.

Neque convallis >a cras semper auctor. Sit amet mauris commodo quis imperdiet massa. Ac ut consequat
semper viverra nam libero justo laoreet sit. 'Adipiscing' commodo elit at imperdiet dui accumsan.
Enim lobortis scelerisque fermentum dui faucibus in ornare. Natoque penatibus et magnis dis parturient
montes nascetur ridiculus mus. At lectus urna duis convallis convallis tellus id interdum. Libero
volutpat sed cras ornare arcu dui vivamus arcu. Cursus in hac habitasse platea dictumst quisque sagittis
purus. Consequat id porta nibh venenatis cras sed felis.";

        b.iter(|| {
            black_box(escape(lorem_ipsum_with_escape_chars));
        })
    });
    group.finish();
}

/// Benchmarks unescaping text encoded using XML rules
fn unescaping(c: &mut Criterion) {
    let mut group = c.benchmark_group("unescape_text");

    group.bench_function("no_chars_to_unescape_long", |b| {
        b.iter(|| {
            black_box(unescape(LOREM_IPSUM_TEXT)).unwrap();
        })
    });

    group.bench_function("no_chars_to_unescape_short", |b| {
        b.iter(|| {
            black_box(unescape("just a bit of text")).unwrap();
        })
    });

    group.bench_function("char_reference", |b| {
        b.iter(|| {
            let text = "prefix &#34;some stuff&#34;,&#x22;more stuff&#x22;";
            black_box(unescape(text)).unwrap();
            let text = "&#38;&#60;";
            black_box(unescape(text)).unwrap();
        })
    });

    group.bench_function("entity_reference", |b| {
        b.iter(|| {
            let text = "age &gt; 72 &amp;&amp; age &lt; 21";
            black_box(unescape(text)).unwrap();
            let text = "&quot;what&apos;s that?&quot;";
            black_box(unescape(text)).unwrap();
        })
    });

    group.bench_function("mixed", |b| {
        let text =
"Lorem ipsum dolor sit amet, &amp;consectetur adipiscing elit, sed do eiusmod tempor incididunt
ut labore et dolore magna aliqua. Hac habitasse platea dictumst vestibulum rhoncus est pellentesque.
Risus ultricies &quot;tristique nulla aliquet enim tortor&quot; at. Fermentum odio eu feugiat pretium
nibh ipsum. Volutpat sed cras ornare arcu dui. Scelerisque fermentum dui faucibus in ornare quam. Arcu
cursus euismod quis &#60;viverra nibh cras pulvinar mattis. Sed viverra tellus in hac habitasse platea.
Quis commodo odio aenean sed. Cursus in hac habitasse platea dictumst quisque sagittis purus.

Neque convallis a cras semper auctor. Sit amet mauris commodo quis imperdiet massa. Ac ut consequat
semper viverra nam libero justo &#35; laoreet sit. Adipiscing commodo elit at imperdiet dui accumsan.
Enim lobortis scelerisque fermentum dui faucibus in ornare. Natoque penatibus et magnis dis parturient
montes nascetur ridiculus mus. At lectus urna &#33;duis convallis convallis tellus id interdum. Libero
volutpat sed cras ornare arcu dui vivamus arcu. Cursus in hac habitasse platea dictumst quisque sagittis
purus. Consequat id porta nibh venenatis cras sed felis.";

        b.iter(|| {
            black_box(unescape(text)).unwrap();
        })
    });
    group.finish();
}

criterion_group!(
    benches,
    read_event,
    read_resolved_event_into,
    one_event,
    attributes,
    attribute_value_normalization,
    escaping,
    unescaping,
);
criterion_main!(benches);

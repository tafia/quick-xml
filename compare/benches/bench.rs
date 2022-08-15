use criterion::{self, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use pretty_assertions::assert_eq;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use serde::Deserialize;
use serde_xml_rs;
use xml::reader::{EventReader, XmlEvent};

static RPM_PRIMARY: &str = include_str!("../../tests/documents/rpm_primary.xml");
static RPM_PRIMARY2: &str = include_str!("../../tests/documents/rpm_primary2.xml");
static RPM_FILELISTS: &str = include_str!("../../tests/documents/rpm_filelists.xml");
static RPM_OTHER: &str = include_str!("../../tests/documents/rpm_other.xml");
static LIBREOFFICE_DOCUMENT: &str = include_str!("../../tests/documents/libreoffice_document.fodt");
static DOCUMENT: &str = include_str!("../../tests/documents/document.xml");
static TEST_WRITER_INDENT: &str = include_str!("../../tests/documents/test_writer_indent.xml");
static SAMPLE_1: &str = include_str!("../../tests/documents/sample_1.xml");
static LINESCORE: &str = include_str!("../../tests/documents/linescore.xml");
static SAMPLE_RSS: &str = include_str!("../../tests/documents/sample_rss.xml");
static SAMPLE_NS: &str = include_str!("../../tests/documents/sample_ns.xml");
static PLAYERS: &str = include_str!("../../tests/documents/players.xml");

static TEST_FILES: [(&str, &str, usize); 12] = [
    // long, mix of attributes and text, not much escaping, mix of attribute lengths, some namespaces
    ("rpm_primary.xml", RPM_PRIMARY, 369),
    // long, mix of attributes and text, not much escaping, mix of attribute lengths, some namespaces
    ("rpm_primary2.xml", RPM_PRIMARY2, 116),
    // long, mostly medium-length text elements, not much escaping
    ("rpm_filelists.xml", RPM_FILELISTS, 184),
    // long, mix of attributes and text, lots of escaping (both entity and char literal), long attributes
    ("rpm_other.xml", RPM_OTHER, 145),
    // long, mix of attributes and text, not much escaping, lots of non-ascii characters, lots of namespaces
    ("libreoffice_document.fodt", LIBREOFFICE_DOCUMENT, 659),
    // medium length, mostly empty tags, a few short attributes per element, no escaping
    ("document.xml", DOCUMENT, 342),
    // medium length, lots of namespaces, no escaping
    ("test_writer_ident.xml", TEST_WRITER_INDENT, 34),
    // short, mix of attributes and text, lots of escapes
    ("sample_1.xml", SAMPLE_1, 15),
    // medium length, lots of attributes, short attributes, few escapes
    ("linescore.xml", LINESCORE, 11),
    // short, lots of namespaces, no escapes
    ("sample_ns.xml", SAMPLE_NS, 11),
    // long, few attributes, mix of attribute lengths, escapes in text content
    ("sample_rss.xml", SAMPLE_RSS, 1550),
    // long, lots of attributes, short attributes, no text, no escapes
    ("players.xml", PLAYERS, 76),
];

// Comparison of low-level APIs from several XML libraries
fn low_level_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("low-level API");
    for (filename, data, total_tags) in TEST_FILES.iter() {
        let total_tags = *total_tags;

        group.throughput(Throughput::Bytes(data.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("quick_xml", filename),
            *data,
            |b, input| {
                b.iter(|| {
                    let mut r = Reader::from_reader(input.as_bytes());
                    r.check_end_names(false).check_comments(false);
                    let mut count = criterion::black_box(0);
                    let mut buf = Vec::new();
                    loop {
                        match r.read_event_into(&mut buf) {
                            Ok(Event::Start(_)) | Ok(Event::Empty(_)) => count += 1,
                            Ok(Event::Eof) => break,
                            _ => (),
                        }
                        buf.clear();
                    }
                    assert_eq!(count, total_tags, "Overall tag count in {}", filename);
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("maybe_xml", filename),
            *data,
            |b, input| {
                use maybe_xml::eval::recv::RecvEvaluator;
                use maybe_xml::token::borrowed::Token;

                b.iter(|| {
                    let mut input = input.as_bytes();
                    let mut eval = RecvEvaluator::new();

                    let mut count = criterion::black_box(0);
                    loop {
                        let consumed = eval.recv(input);
                        match eval.next_token() {
                            Ok(Some(Token::StartTag(_))) => count += 1,
                            Ok(Some(Token::EmptyElementTag(_))) => count += 1,
                            Ok(Some(Token::Eof)) => break,
                            Ok(Some(Token::EofWithBytesNotEvaluated(_))) => break,
                            _ => (),
                        }
                        input = &input[consumed..];
                    }
                    assert_eq!(count, total_tags, "Overall tag count in {}", filename);
                })
            },
        );

        // DISABLED: fails to parse empty attributes
        // group.bench_with_input(BenchmarkId::new("rapid_xml", filename), *data, |b, input| {
        //     use rapid_xml::parser::{EventCode, Parser};

        //     b.iter(|| {
        //         let mut r = Parser::new(input.as_bytes());

        //         let mut count = criterion::black_box(0);
        //         loop {
        //             // Makes no progress if error is returned, so need unwrap()
        //             match r.next().unwrap().code() {
        //                 EventCode::StartTag => count += 1,
        //                 EventCode::Eof => break,
        //                 _ => (),
        //             }
        //         }
        //         assert_eq!(
        //             count, total_tags,
        //             "Overall tag count in {}", filename
        //         );
        //     })
        // });

        group.bench_with_input(
            BenchmarkId::new("xmlparser", filename),
            *data,
            |b, input| {
                use xmlparser::{Token, Tokenizer};

                b.iter(|| {
                    let mut count = criterion::black_box(0);
                    for token in Tokenizer::from(input) {
                        match token {
                            Ok(Token::ElementStart { .. }) => count += 1,
                            _ => (),
                        }
                    }
                    assert_eq!(count, total_tags, "Overall tag count in {}", filename);
                })
            },
        );

        group.bench_with_input(BenchmarkId::new("RustyXml", filename), *data, |b, input| {
            use rusty_xml::{Event, Parser};

            b.iter(|| {
                let mut r = Parser::new();
                r.feed_str(input);

                let mut count = criterion::black_box(0);
                for event in r {
                    match event.unwrap() {
                        Event::ElementStart(_) => count += 1,
                        _ => (),
                    }
                }
                assert_eq!(count, total_tags, "Overall tag count in {}", filename);
            })
        });

        group.bench_with_input(
            BenchmarkId::new("xml_oxide", filename),
            *data,
            |b, input| {
                use xml_oxide::sax::parser::Parser;
                use xml_oxide::sax::Event;

                b.iter(|| {
                    let mut r = Parser::from_reader(input.as_bytes());

                    let mut count = criterion::black_box(0);
                    loop {
                        // Makes no progress if error is returned, so need unwrap()
                        match r.read_event().unwrap() {
                            Event::StartElement(_) => count += 1,
                            Event::EndDocument => break,
                            _ => (),
                        }
                    }
                    assert_eq!(count, total_tags, "Overall tag count in {}", filename);
                })
            },
        );

        group.bench_with_input(BenchmarkId::new("xml5ever", filename), *data, |b, input| {
            use xml5ever::buffer_queue::BufferQueue;
            use xml5ever::tokenizer::{TagKind, Token, TokenSink, XmlTokenizer};

            struct Sink(usize);
            impl TokenSink for Sink {
                fn process_token(&mut self, token: Token) {
                    match token {
                        Token::TagToken(tag) if tag.kind == TagKind::StartTag => self.0 += 1,
                        Token::TagToken(tag) if tag.kind == TagKind::EmptyTag => self.0 += 1,
                        _ => (),
                    }
                }
            }

            // Copied from xml5ever benchmarks
            // https://github.com/servo/html5ever/blob/429f23943b24f739b78f4d703620d7b1b526475b/xml5ever/benches/xml5ever.rs
            b.iter(|| {
                let sink = criterion::black_box(Sink(0));
                let mut tok = XmlTokenizer::new(sink, Default::default());
                let mut buffer = BufferQueue::new();
                buffer.push_back(input.into());
                let _ = tok.feed(&mut buffer);
                tok.end();

                assert_eq!(tok.sink.0, total_tags, "Overall tag count in {}", filename);
            })
        });

        group.bench_with_input(BenchmarkId::new("xml_rs", filename), *data, |b, input| {
            b.iter(|| {
                let r = EventReader::new(input.as_bytes());
                let mut count = criterion::black_box(0);
                for e in r {
                    if let Ok(XmlEvent::StartElement { .. }) = e {
                        count += 1;
                    }
                }
                assert_eq!(count, total_tags, "Overall tag count in {}", filename);
            })
        });
    }

    group.finish();
}

/// Runs benchmarks for several XML libraries using serde deserialization
#[allow(dead_code)] // We do not use structs
fn serde_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("serde");

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

    group.throughput(Throughput::Bytes(SAMPLE_RSS.len() as u64));

    group.bench_with_input(
        BenchmarkId::new("quick_xml", "sample_rss.xml"),
        SAMPLE_RSS,
        |b, input| {
            b.iter(|| {
                let rss: Rss = criterion::black_box(quick_xml::de::from_str(input).unwrap());
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
            let rss = criterion::black_box(Rss::deserialize(&mut de).unwrap());
            assert_eq!(rss.channel.items.len(), 99);
        });
    });*/

    group.bench_with_input(
        BenchmarkId::new("xml_rs", "sample_rss.xml"),
        SAMPLE_RSS,
        |b, input| {
            b.iter(|| {
                let rss: Rss = criterion::black_box(serde_xml_rs::from_str(input).unwrap());
                assert_eq!(rss.channel.items.len(), 99);
            })
        },
    );

    group.finish();
}

criterion_group!(benches, low_level_comparison, serde_comparison);
criterion_main!(benches);

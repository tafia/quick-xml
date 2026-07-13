use criterion::{self, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use pretty_assertions::assert_eq;
use quick_xml::events::Event;
use quick_xml::reader::{self, Reader, XmlReader};
use std::hint::black_box;
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
    ("test_writer_indent.xml", TEST_WRITER_INDENT, 34),
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
            BenchmarkId::new("quick_xml:borrowed", filename),
            *data,
            |b, input| {
                b.iter(|| {
                    let mut reader = Reader::from_str(input);
                    reader.config_mut().check_end_names = false;
                    let mut count = black_box(0);
                    loop {
                        match reader.read_event() {
                            Ok(Event::Start(_)) | Ok(Event::Empty(_)) => count += 1,
                            Ok(Event::Eof) => break,
                            _ => (),
                        }
                    }
                    assert_eq!(count, total_tags, "Overall tag count in {}", filename);
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("quick_xml:buffered", filename),
            *data,
            |b, input| {
                b.iter(|| {
                    let mut reader = Reader::from_reader(input.as_bytes());
                    reader.config_mut().check_end_names = false;
                    let mut count = black_box(0);
                    let mut buf = Vec::new();
                    loop {
                        match reader.read_event_into(&mut buf) {
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
            BenchmarkId::new("quick_xml:reader", filename),
            *data,
            |b, input| {
                b.iter(|| {
                    let mut reader = XmlReader::from_str(input);
                    // TODO: reader.config_mut().check_end_names = false;
                    let mut count = black_box(0);
                    loop {
                        match reader.read_event() {
                            Ok(reader::Event::Start(_)) | Ok(reader::Event::Empty(_)) => count += 1,
                            Ok(reader::Event::Eof) => break,
                            _ => (),
                        }
                    }
                    assert_eq!(count, total_tags, "Overall tag count in {}", filename);
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("maybe_xml:0.10", filename),
            *data,
            |b, input| {
                use maybe_xml_0_10::token::Ty;
                use maybe_xml_0_10::Reader;

                b.iter(|| {
                    let reader = Reader::from_str(input);

                    let mut count = black_box(0);
                    for token in reader.into_iter() {
                        match token.ty() {
                            Ty::StartTag(_) | Ty::EmptyElementTag(_) => count += 1,
                            _ => (),
                        }
                    }
                    assert_eq!(count, total_tags, "Overall tag count in {}", filename);
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("maybe_xml:0.11", filename),
            *data,
            |b, input| {
                use maybe_xml::token::Ty;
                use maybe_xml::Reader;

                b.iter(|| {
                    let reader = Reader::from_str(input);

                    let mut count = black_box(0);
                    for token in reader.into_iter() {
                        match token.ty() {
                            Ty::StartTag(_) | Ty::EmptyElementTag(_) => count += 1,
                            _ => (),
                        }
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

        //         let mut count = black_box(0);
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
                    let mut count = black_box(0);
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

        // rxml does not support parsing comments yet, so disable those tests for it
        // https://codeberg.org/jssfr/rxml/issues/24
        if data.as_ptr() != TEST_WRITER_INDENT.as_ptr()
            && data.as_ptr() != PLAYERS.as_ptr()
            && data.as_ptr() != LINESCORE.as_ptr()
            // contains processing instructions, which are not supported by rxml
            && data.as_ptr() != SAMPLE_RSS.as_ptr()
        {
            group.bench_with_input(
                BenchmarkId::new("rxml:borrowed", filename),
                *data,
                |b, input| {
                    use rxml::{Event, Parse, Parser};

                    b.iter(|| {
                        let mut doc = input.as_bytes();
                        let mut p = Parser::new();
                        let mut count = black_box(0);
                        while doc.len() > 0 {
                            // true = doc contains the entire document
                            match p.parse(&mut doc, true).unwrap() {
                                Some(Event::StartElement(..)) => count += 1,
                                None => break,
                                _ => (),
                            }
                        }
                        assert_eq!(count, total_tags, "Overall tag count in {}", filename);
                    })
                },
            );

            group.bench_with_input(
                BenchmarkId::new("rxml:buffered", filename),
                *data,
                |b, input| {
                    use rxml::{as_eof_flag, Event, Reader};
                    use std::io::BufReader;

                    b.iter(|| {
                        let reader = BufReader::new(input.as_bytes());
                        let mut reader = Reader::new(reader);
                        let mut count = black_box(0);
                        let result = as_eof_flag(reader.read_all(|event| match event {
                            Event::StartElement(..) => count += 1,
                            _ => (),
                        }));
                        assert_eq!(result.unwrap(), true); // true indicates eof
                        assert_eq!(count, total_tags, "Overall tag count in {}", filename);
                    })
                },
            );
        }

        group.bench_with_input(BenchmarkId::new("RustyXml", filename), *data, |b, input| {
            use rusty_xml::{Event, Parser};

            b.iter(|| {
                let mut r = Parser::new();
                r.feed_str(input);

                let mut count = black_box(0);
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

                    let mut count = black_box(0);
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
            use markup5ever::buffer_queue::BufferQueue;
            use std::cell::Cell;
            use xml5ever::tokenizer::{ProcessResult, TagKind, Token, TokenSink, XmlTokenizer};

            struct Sink(Cell<usize>);
            impl TokenSink for Sink {
                type Handle = ();

                fn process_token(&self, token: Token) -> ProcessResult<Self::Handle> {
                    match token {
                        Token::Tag(tag) if tag.kind == TagKind::StartTag => {
                            self.0.set(self.0.get() + 1);
                        }
                        Token::Tag(tag) if tag.kind == TagKind::EmptyTag => {
                            self.0.set(self.0.get() + 1);
                        }
                        _ => (),
                    }
                    ProcessResult::Continue
                }
            }

            // Copied from xml5ever benchmarks
            // https://github.com/servo/html5ever/blob/a7c9d989b9b3426288a4ed362fb4c4671b2dd8c2/xml5ever/benches/xml5ever.rs#L57-L68
            b.iter(|| {
                let sink = black_box(Sink(Cell::new(0)));
                let tok = XmlTokenizer::new(sink, Default::default());
                let buffer = BufferQueue::default();
                buffer.push_back(input.into());
                let _ = tok.feed(&buffer);
                tok.end();

                assert_eq!(
                    tok.sink.0.into_inner(),
                    total_tags,
                    "Overall tag count in {}",
                    filename
                );
            })
        });

        group.bench_with_input(BenchmarkId::new("xml_rs", filename), *data, |b, input| {
            b.iter(|| {
                let r = EventReader::new(input.as_bytes());
                let mut count = black_box(0);
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

criterion_group!(benches, low_level_comparison);
criterion_main!(benches);

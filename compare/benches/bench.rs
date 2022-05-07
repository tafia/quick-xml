use criterion::{self, criterion_group, criterion_main, Criterion};
use pretty_assertions::assert_eq;
use quick_xml::{self, events::Event, Reader};
use serde::Deserialize;
use serde_xml_rs;
use xml::reader::{EventReader, XmlEvent};

static SOURCE: &str = include_str!("../../tests/sample_rss.xml");

/// Runs benchmarks for several XML libraries using low-level API
fn low_level_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("low-level API");

    group.bench_function("quick_xml", |b| {
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
            assert_eq!(count, 1550, "Overall tag count in ./tests/sample_rss.xml");
        })
    });

    group.bench_function("xmlparser", |b| {
        use xmlparser::{Token, Tokenizer};

        b.iter(|| {
            let mut count = criterion::black_box(0);
            for token in Tokenizer::from(SOURCE) {
                match token {
                    Ok(Token::ElementStart { .. }) => count += 1,
                    _ => (),
                }
            }
            assert_eq!(count, 1550, "Overall tag count in ./tests/sample_rss.xml");
        })
    });

    group.bench_function("xml5ever", |b| {
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
            buffer.push_back(SOURCE.into());
            let _ = tok.feed(&mut buffer);
            tok.end();

            assert_eq!(
                tok.sink.0, 1550,
                "Overall tag count in ./tests/sample_rss.xml"
            );
        })
    });

    group.bench_function("xml_rs", |b| {
        b.iter(|| {
            let r = EventReader::new(SOURCE.as_bytes());
            let mut count = criterion::black_box(0);
            for e in r {
                if let Ok(XmlEvent::StartElement { .. }) = e {
                    count += 1;
                }
            }
            assert_eq!(count, 1550, "Overall tag count in ./tests/sample_rss.xml");
        })
    });
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

    group.bench_function("quick_xml", |b| {
        b.iter(|| {
            let rss: Rss = quick_xml::de::from_str(SOURCE).unwrap();
            assert_eq!(rss.channel.items.len(), 99);
        })
    });

    group.bench_function("xml_rs", |b| {
        b.iter(|| {
            let rss: Rss = serde_xml_rs::from_str(SOURCE).unwrap();
            assert_eq!(rss.channel.items.len(), 99);
        });
    });
    group.finish();
}

criterion_group!(benches, low_level_comparison, serde_comparison);
criterion_main!(benches);

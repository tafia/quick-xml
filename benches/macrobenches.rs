use criterion::{self, criterion_group, criterion_main, Criterion, Throughput};
use quick_xml::events::Event;
use quick_xml::reader::{NsReader, Reader};
use quick_xml::Result as XmlResult;

static RPM_PRIMARY: &str = include_str!("../tests/documents/rpm_primary.xml");
static RPM_PRIMARY2: &str = include_str!("../tests/documents/rpm_primary2.xml");
static RPM_FILELISTS: &str = include_str!("../tests/documents/rpm_filelists.xml");
static RPM_OTHER: &str = include_str!("../tests/documents/rpm_other.xml");
static LIBREOFFICE_DOCUMENT: &str = include_str!("../tests/documents/libreoffice_document.fodt");
static DOCUMENT: &str = include_str!("../tests/documents/document.xml");
static TEST_WRITER_INDENT: &str = include_str!("../tests/documents/test_writer_indent.xml");
static SAMPLE_1: &str = include_str!("../tests/documents/sample_1.xml");
static LINESCORE: &str = include_str!("../tests/documents/linescore.xml");
static SAMPLE_RSS: &str = include_str!("../tests/documents/sample_rss.xml");
static SAMPLE_NS: &str = include_str!("../tests/documents/sample_ns.xml");
static PLAYERS: &str = include_str!("../tests/documents/players.xml");

static INPUTS: &[(&str, &str)] = &[
    // long, mix of attributes and text, not much escaping, mix of attribute lengths, some namespaces
    ("rpm_primary.xml", RPM_PRIMARY),
    // long, mix of attributes and text, not much escaping, mix of attribute lengths, some namespaces
    ("rpm_primary2.xml", RPM_PRIMARY2),
    // long, mostly medium-length text elements, not much escaping
    ("rpm_filelists.xml", RPM_FILELISTS),
    // long, mix of attributes and text, lots of escaping (both entity and char literal), long attributes
    ("rpm_other.xml", RPM_OTHER),
    // long, mix of attributes and text, not much escaping, lots of non-ascii characters, lots of namespaces
    ("libreoffice_document.fodt", LIBREOFFICE_DOCUMENT),
    // medium length, mostly empty tags, a few short attributes per element, no escaping
    ("document.xml", DOCUMENT),
    // medium length, lots of namespaces, no escaping
    ("test_writer_ident.xml", TEST_WRITER_INDENT),
    // short, mix of attributes and text, lots of escapes
    ("sample_1.xml", SAMPLE_1),
    // medium length, lots of attributes, short attributes, few escapes
    ("linescore.xml", LINESCORE),
    // short, lots of namespaces, no escapes
    ("sample_ns.xml", SAMPLE_NS),
    // long, few attributes, mix of attribute lengths, escapes in text content
    ("sample_rss.xml", SAMPLE_RSS),
    // long, lots of attributes, short attributes, no text, no escapes
    ("players.xml", PLAYERS),
];

// TODO: use fully normalized attribute values
fn parse_document_from_str(doc: &str) -> XmlResult<()> {
    let mut r = Reader::from_str(doc);
    loop {
        match criterion::black_box(r.read_event()?) {
            Event::Start(e) | Event::Empty(e) => {
                for attr in e.attributes() {
                    criterion::black_box(attr?.decode_and_unescape_value(r.decoder())?);
                }
            }
            Event::Text(e) => {
                criterion::black_box(e.decode()?);
            }
            Event::CData(e) => {
                criterion::black_box(e.into_inner());
            }
            Event::End(_) => (),
            Event::Eof => break,
            _ => (),
        }
    }
    Ok(())
}

// TODO: use fully normalized attribute values
fn parse_document_from_bytes(doc: &[u8]) -> XmlResult<()> {
    let mut r = Reader::from_reader(doc);
    let mut buf = Vec::new();
    loop {
        match criterion::black_box(r.read_event_into(&mut buf)?) {
            Event::Start(e) | Event::Empty(e) => {
                for attr in e.attributes() {
                    criterion::black_box(attr?.decode_and_unescape_value(r.decoder())?);
                }
            }
            Event::Text(e) => {
                criterion::black_box(e.decode()?);
            }
            Event::CData(e) => {
                criterion::black_box(e.into_inner());
            }
            Event::End(_) => (),
            Event::Eof => break,
            _ => (),
        }
        buf.clear();
    }
    Ok(())
}

// TODO: use fully normalized attribute values
fn parse_document_from_str_with_namespaces(doc: &str) -> XmlResult<()> {
    let mut r = NsReader::from_str(doc);
    loop {
        match criterion::black_box(r.read_resolved_event()?) {
            (resolved_ns, Event::Start(e) | Event::Empty(e)) => {
                criterion::black_box(resolved_ns);
                for attr in e.attributes() {
                    criterion::black_box(attr?.decode_and_unescape_value(r.decoder())?);
                }
            }
            (resolved_ns, Event::Text(e)) => {
                criterion::black_box(e.decode()?);
                criterion::black_box(resolved_ns);
            }
            (resolved_ns, Event::CData(e)) => {
                criterion::black_box(e.into_inner());
                criterion::black_box(resolved_ns);
            }
            (_, Event::End(_)) => (),
            (_, Event::Eof) => break,
            _ => (),
        }
    }
    Ok(())
}

// TODO: use fully normalized attribute values
fn parse_document_from_bytes_with_namespaces(doc: &[u8]) -> XmlResult<()> {
    let mut r = NsReader::from_reader(doc);
    let mut buf = Vec::new();
    loop {
        match criterion::black_box(r.read_resolved_event_into(&mut buf)?) {
            (resolved_ns, Event::Start(e) | Event::Empty(e)) => {
                criterion::black_box(resolved_ns);
                for attr in e.attributes() {
                    criterion::black_box(attr?.decode_and_unescape_value(r.decoder())?);
                }
            }
            (resolved_ns, Event::Text(e)) => {
                criterion::black_box(e.decode()?);
                criterion::black_box(resolved_ns);
            }
            (resolved_ns, Event::CData(e)) => {
                criterion::black_box(e.into_inner());
                criterion::black_box(resolved_ns);
            }
            (_, Event::End(_)) => (),
            (_, Event::Eof) => break,
            _ => (),
        }
        buf.clear();
    }
    Ok(())
}

/// Just parse - no decoding overhead
pub fn bench_parse_document_nocopy(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_document_nocopy");

    for (id, data) in INPUTS.iter() {
        group.throughput(Throughput::Bytes(data.len() as u64));
        group.bench_with_input(*id, *data, |b, input| {
            b.iter(|| parse_document_from_str(input).unwrap())
        });
    }

    group.finish();
}

/// Decode into a buffer, then parse
pub fn bench_decode_and_parse_document(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_and_parse_document");

    for (id, data) in INPUTS.iter() {
        group.throughput(Throughput::Bytes(data.len() as u64));
        group.bench_with_input(*id, *data, |b, input| {
            b.iter(|| parse_document_from_bytes(input.as_bytes()).unwrap())
        });
    }

    group.finish();
}

/// Just parse - no decoding overhead - including namespaces
pub fn bench_parse_document_nocopy_with_namespaces(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_document_nocopy_with_namespaces");

    for (id, data) in INPUTS.iter() {
        group.throughput(Throughput::Bytes(data.len() as u64));
        group.bench_with_input(*id, *data, |b, input| {
            b.iter(|| parse_document_from_str_with_namespaces(input).unwrap())
        });
    }

    group.finish();
}

/// Decode into a buffer, then parse - including namespaces
pub fn bench_decode_and_parse_document_with_namespaces(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_and_parse_document_with_namespaces");

    for (id, data) in INPUTS.iter() {
        group.throughput(Throughput::Bytes(data.len() as u64));
        group.bench_with_input(*id, *data, |b, input| {
            b.iter(|| parse_document_from_bytes_with_namespaces(input.as_bytes()).unwrap())
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_parse_document_nocopy,
    bench_decode_and_parse_document,
    bench_parse_document_nocopy_with_namespaces,
    bench_decode_and_parse_document_with_namespaces,
);
criterion_main!(benches);

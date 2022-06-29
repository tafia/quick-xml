use criterion::{self, criterion_group, Criterion};
use quick_xml::events::Event;
use quick_xml::Reader;
use quick_xml::Result as XmlResult;

static RPM_PRIMARY: &[u8] = include_bytes!("../tests/documents/rpm_primary.xml");
static RPM_PRIMARY2: &[u8] = include_bytes!("../tests/documents/rpm_primary2.xml");
static RPM_FILELISTS: &[u8] = include_bytes!("../tests/documents/rpm_filelists.xml");
static RPM_OTHER: &[u8] = include_bytes!("../tests/documents/rpm_other.xml");
static LIBREOFFICE_DOCUMENT: &[u8] = include_bytes!("../tests/documents/libreoffice_document.fodt");
static DOCUMENT: &[u8] = include_bytes!("../tests/documents/document.xml");
static TEST_WRITER_INDENT: &[u8] = include_bytes!("../tests/documents/test_writer_indent.xml");
static SAMPLE_1: &[u8] = include_bytes!("../tests/documents/sample_1.xml");
static LINESCORE: &[u8] = include_bytes!("../tests/documents/linescore.xml");
static SAMPLE_RSS: &[u8] = include_bytes!("../tests/documents/sample_rss.xml");
static SAMPLE_NS: &[u8] = include_bytes!("../tests/documents/sample_ns.xml");
static PLAYERS: &[u8] = include_bytes!("../tests/documents/players.xml");

// TODO: read the namespaces too
// TODO: use fully normalized attribute values
fn parse_document(doc: &[u8]) -> XmlResult<()> {
    let mut r = Reader::from_reader(doc);
    loop {
        match r.read_event_unbuffered()? {
            Event::Start(e) | Event::Empty(e) => {
                for attr in e.attributes() {
                    criterion::black_box(attr?.unescaped_value()?);
                }
            },
            Event::Text(e) => {
                criterion::black_box(e.unescaped()?);
            },
            Event::CData(e) => {
                criterion::black_box(e.into_inner());
            },
            Event::End(_) => (),
            Event::Eof => break,
            _ => (),
        }
    }
    Ok(())
}

pub fn bench_fully_parse_document(c: &mut Criterion) {
    let mut group = c.benchmark_group("fully_parse_document");

    // long, mix of attributes and text, not much escaping, mix of attribute lengths, some namespaces
    group.bench_function("rpm_primary.xml", |b| {
        b.iter(|| {
            parse_document(RPM_PRIMARY).unwrap();
        })
    });

    // long, mix of attributes and text, not much escaping, mix of attribute lengths, some namespaces
    group.bench_function("rpm_primary2.xml", |b| {
        b.iter(|| {
            parse_document(RPM_PRIMARY2).unwrap();
        })
    });

    // long, mostly medium-length text elements, not much escaping
    group.bench_function("rpm_filelists.xml", |b| {
        b.iter(|| {
            parse_document(RPM_FILELISTS).unwrap();
        })
    });

    // long, mix of attributes and text, lots of escaping (both entity and char literal), long attributes
    group.bench_function("rpm_other.xml", |b| {
        b.iter(|| {
            parse_document(RPM_OTHER).unwrap();
        })
    });

    // long, mix of attributes and text, not much escaping, lots of non-ascii characters, lots of namespaces
    group.bench_function("libreoffice_document.fodt", |b| {
        b.iter(|| {
            parse_document(LIBREOFFICE_DOCUMENT).unwrap();
        })
    });

    // medium length, mostly empty tags, a few short attributes per element, no escaping
    group.bench_function("document.xml", |b| {
        b.iter(|| {
            parse_document(DOCUMENT).unwrap();
        })
    });

    // medium length, lots of namespaces, no escaping
    group.bench_function("test_writer_ident.xml", |b| {
        b.iter(|| {
            parse_document(TEST_WRITER_INDENT).unwrap();
        })
    });

    // short, mix of attributes and text, lots of escapes
    group.bench_function("sample_1.xml", |b| {
        b.iter(|| {
            parse_document(SAMPLE_1).unwrap();
        })
    });

    // medium length, lots of attributes, short attributes, few escapes
    group.bench_function("linescore.xml", |b| {
        b.iter(|| {
            parse_document(LINESCORE).unwrap();
        })
    });

    // short, lots of namespaces, no escapes
    group.bench_function("sample_ns.xml", |b| {
        b.iter(|| {
            parse_document(SAMPLE_NS).unwrap();
        })
    });

    // long, few attributes, mix of attribute lengths, escapes in text content
    group.bench_function("sample_rss.xml", |b| {
        b.iter(|| {
            parse_document(SAMPLE_RSS).unwrap();
        })
    });

    // long, lots of attributes, short attributes, no text, no escapes
    group.bench_function("players.xml", |b| {
        b.iter(|| {
            parse_document(PLAYERS).unwrap();
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_fully_parse_document,
);

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use quick_xml::encoding::Utf8ValidatingReader;
use std::io::{BufReader, Read};

#[cfg(feature = "encoding")]
use quick_xml::encoding::DecodingReader;

static SAMPLE: &[u8] = include_bytes!("../tests/documents/sample_rss.xml");

/// Read the entire input through the reader using a fixed-size buffer,
/// returning the total number of bytes read.
fn drain_reader(reader: &mut impl Read, buf: &mut [u8]) -> usize {
    let mut total = 0;
    loop {
        match reader.read(buf) {
            Ok(0) => break,
            Ok(n) => total += n,
            Err(e) => panic!("unexpected error: {e}"),
        }
    }
    total
}

/// Encode a UTF-8 byte slice as UTF-16 LE with BOM.
#[cfg(feature = "encoding")]
fn to_utf16le_with_bom(utf8: &[u8]) -> Vec<u8> {
    let s = std::str::from_utf8(utf8).expect("SAMPLE must be valid UTF-8");
    let mut out = vec![0xFF, 0xFE]; // UTF-16 LE BOM
    for code_unit in s.encode_utf16() {
        out.extend_from_slice(&code_unit.to_le_bytes());
    }
    out
}

fn bench_utf8_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("utf8_read");
    group.throughput(Throughput::Bytes(SAMPLE.len() as u64));

    for buf_size in [64, 1024, 8192] {
        group.bench_with_input(
            BenchmarkId::new("BufReader_only", buf_size),
            &buf_size,
            |b, &buf_size| {
                b.iter(|| {
                    let mut reader = BufReader::new(SAMPLE);
                    let mut buf = vec![0u8; buf_size];
                    let n = drain_reader(&mut reader, &mut buf);
                    assert_eq!(n, SAMPLE.len());
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("Utf8ValidatingReader", buf_size),
            &buf_size,
            |b, &buf_size| {
                b.iter(|| {
                    let mut reader = Utf8ValidatingReader::new(BufReader::new(SAMPLE));
                    let mut buf = vec![0u8; buf_size];
                    let n = drain_reader(&mut reader, &mut buf);
                    assert_eq!(n, SAMPLE.len());
                });
            },
        );

        #[cfg(feature = "encoding")]
        group.bench_with_input(
            BenchmarkId::new("DecodingReader_utf8", buf_size),
            &buf_size,
            |b, &buf_size| {
                b.iter(|| {
                    let mut reader = DecodingReader::new(BufReader::new(SAMPLE));
                    let mut buf = vec![0u8; buf_size];
                    let n = drain_reader(&mut reader, &mut buf);
                    assert_eq!(n, SAMPLE.len());
                });
            },
        );
    }

    group.finish();
}

#[cfg(feature = "encoding")]
fn bench_utf16_decoding(c: &mut Criterion) {
    let utf16_sample = to_utf16le_with_bom(SAMPLE);

    let mut group = c.benchmark_group("utf16_read");
    group.throughput(Throughput::Bytes(utf16_sample.len() as u64));

    for buf_size in [64, 1024, 8192] {
        group.bench_with_input(
            BenchmarkId::new("DecodingReader_utf16le", buf_size),
            &buf_size,
            |b, &buf_size| {
                b.iter(|| {
                    let mut reader = DecodingReader::new(&utf16_sample[..]);
                    let mut buf = vec![0u8; buf_size];
                    drain_reader(&mut reader, &mut buf);
                });
            },
        );
    }

    group.finish();
}

#[cfg(feature = "encoding")]
criterion_group!(benches, bench_utf8_validation, bench_utf16_decoding);
#[cfg(not(feature = "encoding"))]
criterion_group!(benches, bench_utf8_validation);
criterion_main!(benches);

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use quick_xml::encoding::Utf8ValidatingReader;
use std::io::{BufReader, Read};

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
    }

    group.finish();
}

criterion_group!(benches, bench_utf8_validation);
criterion_main!(benches);

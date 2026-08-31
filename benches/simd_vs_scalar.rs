use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use memchr::{memchr, memchr2, memchr3};

fn bench_memchr(c: &mut Criterion) {
    let mut group = c.benchmark_group("memchr_vs_scalar");
    // sample from tests/documents/sample_rss.xml style: mixed XML text with < > & ' " \r
    let sample = include_bytes!("../tests/documents/sample_rss.xml");
    // Create larger buffer by repeating
    let mut large = Vec::with_capacity(sample.len() * 20);
    for _ in 0..20 { large.extend_from_slice(sample); }
    
    group.throughput(Throughput::Bytes(large.len() as u64));
    
    // Scalar baseline: iter.position with closure (old _escape logic)
    group.bench_function(BenchmarkId::new("scalar_position_6", large.len()), |b| {
        b.iter(|| {
            let mut pos = 0;
            let mut count = 0;
            let bytes = large.as_slice();
            let mut iter = bytes.iter();
            while let Some(i) = iter.position(|&b| matches!(b, b'<' | b'>' | b'&' | b'\'' | b'"' | b'\r')) {
                count += 1;
                pos += i + 1;
                if pos >= bytes.len() { break; }
                // reset iter to remaining slice for fair comparison (simulates _escape loop)
                iter = bytes[pos..].iter();
                if count > 1000 { break; }
            }
            std::hint::black_box(count)
        })
    });

    // SIMD memchr: 2x memchr3 (new escape impl)
    group.bench_function(BenchmarkId::new("memchr_simd_6", large.len()), |b| {
        b.iter(|| {
            let mut pos = 0;
            let mut count = 0;
            let bytes = large.as_slice();
            while pos < bytes.len() {
                let slice = &bytes[pos..];
                let a = memchr3(b'<', b'>', b'&', slice);
                let bpos = memchr3(b'\'', b'"', b'\r', slice);
                let next = match (a, bpos) {
                    (Some(x), Some(y)) => Some(x.min(y)),
                    (Some(x), None) => Some(x),
                    (None, Some(y)) => Some(y),
                    (None, None) => None,
                };
                if let Some(i) = next {
                    count += 1;
                    pos += i + 1;
                    if count > 1000 { break; }
                } else { break; }
            }
            std::hint::black_box(count)
        })
    });

    // Single char memchr (baseline)
    group.bench_function(BenchmarkId::new("memchr_single", large.len()), |b| {
        b.iter(|| {
            let mut pos = 0;
            let mut count = 0;
            while let Some(i) = memchr(b'<', &large[pos..]) {
                count += 1;
                pos += i + 1;
                if pos >= large.len() || count > 1000 { break; }
            }
            std::hint::black_box(count)
        })
    });

    group.finish();
}

criterion_group!(benches, bench_memchr);
criterion_main!(benches);

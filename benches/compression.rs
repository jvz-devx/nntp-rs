//! Benchmarks for NNTP compression (RFC 8054)
//!
//! Tests performance of COMPRESS DEFLATE and XFEATURE COMPRESS GZIP

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use flate2::Compression;
use flate2::write::{DeflateEncoder, GzEncoder};
use std::io::Write;

/// Sample NNTP response data (typical LIST ACTIVE response with ~1000 newsgroups)
fn generate_sample_data(size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size);
    // Simulate typical NNTP response lines: "comp.lang.rust 12345 1 y\r\n"
    for i in 0..size / 40 {
        let line = format!("comp.lang.group{} {} {} y\r\n", i % 1000, i * 10, i);
        data.extend_from_slice(line.as_bytes());
    }
    data
}

fn bench_deflate_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("deflate_compression");

    for size in [1_024, 10_240, 102_400, 1_024_000].iter() {
        group.throughput(Throughput::Bytes(*size as u64));

        let data = generate_sample_data(*size);

        for level in [
            Compression::fast(),
            Compression::default(),
            Compression::best(),
        ]
        .iter()
        {
            group.bench_with_input(
                BenchmarkId::from_parameter(format!("{}KB_level{}", size / 1024, level.level())),
                size,
                |b, _| {
                    b.iter(|| {
                        let mut encoder = DeflateEncoder::new(Vec::new(), *level);
                        encoder.write_all(black_box(&data)).unwrap();
                        encoder.finish().unwrap()
                    });
                },
            );
        }
    }
    group.finish();
}

fn bench_gzip_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("gzip_compression");

    for size in [1_024, 10_240, 102_400, 1_024_000].iter() {
        group.throughput(Throughput::Bytes(*size as u64));

        let data = generate_sample_data(*size);

        for level in [
            Compression::fast(),
            Compression::default(),
            Compression::best(),
        ]
        .iter()
        {
            group.bench_with_input(
                BenchmarkId::from_parameter(format!("{}KB_level{}", size / 1024, level.level())),
                size,
                |b, _| {
                    b.iter(|| {
                        let mut encoder = GzEncoder::new(Vec::new(), *level);
                        encoder.write_all(black_box(&data)).unwrap();
                        encoder.finish().unwrap()
                    });
                },
            );
        }
    }
    group.finish();
}

criterion_group!(benches, bench_deflate_compression, bench_gzip_compression);
criterion_main!(benches);

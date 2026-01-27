//! Benchmarks for PAR2 file verification
//!
//! Tests performance of MD5 hashing and CRC32 checks used in PAR2 verification

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

/// Generate random-ish file data for benchmarking
fn generate_file_data(size: usize) -> Vec<u8> {
    // Simple deterministic "random" data
    (0..size)
        .map(|i| ((i * 1103515245 + 12345) >> 16) as u8)
        .collect()
}

fn bench_md5_hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("par2_md5_hash");

    // PAR2 uses MD5 for file integrity verification
    // Test different file sizes (1MB to 100MB typical for Usenet files)
    for size in [1_048_576, 10_485_760, 52_428_800, 104_857_600].iter() {
        group.throughput(Throughput::Bytes(*size as u64));

        let data = generate_file_data(*size);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}MB", size / 1_048_576)),
            size,
            |b, _| {
                b.iter(|| {
                    use md5::{Digest, Md5};
                    let mut hasher = Md5::new();
                    hasher.update(black_box(&data));
                    hasher.finalize()
                });
            },
        );
    }

    group.finish();
}

fn bench_crc32_hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("par2_crc32_hash");

    // PAR2 uses CRC32 for packet integrity
    for size in [1_048_576, 10_485_760, 52_428_800, 104_857_600].iter() {
        group.throughput(Throughput::Bytes(*size as u64));

        let data = generate_file_data(*size);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}MB", size / 1_048_576)),
            size,
            |b, _| {
                b.iter(|| {
                    use crc32fast::Hasher;
                    let mut hasher = Hasher::new();
                    hasher.update(black_box(&data));
                    hasher.finalize()
                });
            },
        );
    }

    group.finish();
}

fn bench_par2_packet_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("par2_packet_validation");

    // Simulate PAR2 packet validation (MD5 hash check on packet)
    // Typical PAR2 packet sizes range from 64 bytes (header) to several KB
    for size in [64, 1_024, 10_240, 102_400].iter() {
        group.throughput(Throughput::Bytes(*size as u64));

        let data = generate_file_data(*size);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}B", size)),
            size,
            |b, _| {
                b.iter(|| {
                    use md5::{Digest, Md5};
                    // Validate packet hash (skip first 32 bytes per PAR2 spec)
                    let packet_data = if data.len() > 32 {
                        &data[32..]
                    } else {
                        &data[..]
                    };
                    let mut hasher = Md5::new();
                    hasher.update(black_box(packet_data));
                    hasher.finalize()
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_md5_hashing,
    bench_crc32_hashing,
    bench_par2_packet_validation
);
criterion_main!(benches);

//! Benchmarks for yEnc binary encoding/decoding
//!
//! Tests performance of yEnc decoding which is critical for Usenet binary downloads

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

/// Generate sample yEnc encoded data
///
/// yEnc encoding: byte_out = (byte_in + 42) % 256
/// Special escaping for =, \r, \n, \0 using =<escaped_byte>
fn generate_yenc_data(size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size + size / 100); // Account for escaping overhead
    let mut line_len = 0;
    const LINE_LENGTH: usize = 128;

    for i in 0..size {
        let byte = (i % 256) as u8;
        let encoded = byte.wrapping_add(42);

        // Escape special characters: =, \r, \n, \0
        if encoded == b'=' || encoded == b'\r' || encoded == b'\n' || encoded == 0 {
            data.push(b'=');
            data.push(encoded.wrapping_add(64));
            line_len += 2;
        } else {
            data.push(encoded);
            line_len += 1;
        }

        // Insert line breaks at ~128 chars (typical yEnc line length)
        if line_len >= LINE_LENGTH {
            data.extend_from_slice(b"\r\n");
            line_len = 0;
        }
    }

    data
}

/// Simple yEnc decoder for benchmarking
///
/// Production code is in src/yenc.rs - this is a simplified version
fn decode_yenc_simple(data: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(data.len());
    let mut escaped = false;

    for &byte in data {
        if byte == b'\r' || byte == b'\n' {
            continue;
        }

        if escaped {
            output.push(byte.wrapping_sub(64).wrapping_sub(42));
            escaped = false;
        } else if byte == b'=' {
            escaped = true;
        } else {
            output.push(byte.wrapping_sub(42));
        }
    }

    output
}

fn bench_yenc_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("yenc_decode");

    // Test different file sizes (1KB to 10MB)
    for size in [1_024, 10_240, 102_400, 1_024_000, 10_240_000].iter() {
        group.throughput(Throughput::Bytes(*size as u64));

        let data = generate_yenc_data(*size);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}KB", size / 1024)),
            size,
            |b, _| {
                b.iter(|| decode_yenc_simple(black_box(&data)));
            },
        );
    }

    group.finish();
}

fn bench_yenc_crc32(c: &mut Criterion) {
    let mut group = c.benchmark_group("yenc_crc32");

    // CRC32 calculation is part of yEnc verification
    for size in [1_024, 10_240, 102_400, 1_024_000, 10_240_000].iter() {
        group.throughput(Throughput::Bytes(*size as u64));

        let data = vec![0u8; *size];

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}KB", size / 1024)),
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

criterion_group!(benches, bench_yenc_decode, bench_yenc_crc32);
criterion_main!(benches);

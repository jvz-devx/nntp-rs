# Benchmarks

This directory contains [Criterion](https://github.com/bheisler/criterion.rs) benchmarks for performance-critical components of nntp-rs.

## Running Benchmarks

Run all benchmarks:
```bash
cargo bench
```

Run specific benchmark suite:
```bash
cargo bench --bench compression
cargo bench --bench yenc
cargo bench --bench par2
```

Run specific benchmark within a suite:
```bash
cargo bench --bench compression -- deflate
cargo bench --bench yenc -- crc32
cargo bench --bench par2 -- md5
```

## Benchmark Suites

### compression.rs
Tests NNTP compression performance (RFC 8054):
- **deflate_compression**: COMPRESS DEFLATE at 3 compression levels across 4 data sizes
- **gzip_compression**: XFEATURE COMPRESS GZIP at 3 compression levels across 4 data sizes

Measures throughput for compressing typical NNTP responses (LIST ACTIVE format).

### yenc.rs
Tests yEnc binary encoding/decoding performance:
- **yenc_decode**: Decoding performance for 1KB to 10MB files
- **yenc_crc32**: CRC32 verification performance

Simulates realistic yEnc data with proper escaping and line breaks.

### par2.rs
Tests PAR2 file verification performance:
- **par2_md5_hash**: MD5 hashing for 1MB to 100MB files
- **par2_crc32_hash**: CRC32 hashing for large files
- **par2_packet_validation**: Packet integrity checking

## Results

After running benchmarks, HTML reports are generated in `target/criterion/`:
```bash
# View results in browser
open target/criterion/report/index.html  # macOS
xdg-open target/criterion/report/index.html  # Linux
```

Criterion automatically compares against previous runs and shows performance changes.

## Performance Baselines

To save current performance as a baseline:
```bash
cargo bench -- --save-baseline my-baseline
```

To compare against a baseline:
```bash
cargo bench -- --baseline my-baseline
```

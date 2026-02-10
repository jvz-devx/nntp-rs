# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-02-10

### Added

- 64 MB size limit on compressed NNTP blocks (`MAX_COMPRESSED_BLOCK_SIZE`) to prevent OOM from malicious or broken servers
- PAR2 parser validates `packet.length >= header_size` before subtraction (prevents arithmetic underflow on malformed files)
- PAR2 parser rejects `slice_size == 0` in main packets (prevents division-by-zero)
- `BandwidthLimiter::new()` panics on `bytes_per_second == 0` (prevents division-by-zero; documented with `# Panics`)
- `LruHeaderCache` uses `wrapping_add` for access counter (explicit overflow behavior across debug/release)
- Live integration tests: `high_throughput`, `listing_extended`, and `pool` test modules
- Live integration `get_test_config()` supports fallback env var names (`NNTP_USERNAME`, `NNTP_PASSWORD`, `NNTP_PORT_SSL`)

### Changed

- **Breaking:** `YencDecoded::verify_crc32()` now returns `Option<bool>` instead of `bool` — `None` means no CRC available, `Some(false)` means corruption, `Some(true)` means verified
- `YencMultipartAssembler::add_part()` and `ArticleAssembler` now treat missing CRC as unverifiable instead of corrupted — only confirmed CRC mismatches (`Some(false)`) are rejected
- `NntpClient::maybe_decompress()` now returns `Result<Vec<u8>>` — decompression errors are propagated instead of silently returning raw compressed data

### Fixed

- Binary reader (`read_multiline_response_binary`) now strips NNTP line terminators (`\r\n`) before appending to payload — previously embedded protocol framing bytes into binary data
- PAR2 verification: slices with missing IFSC checksums are now correctly reported as damaged (previously silently skipped)
- PAR2 verification: when no IFSC packet exists and MD5 hashes fail, all slices are now marked damaged (previously reported 0 damaged slices)
- `ArticleAssembler` no longer clones decoded data unnecessarily for multipart files

## [0.2.1] - 2026-01-27

### Changed

- **Breaking (MSRV):** Rust edition upgraded from 2021 to 2024 (requires Rust 1.85+; MSRV remains 1.93)
- Upgraded `bb8` from 0.8 to 0.9 — uses native `async fn` in traits (RPITIT), eliminating `Pin<Box<dyn Future>>` heap allocations on every pool operation
- Upgraded `thiserror` from 1 to 2 — cleaner proc-macro implementation, identical API
- Replaced `parking_lot::Mutex` with `std::sync::Mutex` — removes a dependency; lock durations are trivially short (timestamps and round-robin indices)
- Replaced `#[allow(...)]` with `#[expect(...)]` (Rust 1.81) — warns when suppressed lints no longer fire, preventing stale suppressions
- Collapsed nested `if`/`if let` patterns into let-chains (Rust 1.87) where flagged by clippy
- Moved test-only `decode_line()` helper into `#[cfg(test)]` module

### Removed

- `async-trait` dependency — no longer needed with bb8 0.9's native async trait support
- `parking_lot` dependency — replaced by `std::sync::Mutex`
- Stale `#[allow(dead_code)]` / `#[expect(dead_code)]` attributes on 5 public items that edition 2024 no longer considers dead code
- Stale `#[expect(clippy::unwrap_used)]` / `#[expect(clippy::expect_used)]` attributes on 3 functions that no longer trigger those lints

### Fixed

- Set minimum versions for all dependencies to fix `-Z minimal-versions` CI check — prevents resolution to ancient versions that don't compile on modern Rust (e.g., `flate2 1.0.0` → `gcc 0.3.3`, `lazy_static 1.0.0` → missing API)

## [0.2.0] - 2025-01-27

### Added

- `GroupInfo` struct — `select_group()` now returns a struct with named fields (`count`, `first`, `last`) instead of a bare `(u64, u64, u64)` tuple
- `ArticleInfo` struct — `stat()`, `next()`, and `last()` now return a struct with named fields (`number`, `message_id`) instead of a bare `(u64, String)` tuple
- `ArticleInfo` and `GroupInfo` are re-exported from the crate root
- `#[must_use]` annotations on `NntpClient`, `NntpPool`, `NntpResponse`, `NntpBinaryResponse`, `Capabilities`, `ServerConfig`, and key accessor methods
- Named constants for I/O tuning — timeouts (`SINGLE_LINE_TIMEOUT`, `MULTILINE_TIMEOUT`), buffer sizes (`COMPRESSED_READ_BUFFER_SIZE`, `BINARY_DATA_INITIAL_CAPACITY`, `BUFREADER_CAPACITY`), and connection timeouts (`TCP_CONNECT_TIMEOUT_SECS`, `TLS_HANDSHAKE_TIMEOUT_SECS`)
- Expanded test coverage for compression, I/O (dot-stuffing, terminator detection, buffer pre-allocation), connection (timeout constants, certificate verifier, state transitions), listing commands, and PAR2 parsing

### Changed

- **Breaking:** `select_group()` returns `Result<GroupInfo>` instead of `Result<(u64, u64, u64)>`
- **Breaking:** `stat()`, `next()`, `last()` return `Result<ArticleInfo>` instead of `Result<(u64, String)>`
- **Breaking:** Response parsers (`parse_group_response`, `parse_stat_response`, `parse_hdr_response`, `parse_list_*_response`, etc.) now take `NntpResponse` by value instead of by reference
- **Breaking:** `SegmentFetchResult.segment` replaced with `segment_index: usize` to avoid cloning the full segment struct; `fetch_segment()` now takes an additional `segment_index` parameter
- **Breaking:** `FileDescriptionPacket.name` changed from `String` to `Arc<str>` for cheaper cloning during PAR2 verification
- Several command builders now return `&'static str` instead of `String` for zero-allocation commands (`compress_deflate()`, `quit()`, `capabilities()`, `help()`, `date()`, `mode_reader()`, etc.)
- `parse_response_line()` now rejects 4+ digit response codes per RFC 3977 Section 3.1
- Decompression and I/O buffers pre-allocated with estimated capacities instead of growing from empty
- `Article::serialize_for_posting()` uses `write!` macro with pre-allocated buffer instead of `format!()` + `push_str()`
- Doc examples updated to use `ServerConfig::plain()`/`ServerConfig::tls()` constructors; README code blocks annotated with `no_run`/`ignore`

### Removed

- `src/article_original.rs` — temporary module used during refactoring (2031 lines)
- Monolithic `src/commands.rs` replaced by `src/commands/` module directory with focused submodules (`article`, `group`, `hdr`, `list`, `over`, `response`); all public items re-exported for compatibility
- Monolithic `src/yenc.rs` replaced by `src/yenc/` module directory with focused submodules (`assembler`, `decode`, `encode`, `params`, `types`); all public items re-exported for compatibility
- Monolithic `tests/rfc3977/list.rs` split into `list_active.rs`, `list_active_times.rs`, `list_headers.rs`, `list_newsgroups.rs`, `list_overview.rs`
- Development phase markers removed from `Cargo.toml` dependency comments

### Fixed

- `parse_response_line()` no longer silently accepts malformed 4+ digit status codes (e.g., `"2000 OK"` was parsed as code 200)
- PAR2 parsing eliminates `unwrap()` on untrusted data — replaced with safe `read_u32_le()`/`read_u64_le()` helpers that return proper errors on truncated or malformed packets
- NZB parsing refactored: extracted `parse_meta_type()`, `parse_file_attributes()`, and `parse_segment_attributes()` helpers to reduce nesting and duplication
- Connection pool retry logging no longer uses `expect()` for the last-error reference
- Compressed block reading deduplicated via `read_compressed_block()` helper
- NNTP dot-stuffing removal extracted to `strip_byte_stuffing()` function

## [0.1.1] - 2025-01-22

### Fixed

- Removed unrelated shell.nix references

## [0.1.0] - 2025-01-21

### Added

- Async NNTP client with TLS support via rustls
- RFC 8054 COMPRESS DEFLATE (full session compression)
- XFEATURE COMPRESS GZIP fallback (headers-only compression)
- Connection pooling via bb8 with configurable pool size
- Exponential backoff retry with jitter
- XOVER command for article overview fetching
- Article, HEAD, and BODY fetch commands
- Bandwidth statistics tracking for compression
- Broken connection detection and removal
- RFC 5536 Article Validation: `validate_message_id()`, `validate_newsgroup_name()`, `parse_date()`, `validate_date()`, and `Headers::validate()` for validating article headers
- RFC 2047 Encoded Words: Full support for decoding non-ASCII characters in headers (Base64 and Quoted-Printable encodings)
- MIME Detection: `Article::is_mime()`, `Article::content_type()`, `Article::is_multipart()`, and `Article::charset()` methods
- Path Header Parsing: `Headers::parse_path()`, `Headers::originating_server()`, and `Headers::path_length()` methods
- ValidationConfig for configurable validation behavior (strict/lenient modes, future date checking, age limits)

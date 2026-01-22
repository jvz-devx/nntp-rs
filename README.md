# nntp-rs

[![Crates.io](https://img.shields.io/crates/v/nntp-rs.svg)](https://crates.io/crates/nntp-rs)
[![Documentation](https://docs.rs/nntp-rs/badge.svg)](https://docs.rs/nntp-rs)
[![CI](https://github.com/jvz-devx/nntp-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/jvz-devx/nntp-rs/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/jvz-devx/nntp-rs/branch/main/graph/badge.svg)](https://codecov.io/gh/jvz-devx/nntp-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

High-performance async NNTP client library for Rust with compression and connection pooling.

**Scope:** This library is designed for **reading and downloading** from Usenet/NNTP servers (client-to-server). Server-to-server peering and article posting are out of scope.

## Features

- **Async/await** - Built on Tokio for high-performance async I/O
- **TLS/SSL** - Secure connections via rustls (implicit TLS on port 563)
- **Compression** - RFC 8054 COMPRESS DEFLATE + XFEATURE COMPRESS GZIP with automatic fallback (50-80% bandwidth reduction)
- **Connection pooling** - bb8-based pool with configurable size
- **Retry logic** - Exponential backoff with jitter to prevent thundering herd
- **Binary support** - yEnc decoding, NZB parsing, PAR2 verification
- **Zero unsafe code** - Pure safe Rust

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
nntp-rs = "0.1"
```

## Quick Start

### Single Connection

```rust
use nntp_rs::{NntpClient, ServerConfig};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ServerConfig::tls("news.example.com", "username", "password");

    let mut client = NntpClient::connect(Arc::new(config)).await?;
    client.authenticate().await?;

    // Enable compression (optional, but recommended)
    client.try_enable_compression().await?;

    // Select a newsgroup
    let (count, first, last) = client.select_group("alt.test").await?;
    println!("Group has {} articles ({}-{})", count, first, last);

    // Fetch article overview data
    let entries = client.fetch_xover(&format!("{}-{}", last - 10, last)).await?;
    for entry in entries {
        println!("{}: {}", entry.article_number, entry.subject);
    }

    Ok(())
}
```

### Connection Pool (Recommended for high throughput)

```rust
use nntp_rs::{NntpPool, ServerConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ServerConfig::tls("news.example.com", "username", "password");

    // Create pool with 10 connections
    let pool = NntpPool::new(config, 10).await?;

    // Get connection from pool (automatically authenticated with compression)
    let mut conn = pool.get().await?;

    let (count, _, _) = conn.select_group("alt.test").await?;
    println!("Group has {} articles", count);

    // Connection returns to pool when dropped
    Ok(())
}
```

## Compression

This library supports two compression modes with automatic fallback:

1. **RFC 8054 COMPRESS DEFLATE** - Full bidirectional session compression (best)
2. **XFEATURE COMPRESS GZIP** - Headers-only compression (wider compatibility)

Compression is automatically negotiated when using the connection pool, or can be manually enabled:

```rust
let enabled = client.try_enable_compression().await?;
if enabled {
    let (compressed, decompressed) = client.get_bandwidth_stats();
    println!("Saved {} bytes", decompressed - compressed);
}
```

## TLS/Security

nntp-rs uses modern TLS with strong security defaults:

- **TLS 1.3** preferred, **TLS 1.2** supported (TLS 1.0/1.1 disabled per RFC 8996)
- **Strong cipher suites** with forward secrecy (ECDHE) and authenticated encryption (GCM/Poly1305)
- **Certificate validation** using Mozilla's CA bundle (webpki-roots)
- **Hostname verification** with SNI support

### Configuration

```rust
// Secure connection (recommended)
let config = ServerConfig::tls("news.example.com", "username", "password");

// Plaintext connection (not recommended)
let config = ServerConfig::plain("news.example.com", "username", "password");
```

## Article Parsing (RFC 5536)

nntp-rs provides comprehensive article parsing with validation and international character support:

### Features

- **Header parsing** - All RFC 5536 required and optional headers
- **Validation** - Message-ID, newsgroup names, and date format validation
- **RFC 2047 encoded words** - Automatic decoding of international characters in headers (UTF-8, ISO-8859-1, Windows-1252)
- **MIME detection** - Content-Type parsing and multipart detection
- **Path parsing** - Extract server routing information

### Example

```rust
use nntp_rs::{Article, ValidationConfig};

// Parse an article
let article = Article::parse(article_text)?;

// Access headers
println!("From: {}", article.headers.from);
println!("Subject: {}", article.headers.subject);  // RFC 2047 encoded words automatically decoded
println!("Message-ID: {}", article.headers.message_id);

// Validate article format
let config = ValidationConfig::strict();
article.headers.validate(&config)?;

// Check MIME type
if article.is_mime() {
    println!("Content-Type: {:?}", article.content_type());
    println!("Charset: {:?}", article.charset());
}

// Parse path
let servers = article.headers.parse_path();
println!("Article routed through {} servers", servers.len());
```

## RFC Compliance

| RFC | Title | Status | Test Coverage |
|-----|-------|--------|---------------|
| RFC 3977 | NNTP Core Protocol | Reader commands | ~600 tests |
| RFC 4642 | TLS with NNTP | Implicit TLS only | Verified |
| RFC 4643 | Authentication | USER/PASS + SASL PLAIN | ~100 tests |
| RFC 5536 | Netnews Article Format | Complete | ~156 tests |
| RFC 8054 | Compression | Complete | ~30 tests |
| RFC 8143 | TLS Best Practices | Compliant | Verified |
| RFC 7525 | BCP 195 TLS | Compliant | Verified |
| RFC 8996 | Deprecate TLS 1.0/1.1 | Compliant | Verified |
| RFC 4644 | Streaming Feeds | Out of scope | - |

**Total test coverage:** ~1,400 tests (>95% real behavioral tests)

*Note: RFC 4644 (streaming feeds) is for server-to-server peering and is out of scope for this client library.*

### What's Tested

- Core NNTP commands: GROUP, ARTICLE, HEAD, BODY, STAT, XOVER/OVER
- Authentication flows: AUTHINFO USER/PASS, SASL PLAIN
- Compression: COMPRESS DEFLATE, XFEATURE COMPRESS GZIP
- Response parsing and multi-line handling
- Connection pooling and retry logic
- Article format parsing (RFC 5536):
  - Header parsing and validation (Message-ID, newsgroup names, dates)
  - RFC 2047 encoded words (international characters in headers)
  - MIME detection and Content-Type parsing
  - Path header parsing
- yEnc decoding with CRC32 verification
- NZB parsing and segment ordering
- PAR2 file parsing and checksum extraction

## Current Limitations

### Out of Scope (by design)

This library focuses on **client-to-server reading/downloading**. The following server-to-server and posting features are intentionally not implemented:

- **POST/IHAVE** - Article posting (not a posting client)
- **RFC 4644** - Streaming feeds (CHECK/TAKETHIS) for server-to-server peering
- **yEnc encoding** - Only decoding; not designed for uploading binaries

### Not Yet Implemented

- **STARTTLS** - Only implicit TLS (port 563) is supported; STARTTLS upgrade not implemented
- **RFC 6048** - Extended LIST commands
- **PAR2 repair** - Only verification; Reed-Solomon recovery not implemented
- **Multi-server failover** - Single server only
- **Rate limiting** - No bandwidth throttling
- **Header caching** - No persistent cache

## API Reference

### NntpClient

- `connect(config)` - Connect to NNTP server
- `authenticate()` - Authenticate with username/password
- `authenticate_sasl(mechanism)` - SASL authentication
- `try_enable_compression()` - Enable compression (returns true if successful)
- `select_group(name)` - Select a newsgroup
- `fetch_article(id)` - Fetch full article
- `fetch_head(id)` - Fetch article headers only
- `fetch_body(id)` - Fetch article body only
- `fetch_xover(range)` - Fetch article overview data
- `quit()` - Close connection gracefully

### NntpPool

- `new(config, max_size)` - Create pool with default retry config
- `with_retry_config(config, max_size, retry_config)` - Create pool with custom retry
- `get()` - Get connection with automatic retry
- `get_no_retry()` - Get connection without retry
- `state()` - Get pool statistics

## License

MIT License - see [LICENSE](LICENSE) for details.

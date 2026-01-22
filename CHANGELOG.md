# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

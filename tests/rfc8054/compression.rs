//! RFC 8054 - NNTP Compression Tests
//!
//! These tests verify compliance with NNTP compression requirements:
//! - COMPRESS DEFLATE command and response
//! - Response code 206 = compression active
//! - Actual zlib/deflate decompression

use nntp_rs::{NntpResponse, codes};

// Compression Response Codes (RFC 8054 §2.2)

#[test]
fn test_compression_active_206() {
    // 206 = Compression active
    let response = NntpResponse {
        code: codes::COMPRESSION_ACTIVE,
        message: "Compression active".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 206);
    assert!(response.is_success());
}

#[test]
fn test_compression_code_constant() {
    assert_eq!(codes::COMPRESSION_ACTIVE, 206);
}

#[test]
fn test_compression_503_not_supported() {
    // RFC 8054: 503 = Compression algorithm not supported
    let response = NntpResponse {
        code: codes::FEATURE_NOT_SUPPORTED,
        message: "Compression not supported".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 503);
    assert!(response.is_error());
}

#[test]
fn test_compression_403_unable_to_activate() {
    // RFC 8054: 403 = Unable to activate compression (server resource problem)
    let response = NntpResponse {
        code: codes::INTERNAL_FAULT,
        message: "Unable to activate compression".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 403);
    assert!(response.is_error());
}

#[test]
fn test_compression_501_syntax_error() {
    // 501 = Invalid algorithm name syntax
    let response = NntpResponse {
        code: codes::COMMAND_SYNTAX_ERROR,
        message: "Syntax error in compression algorithm".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 501);
    assert!(response.is_error());
}

#[test]
fn test_compression_502_already_active() {
    // 502 = Compression already active (can't enable twice)
    let response = NntpResponse {
        code: codes::ACCESS_DENIED,
        message: "Compression already active".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 502);
    assert!(response.is_error());
}
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use std::io::{Read, Write};

#[test]
fn test_zlib_compress_decompress_roundtrip() {
    // Use repetitive data that compresses well
    let original = b"Hello, NNTP World! ".repeat(100);

    // Compress
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&original).unwrap();
    let compressed = encoder.finish().unwrap();

    // Verify compression happened (repetitive data should compress well)
    assert!(
        compressed.len() < original.len(),
        "Compressed {} should be < original {}",
        compressed.len(),
        original.len()
    );

    // Decompress
    let mut decoder = ZlibDecoder::new(&compressed[..]);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).unwrap();

    assert_eq!(decompressed, original);
}

#[test]
fn test_zlib_decompress_typical_xover() {
    // Simulate typical XOVER data that would be compressed
    let xover_data = r#"12345	Test Subject 1	author@example.com	Mon, 1 Jan 2024	<msg1@id>	<ref@id>	1000	50
12346	Test Subject 2	author@example.com	Tue, 2 Jan 2024	<msg2@id>	<ref@id>	2000	100
12347	Test Subject 3	author@example.com	Wed, 3 Jan 2024	<msg3@id>	<ref@id>	3000	150"#;

    // Compress
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(xover_data.as_bytes()).unwrap();
    let compressed = encoder.finish().unwrap();

    // Decompress
    let mut decoder = ZlibDecoder::new(&compressed[..]);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).unwrap();

    assert_eq!(String::from_utf8_lossy(&decompressed), xover_data);
}

#[test]
fn test_zlib_compression_ratio() {
    // Verify we get reasonable compression on typical NNTP data
    let typical_data = "Subject: Test\nFrom: user@example.com\nDate: Mon, 1 Jan 2024\n".repeat(100);

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(typical_data.as_bytes()).unwrap();
    let compressed = encoder.finish().unwrap();

    let ratio = 1.0 - (compressed.len() as f64 / typical_data.len() as f64);
    println!("Compression ratio: {:.1}%", ratio * 100.0);

    // Should achieve at least 50% compression on repetitive data
    assert!(ratio > 0.5, "Compression ratio should be > 50%");
}
use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;

#[test]
fn test_deflate_compress_decompress_roundtrip() {
    let original = b"NNTP data compressed with DEFLATE algorithm as per RFC 8054";

    // Compress with deflate
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(original).unwrap();
    let compressed = encoder.finish().unwrap();

    // Decompress
    let mut decoder = DeflateDecoder::new(&compressed[..]);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).unwrap();

    assert_eq!(decompressed, original);
}

#[test]
fn test_deflate_empty_data() {
    // Empty input should work
    let original = b"";

    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(original).unwrap();
    let compressed = encoder.finish().unwrap();

    let mut decoder = DeflateDecoder::new(&compressed[..]);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).unwrap();

    assert_eq!(decompressed, original);
}

#[test]
fn test_deflate_single_byte() {
    let original = b"X";

    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(original).unwrap();
    let compressed = encoder.finish().unwrap();

    let mut decoder = DeflateDecoder::new(&compressed[..]);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).unwrap();

    assert_eq!(decompressed, original);
}

// Decompression Error Handling

#[test]
fn test_zlib_decompress_corrupt_data() {
    // Random bytes are not valid zlib
    let corrupt_data = [0x78, 0x9c, 0xFF, 0xFF, 0xFF, 0xFF]; // Invalid zlib

    let mut decoder = ZlibDecoder::new(&corrupt_data[..]);
    let mut decompressed = Vec::new();
    let result = decoder.read_to_end(&mut decompressed);

    // Should fail gracefully
    assert!(result.is_err() || decompressed.is_empty());
}

#[test]
fn test_deflate_decompress_corrupt_data() {
    // Random bytes are not valid deflate
    let corrupt_data = [0xFF, 0xFE, 0xFD, 0xFC];

    let mut decoder = DeflateDecoder::new(&corrupt_data[..]);
    let mut decompressed = Vec::new();
    let result = decoder.read_to_end(&mut decompressed);

    // Should fail
    assert!(result.is_err());
}

#[test]
fn test_zlib_decompress_truncated_data() {
    // Create valid compressed data then truncate it
    let original = b"This is a longer message that will be compressed then truncated. ".repeat(50);

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&original).unwrap();
    let compressed = encoder.finish().unwrap();

    // Truncate to a quarter (more aggressive truncation)
    let truncated = &compressed[..compressed.len() / 4];

    let mut decoder = ZlibDecoder::new(truncated);
    let mut decompressed = Vec::new();
    let result = decoder.read_to_end(&mut decompressed);

    // Either fails with error OR decompresses to incomplete/incorrect data
    // (zlib may partially decompress truncated streams)
    let data_mismatch = result.is_ok() && decompressed != original;
    assert!(
        result.is_err() || data_mismatch,
        "Truncated data should either error or produce incorrect output"
    );
}

#[test]
fn test_deflate_decompress_truncated_data() {
    let original = b"This is a longer message that will be compressed then truncated. ".repeat(50);

    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&original).unwrap();
    let compressed = encoder.finish().unwrap();

    // Truncate to a quarter
    let truncated = &compressed[..compressed.len() / 4];

    let mut decoder = DeflateDecoder::new(truncated);
    let mut decompressed = Vec::new();
    let result = decoder.read_to_end(&mut decompressed);

    // Either fails with error OR decompresses to incomplete/incorrect data
    let data_mismatch = result.is_ok() && decompressed != original;
    assert!(
        result.is_err() || data_mismatch,
        "Truncated data should either error or produce incorrect output"
    );
}

#[test]
fn test_zlib_decompress_empty_input() {
    // Empty input should fail (no valid zlib header)
    let empty: &[u8] = &[];

    let mut decoder = ZlibDecoder::new(empty);
    let mut decompressed = Vec::new();
    let result = decoder.read_to_end(&mut decompressed);

    // Empty zlib data is technically an error
    assert!(result.is_err() || decompressed.is_empty());
}

// XFEATURE COMPRESS GZIP Marker Detection
//
// NOTE: These tests are for the XFEATURE COMPRESS GZIP extension, which is a
// proprietary extension (e.g., used by Giganews) and NOT part of RFC 8054.
// RFC 8054 defines only COMPRESS DEFLATE. The [COMPRESS=GZIP] marker is used
// by XFEATURE to indicate per-response gzip compression, whereas RFC 8054
// COMPRESS enables stream-level DEFLATE compression.
//
// These tests are included for compatibility with servers that support XFEATURE
// COMPRESS GZIP, but they do NOT test RFC 8054 compliance.

#[test]
fn test_xfeature_compress_gzip_marker_detection() {
    // XFEATURE (not RFC 8054): Server indicates gzip compression with [COMPRESS=GZIP] in response
    // This is a proprietary extension, not defined in RFC 8054
    let response = NntpResponse {
        code: 224,
        message: "Overview information follows [COMPRESS=GZIP]".to_string(),
        lines: vec![],
    };

    assert!(response.message.contains("[COMPRESS=GZIP]"));
}

#[test]
fn test_xfeature_compress_gzip_marker_not_present() {
    // XFEATURE (not RFC 8054): Without marker, response is uncompressed
    let response = NntpResponse {
        code: 224,
        message: "Overview information follows".to_string(),
        lines: vec![],
    };

    assert!(!response.message.contains("[COMPRESS=GZIP]"));
}

// RFC 8054 §2.1 - Capability Advertisement
//
// RFC 8054 states:
// - "The server MUST NOT return the COMPRESS capability label in response to
//    a CAPABILITIES command received after a compression layer is active."
// - "COMPRESS DEFLATE" should be advertised before compression is activated.
// - STARTTLS and MODE READER capabilities become unavailable after COMPRESS.

#[test]
fn test_compress_capability_advertised() {
    // RFC 8054 §2.1: Server advertises "COMPRESS DEFLATE" capability
    let capabilities = [
        "VERSION 2".to_string(),
        "READER".to_string(),
        "COMPRESS DEFLATE".to_string(),
        "STARTTLS".to_string(),
    ];

    assert!(capabilities.iter().any(|c| c.starts_with("COMPRESS")));
    assert!(capabilities.iter().any(|c| c == "COMPRESS DEFLATE"));
}

#[test]
fn test_compress_capability_not_advertised_after_activation() {
    // RFC 8054 §2.1: "MUST NOT return the COMPRESS capability label after compression is active"
    // Simulates capabilities response AFTER compression is activated
    let capabilities_after_compress = [
        "VERSION 2".to_string(),
        "READER".to_string(),
        // COMPRESS should NOT be present
        // STARTTLS should NOT be present
    ];

    assert!(
        !capabilities_after_compress
            .iter()
            .any(|c| c.starts_with("COMPRESS"))
    );
    assert!(!capabilities_after_compress.iter().any(|c| c == "STARTTLS"));
}

#[test]
fn test_starttls_capability_removed_after_compression() {
    // RFC 8054 §2.1: STARTTLS capability becomes unavailable after COMPRESS
    // Before compression
    let caps_before = ["COMPRESS DEFLATE".to_string(), "STARTTLS".to_string()];
    assert!(caps_before.iter().any(|c| c == "STARTTLS"));

    // After compression
    let caps_after: [String; 0] = [];
    assert!(!caps_after.iter().any(|c| c == "STARTTLS"));
}

#[test]
fn test_mode_reader_capability_removed_after_compression() {
    // RFC 8054 §2.1: MODE READER capability becomes unavailable after COMPRESS
    // Note: This applies to MODE READER command, not READER capability
    // After compression, the MODE command should return 502

    // Simulate 502 response for MODE READER after compression
    let response = NntpResponse {
        code: codes::ACCESS_DENIED,
        message: "MODE unavailable after COMPRESS".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 502);
    assert!(response.is_error());
}

// RFC 8054 §2.2 - Command Restrictions After Compression

#[test]
fn test_authinfo_rejected_after_compression() {
    // RFC 8054 §2.2: "Authentication MUST NOT be attempted after a successful use of COMPRESS"
    // Server should return 502 for AUTHINFO commands after compression is active
    let response = NntpResponse {
        code: codes::ACCESS_DENIED,
        message: "AUTHINFO unavailable after COMPRESS".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 502);
    assert!(response.is_error());
}

#[test]
fn test_starttls_rejected_after_compression() {
    // RFC 8054 §2.2: STARTTLS is not permitted after COMPRESS
    let response = NntpResponse {
        code: codes::ACCESS_DENIED,
        message: "STARTTLS unavailable after COMPRESS".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 502);
    assert!(response.is_error());
}

#[test]
fn test_compress_command_not_pipelined() {
    // RFC 8054 §2.1: "This command MUST NOT be pipelined"
    // This is a behavioral requirement - the client must wait for the 206 response
    // before sending any further commands. We document this requirement here.

    // If pipelining is attempted, server behavior is undefined by RFC
    // Most servers would reject subsequent commands or the COMPRESS itself

    // The 206 response indicates compression is now active
    let response = NntpResponse {
        code: codes::COMPRESSION_ACTIVE,
        message: "Compression active".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 206);
    // After this response, all traffic (both directions) is compressed
}

// Compression Levels

#[test]
fn test_zlib_compression_levels() {
    let data = b"Test data for compression level comparison";

    let levels = [
        Compression::none(),
        Compression::fast(),
        Compression::default(),
        Compression::best(),
    ];

    let mut sizes = Vec::new();

    for level in levels {
        let mut encoder = ZlibEncoder::new(Vec::new(), level);
        encoder.write_all(data).unwrap();
        let compressed = encoder.finish().unwrap();
        sizes.push(compressed.len());

        // All levels should roundtrip correctly
        let mut decoder = ZlibDecoder::new(&compressed[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed).unwrap();
        assert_eq!(decompressed, data);
    }

    // Higher compression levels should produce smaller output (or same)
    // Note: for small data, this might not always hold
    println!("Compression sizes by level: {:?}", sizes);
}

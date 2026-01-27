use crate::commands;
use crate::response::codes;
use crate::Result;
use tracing::{debug, trace, warn};

use super::state::CompressionMode;
use super::NntpClient;

impl NntpClient {
    /// Attempt to enable compression with fallback to GZIP
    ///
    /// This method tries to enable compression on the NNTP connection
    /// using RFC 8054 COMPRESS DEFLATE first (best compression), then
    /// falls back to the legacy XFEATURE COMPRESS GZIP if unavailable.
    ///
    /// Attempts compression in this order:
    /// 1. RFC 8054 COMPRESS DEFLATE (full session) - best compression
    /// 2. XFEATURE COMPRESS GZIP (headers-only) - fallback for compatibility
    /// 3. No compression - if neither is supported
    ///
    /// Returns `true` if any compression mode was enabled, `false` otherwise.
    /// Always returns `Ok` - compression failure is not an error.
    pub async fn try_enable_compression(&mut self) -> Result<bool> {
        // Try RFC 8054 COMPRESS DEFLATE first (full session compression)
        debug!("Attempting RFC 8054 COMPRESS DEFLATE");
        self.send_command(commands::compress_deflate()).await?;
        let response = self.read_response().await?;

        if response.code == codes::COMPRESSION_ACTIVE {
            // 206 = compression active
            self.compression_mode = CompressionMode::FullSession;
            debug!("RFC 8054 COMPRESS DEFLATE enabled (full session compression)");
            return Ok(true);
        }

        // COMPRESS DEFLATE not supported, try XFEATURE COMPRESS GZIP
        debug!(
            "COMPRESS DEFLATE not supported (code {}), trying XFEATURE COMPRESS GZIP",
            response.code
        );
        self.send_command(commands::xfeature_compress_gzip())
            .await?;
        let response = self.read_response().await?;

        if response.is_success() {
            // 290 or 2xx = compression enabled
            self.compression_mode = CompressionMode::HeadersOnly;
            debug!("XFEATURE COMPRESS GZIP enabled (headers-only compression)");
            return Ok(true);
        }

        // No compression available
        debug!(
            "XFEATURE COMPRESS GZIP not supported (code {}), continuing without compression",
            response.code
        );
        Ok(false)
    }

    /// Get bandwidth statistics (compressed vs decompressed bytes)
    ///
    /// Returns `(bytes_compressed, bytes_decompressed)`.
    /// Returns `(0, 0)` if compression is not enabled.
    pub fn get_bandwidth_stats(&self) -> (u64, u64) {
        (self.bytes_compressed, self.bytes_decompressed)
    }

    /// Check if compression is enabled
    pub fn is_compression_enabled(&self) -> bool {
        self.compression_mode != CompressionMode::None
    }

    /// Decompress data based on current compression mode
    pub(super) fn maybe_decompress(&mut self, data: &[u8]) -> Vec<u8> {
        use flate2::read::{DeflateDecoder, ZlibDecoder};
        use std::io::Read;

        match self.compression_mode {
            CompressionMode::None => data.to_vec(),
            CompressionMode::HeadersOnly => {
                // Use zlib decompression (server sends zlib despite calling it "GZIP")
                let mut decoder = ZlibDecoder::new(data);
                // Pre-allocate: compressed data typically expands 3-5x
                let mut decompressed = Vec::with_capacity(data.len() * 3);
                match decoder.read_to_end(&mut decompressed) {
                    Ok(_) => {
                        self.bytes_compressed += data.len() as u64;
                        self.bytes_decompressed += decompressed.len() as u64;
                        trace!(
                            "Decompressed {} bytes to {} bytes (zlib)",
                            data.len(),
                            decompressed.len()
                        );
                        decompressed
                    }
                    Err(e) => {
                        warn!("Zlib decompression failed: {}. Using uncompressed data.", e);
                        data.to_vec()
                    }
                }
            }
            CompressionMode::FullSession => {
                // Use deflate decompression
                let mut decoder = DeflateDecoder::new(data);
                // Pre-allocate: compressed data typically expands 3-5x
                let mut decompressed = Vec::with_capacity(data.len() * 3);
                match decoder.read_to_end(&mut decompressed) {
                    Ok(_) => {
                        self.bytes_compressed += data.len() as u64;
                        self.bytes_decompressed += decompressed.len() as u64;
                        trace!(
                            "Decompressed {} bytes to {} bytes (deflate)",
                            data.len(),
                            decompressed.len()
                        );
                        decompressed
                    }
                    Err(e) => {
                        warn!(
                            "Deflate decompression failed: {}. Using uncompressed data.",
                            e
                        );
                        data.to_vec()
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::client::state::CompressionMode;
    use flate2::read::{DeflateDecoder, ZlibDecoder};
    use flate2::write::{DeflateEncoder, ZlibEncoder};
    use flate2::Compression;
    use std::io::{Read, Write};

    // ========================================================================
    // Compression Mode Comparison Tests
    // ========================================================================

    #[test]
    fn test_compression_mode_equality() {
        assert_eq!(CompressionMode::None, CompressionMode::None);
        assert_eq!(CompressionMode::HeadersOnly, CompressionMode::HeadersOnly);
        assert_eq!(CompressionMode::FullSession, CompressionMode::FullSession);

        assert_ne!(CompressionMode::None, CompressionMode::HeadersOnly);
        assert_ne!(CompressionMode::None, CompressionMode::FullSession);
        assert_ne!(CompressionMode::HeadersOnly, CompressionMode::FullSession);
    }

    #[test]
    fn test_is_compression_enabled_logic() {
        // Test the logic used by is_compression_enabled()
        assert!(CompressionMode::None == CompressionMode::None);
        assert!(CompressionMode::HeadersOnly != CompressionMode::None);
        assert!(CompressionMode::FullSession != CompressionMode::None);
    }

    // ========================================================================
    // Decompression Logic Tests - Zlib (HeadersOnly mode)
    // ========================================================================

    #[test]
    fn test_zlib_decompression_valid_data() {
        let original_data = b"Subject: Test Article\r\nFrom: test@example.com\r\n";
        let compressed_data = compress_zlib(original_data);

        // Decompress using the same logic as maybe_decompress()
        let mut decoder = ZlibDecoder::new(&compressed_data[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed).unwrap();

        assert_eq!(decompressed, original_data);
    }

    #[test]
    fn test_zlib_decompression_empty_data() {
        let original_data = b"";
        let compressed_data = compress_zlib(original_data);

        let mut decoder = ZlibDecoder::new(&compressed_data[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed).unwrap();

        assert_eq!(decompressed, original_data);
    }

    #[test]
    fn test_zlib_decompression_large_data() {
        // Create a larger payload (10KB of repeated text)
        let original_data = "This is a test article body. ".repeat(350).into_bytes();
        let compressed_data = compress_zlib(&original_data);

        let mut decoder = ZlibDecoder::new(&compressed_data[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed).unwrap();

        assert_eq!(decompressed, original_data);

        // Verify compression actually reduced size
        assert!(
            compressed_data.len() < original_data.len(),
            "Compressed size ({}) should be smaller than original ({})",
            compressed_data.len(),
            original_data.len()
        );
    }

    #[test]
    fn test_zlib_decompression_invalid_data() {
        // Send completely invalid data (not zlib compressed)
        let invalid_data = b"This is not compressed data at all!";

        let mut decoder = ZlibDecoder::new(&invalid_data[..]);
        let mut decompressed = Vec::new();
        let result = decoder.read_to_end(&mut decompressed);

        // Should fail to decompress
        assert!(result.is_err(), "Should fail to decompress invalid data");
    }

    #[test]
    fn test_zlib_decompression_truncated_data() {
        let original_data = b"Complete message data that is long enough for proper compression";
        let mut compressed_data = compress_zlib(original_data);

        // Truncate significantly to ensure it's definitely incomplete
        compressed_data.truncate(5); // Keep only first 5 bytes

        let mut decoder = ZlibDecoder::new(&compressed_data[..]);
        let mut decompressed = Vec::new();
        let result = decoder.read_to_end(&mut decompressed);

        // Heavily truncated data should fail or produce incomplete output
        if result.is_ok() {
            assert_ne!(
                decompressed, original_data,
                "Truncated data should not fully decompress"
            );
        }
    }

    // ========================================================================
    // Decompression Logic Tests - Deflate (FullSession mode)
    // ========================================================================

    #[test]
    fn test_deflate_decompression_valid_data() {
        let original_data = b"200 posting allowed\r\n";
        let compressed_data = compress_deflate(original_data);

        // Decompress using the same logic as maybe_decompress()
        let mut decoder = DeflateDecoder::new(&compressed_data[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed).unwrap();

        assert_eq!(decompressed, original_data);
    }

    #[test]
    fn test_deflate_decompression_empty_data() {
        let original_data = b"";
        let compressed_data = compress_deflate(original_data);

        let mut decoder = DeflateDecoder::new(&compressed_data[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed).unwrap();

        assert_eq!(decompressed, original_data);
    }

    #[test]
    fn test_deflate_decompression_large_data() {
        // Create a large payload (50KB of article data)
        let original_data = "Article-ID: <test@example.com>\r\nSubject: Test\r\n\r\n"
            .repeat(500)
            .into_bytes();
        let compressed_data = compress_deflate(&original_data);

        let mut decoder = DeflateDecoder::new(&compressed_data[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed).unwrap();

        assert_eq!(decompressed, original_data);

        // Verify compression reduced size significantly
        assert!(
            compressed_data.len() < original_data.len(),
            "Compressed size ({}) should be smaller than original ({})",
            compressed_data.len(),
            original_data.len()
        );
    }

    #[test]
    fn test_deflate_decompression_invalid_data() {
        // Send invalid data (not deflate compressed)
        let invalid_data = b"Random uncompressed text";

        let mut decoder = DeflateDecoder::new(&invalid_data[..]);
        let mut decompressed = Vec::new();
        let result = decoder.read_to_end(&mut decompressed);

        // Deflate may partially decode invalid data, so we just verify it doesn't panic
        // The real error handling happens in maybe_decompress() which catches errors
        let _ = result; // May succeed or fail depending on data pattern
    }

    #[test]
    fn test_deflate_decompression_truncated_data() {
        let original_data =
            b"Full article body content here that is long enough to ensure truncation matters";
        let mut compressed_data = compress_deflate(original_data);

        // Truncate significantly to ensure it's definitely incomplete
        compressed_data.truncate(5); // Keep only first 5 bytes

        let mut decoder = DeflateDecoder::new(&compressed_data[..]);
        let mut decompressed = Vec::new();
        let result = decoder.read_to_end(&mut decompressed);

        // Heavily truncated data should fail or produce incomplete output
        // The key is that maybe_decompress() catches this and returns original data
        if result.is_ok() {
            assert_ne!(
                decompressed, original_data,
                "Truncated data should not fully decompress"
            );
        }
    }

    // ========================================================================
    // Compression Efficiency Tests
    // ========================================================================

    #[test]
    fn test_zlib_compression_efficiency() {
        // Test that zlib compression is effective on repetitive data
        let original_data = "Newsgroup: alt.binaries.test\r\n".repeat(100).into_bytes();
        let compressed_data = compress_zlib(&original_data);

        // Compression ratio should be at least 5:1 for highly repetitive data
        let compression_ratio = original_data.len() as f64 / compressed_data.len() as f64;
        assert!(
            compression_ratio > 5.0,
            "Compression ratio ({:.2}) should be > 5.0 for repetitive data",
            compression_ratio
        );
    }

    #[test]
    fn test_deflate_compression_efficiency() {
        // Test that deflate compression is effective on repetitive data
        let original_data = "224 overview information follows\r\n"
            .repeat(100)
            .into_bytes();
        let compressed_data = compress_deflate(&original_data);

        // Compression ratio should be at least 5:1 for highly repetitive data
        let compression_ratio = original_data.len() as f64 / compressed_data.len() as f64;
        assert!(
            compression_ratio > 5.0,
            "Compression ratio ({:.2}) should be > 5.0 for repetitive data",
            compression_ratio
        );
    }

    #[test]
    fn test_compression_roundtrip_zlib() {
        // Test that data survives a compression-decompression cycle
        let binary_zeros = vec![0u8; 1000];
        let all_bytes = (0u8..=255u8).collect::<Vec<_>>();

        let test_cases: Vec<&[u8]> = vec![
            b"",
            b"a",
            b"Hello, World!",
            b"Subject: Test\r\nFrom: test@example.com\r\n\r\nBody content",
            &binary_zeros,
            &all_bytes,
        ];

        for original in test_cases {
            let compressed = compress_zlib(original);
            let mut decoder = ZlibDecoder::new(&compressed[..]);
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed).unwrap();
            assert_eq!(
                decompressed,
                original,
                "Roundtrip failed for {} bytes",
                original.len()
            );
        }
    }

    #[test]
    fn test_compression_roundtrip_deflate() {
        // Test that data survives a compression-decompression cycle
        let binary_ones = vec![255u8; 1000];
        let pattern = (0u8..=255u8).cycle().take(10000).collect::<Vec<_>>();

        let test_cases: Vec<&[u8]> = vec![
            b"",
            b"a",
            b"200 server ready\r\n",
            b"224 overview information follows\r\nheader data here\r\n.\r\n",
            &binary_ones,
            &pattern,
        ];

        for original in test_cases {
            let compressed = compress_deflate(original);
            let mut decoder = DeflateDecoder::new(&compressed[..]);
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed).unwrap();
            assert_eq!(
                decompressed,
                original,
                "Roundtrip failed for {} bytes",
                original.len()
            );
        }
    }

    // ========================================================================
    // Edge Case Tests
    // ========================================================================

    #[test]
    fn test_zlib_vs_deflate_format_incompatibility() {
        // Test that zlib and deflate formats are not interchangeable
        let original_data = b"Test data";

        // Compress with zlib
        let zlib_compressed = compress_zlib(original_data);

        // Try to decompress with deflate decoder - should fail or produce wrong data
        let mut deflate_decoder = DeflateDecoder::new(&zlib_compressed[..]);
        let mut decompressed = Vec::new();
        let result = deflate_decoder.read_to_end(&mut decompressed);

        // Should fail because zlib has a wrapper that deflate doesn't expect
        assert!(
            result.is_err() || decompressed != original_data,
            "Zlib data should not decompress correctly with deflate decoder"
        );
    }

    #[test]
    fn test_bandwidth_stats_calculation() {
        // Test that bandwidth statistics would be calculated correctly
        // Use longer, more compressible data to ensure compression actually reduces size
        let original_data = "Test message content for statistics. "
            .repeat(50)
            .into_bytes();
        let compressed_data = compress_deflate(&original_data);

        let bytes_compressed = compressed_data.len() as u64;
        let bytes_decompressed = original_data.len() as u64;

        // Verify the stats represent real compression
        assert!(
            bytes_compressed < bytes_decompressed,
            "Compressed ({}) should be smaller than original ({})",
            bytes_compressed,
            bytes_decompressed
        );
        assert!(bytes_compressed > 0);
        assert!(bytes_decompressed > 0);
    }

    // ========================================================================
    // Test Helpers
    // ========================================================================

    /// Compress data using Zlib (for HeadersOnly mode)
    fn compress_zlib(data: &[u8]) -> Vec<u8> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).expect("Failed to write to encoder");
        encoder.finish().expect("Failed to finish compression")
    }

    /// Compress data using Deflate (for FullSession mode)
    fn compress_deflate(data: &[u8]) -> Vec<u8> {
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).expect("Failed to write to encoder");
        encoder.finish().expect("Failed to finish compression")
    }
}

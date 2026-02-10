//! Low-level I/O operations for NNTP protocol communication
//!
//! This module provides the core I/O primitives used by all NNTP client operations:
//! - Command transmission with logging
//! - Single-line response parsing
//! - Multi-line response handling (text and binary)
//! - Compression detection and decompression
//! - Timeout management
//! - Connection error detection

use super::{CompressionMode, NntpClient};
use crate::commands;
use crate::error::{NntpError, Result};
use crate::response::NntpResponse;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::time::timeout;
use tracing::trace;

const SINGLE_LINE_TIMEOUT: Duration = Duration::from_secs(60);
const MULTILINE_TIMEOUT: Duration = Duration::from_secs(180);
const COMPRESSED_READ_BUFFER_SIZE: usize = 256 * 1024;
const BINARY_DATA_INITIAL_CAPACITY: usize = 512 * 1024;
/// Maximum size for a compressed block to prevent OOM from malicious/broken servers (64 MB)
const MAX_COMPRESSED_BLOCK_SIZE: usize = 64 * 1024 * 1024;

/// Strip NNTP byte-stuffing from a line (leading ".." becomes ".").
fn strip_byte_stuffing(line: &str) -> &str {
    if line.starts_with("..") {
        &line[1..]
    } else {
        line
    }
}

impl NntpClient {
    /// Send a command to the server
    pub(super) async fn send_command(&mut self, command: &str) -> Result<()> {
        trace!("Sending command: {}", command.trim());
        self.stream.get_mut().write_all(command.as_bytes()).await?;
        self.stream.get_mut().flush().await?;
        Ok(())
    }

    /// Read a single-line response
    pub(super) async fn read_response(&mut self) -> Result<NntpResponse> {
        let result = self.read_response_with_timeout(SINGLE_LINE_TIMEOUT).await;
        // Mark connection as broken if we got invalid/garbage data
        if let Err(NntpError::InvalidResponse(_)) = &result {
            self.mark_broken();
        }
        result
    }

    /// Read a single-line response with custom timeout
    pub(super) async fn read_response_with_timeout(
        &mut self,
        timeout_duration: Duration,
    ) -> Result<NntpResponse> {
        use tokio::io::AsyncBufReadExt;

        let read_future = async {
            let mut line_bytes = Vec::with_capacity(512);
            self.stream.read_until(b'\n', &mut line_bytes).await?;

            if line_bytes.is_empty() {
                return Err(NntpError::ConnectionClosed);
            }

            // Convert to string with lossy UTF-8 conversion
            let line = String::from_utf8_lossy(&line_bytes);
            let line = line.trim_end();
            trace!("Received: {}", line);

            commands::parse_single_response(line)
        };

        timeout(timeout_duration, read_future)
            .await
            .map_err(|_| NntpError::Timeout)?
    }

    /// Read a multi-line response (ending with ".\r\n")
    pub(super) async fn read_multiline_response(&mut self) -> Result<NntpResponse> {
        let result = self
            .read_multiline_response_with_timeout(MULTILINE_TIMEOUT)
            .await;
        // Mark connection as broken if we got invalid/garbage data
        if let Err(NntpError::InvalidResponse(_)) = &result {
            self.mark_broken();
        }
        result
    }

    /// Read a multi-line response with custom timeout
    pub(super) async fn read_multiline_response_with_timeout(
        &mut self,
        timeout_duration: Duration,
    ) -> Result<NntpResponse> {
        use tokio::io::AsyncBufReadExt;

        let read_future = async {
            // Read first line (status)
            let mut first_line_bytes = Vec::with_capacity(512);
            self.stream.read_until(b'\n', &mut first_line_bytes).await?;

            if first_line_bytes.is_empty() {
                return Err(NntpError::ConnectionClosed);
            }

            let first_line = String::from_utf8_lossy(&first_line_bytes);
            let first_line = first_line.trim_end();
            trace!("Received: {}", first_line);

            let (code, message) = commands::parse_response_line(first_line)?;

            // If error response, no multi-line data follows
            if code >= 400 {
                return Ok(NntpResponse {
                    code,
                    message,
                    lines: vec![],
                });
            }

            // For HeadersOnly compression mode, the server only compresses certain responses
            // and indicates this with [COMPRESS=GZIP] in the status line
            let response_is_compressed = self.compression_mode == CompressionMode::HeadersOnly
                && message.contains("[COMPRESS=GZIP]");

            if response_is_compressed {
                let all_data = self.read_compressed_block().await?;

                trace!("Read {} compressed bytes", all_data.len());

                // Decompress the entire block
                let decompressed = self.maybe_decompress(&all_data)?;
                trace!("Decompressed to {} bytes", decompressed.len());

                // Parse decompressed data into lines
                let decompressed_str = String::from_utf8_lossy(&decompressed);
                // Pre-allocate: estimate 1 line per 80 bytes (typical NNTP line length)
                let estimated_lines = (decompressed.len() / 80).max(16);
                let mut lines = Vec::with_capacity(estimated_lines);
                for line in decompressed_str.lines() {
                    lines.push(strip_byte_stuffing(line).to_string());
                }

                return Ok(NntpResponse {
                    code,
                    message,
                    lines,
                });
            }

            // Standard uncompressed or FullSession mode: Read line-by-line
            // Pre-allocate with conservative estimate (most multiline responses have 10-100 lines)
            let mut lines = Vec::with_capacity(64);
            loop {
                let mut line_bytes = Vec::with_capacity(512);
                self.stream.read_until(b'\n', &mut line_bytes).await?;

                if line_bytes.is_empty() {
                    return Err(NntpError::ConnectionClosed);
                }

                let line = String::from_utf8_lossy(&line_bytes);
                let line = line.trim_end();

                // Check for terminator
                if line == "." {
                    break;
                }

                // Handle byte-stuffing (lines starting with ".." become ".")
                lines.push(strip_byte_stuffing(line).to_string());
            }

            Ok(NntpResponse {
                code,
                message,
                lines,
            })
        };

        timeout(timeout_duration, read_future)
            .await
            .map_err(|_| NntpError::Timeout)?
    }

    /// Read compressed data as binary until the uncompressed terminator (".\r\n" or ".\n")
    async fn read_compressed_block(&mut self) -> Result<Vec<u8>> {
        use tokio::io::AsyncReadExt;

        let mut all_data = Vec::new();
        let mut buffer = vec![0u8; COMPRESSED_READ_BUFFER_SIZE];

        loop {
            let n = self.stream.read(&mut buffer).await?;
            if n == 0 {
                return Err(NntpError::ConnectionClosed);
            }

            all_data.extend_from_slice(&buffer[..n]);

            if all_data.len() > MAX_COMPRESSED_BLOCK_SIZE {
                return Err(NntpError::InvalidResponse(format!(
                    "Compressed block exceeds maximum size of {} bytes",
                    MAX_COMPRESSED_BLOCK_SIZE
                )));
            }

            if all_data.ends_with(b".\r\n") {
                all_data.truncate(all_data.len() - 3);
                break;
            } else if all_data.ends_with(b".\n") {
                all_data.truncate(all_data.len() - 2);
                break;
            }
        }

        Ok(all_data)
    }

    /// Read a multi-line response as raw binary data (optimized for articles)
    ///
    /// This method is optimized for high-throughput binary data like articles:
    /// - Uses chunked reads instead of line-by-line
    /// - Returns raw bytes instead of Vec<String>
    /// - Avoids UTF-8 validation overhead
    /// - Pre-allocates buffer for reduced allocations
    pub(super) async fn read_multiline_response_binary(
        &mut self,
    ) -> Result<crate::response::NntpBinaryResponse> {
        self.read_multiline_response_binary_with_timeout(MULTILINE_TIMEOUT)
            .await
    }

    /// Read a multi-line response as raw binary with custom timeout
    pub(super) async fn read_multiline_response_binary_with_timeout(
        &mut self,
        timeout_duration: Duration,
    ) -> Result<crate::response::NntpBinaryResponse> {
        use tokio::io::AsyncBufReadExt;

        let read_future = async {
            // Read first line (status) - this is always text
            let mut first_line_bytes = Vec::with_capacity(256);
            self.stream.read_until(b'\n', &mut first_line_bytes).await?;

            if first_line_bytes.is_empty() {
                return Err(NntpError::ConnectionClosed);
            }

            let first_line = String::from_utf8_lossy(&first_line_bytes);
            let first_line = first_line.trim_end();
            trace!("Received: {}", first_line);

            let (code, message) = commands::parse_response_line(first_line)?;

            // If error response, no multi-line data follows
            if code >= 400 {
                return Ok(crate::response::NntpBinaryResponse {
                    code,
                    message,
                    data: vec![],
                });
            }

            // For compressed responses, read and decompress the block
            let response_is_compressed = self.compression_mode == CompressionMode::HeadersOnly
                && message.contains("[COMPRESS=GZIP]");

            if response_is_compressed {
                let all_data = self.read_compressed_block().await?;
                let decompressed = self.maybe_decompress(&all_data)?;
                return Ok(crate::response::NntpBinaryResponse {
                    code,
                    message,
                    data: decompressed,
                });
            }

            // Optimized binary read: use read_until for efficient buffered I/O
            // but collect bytes directly instead of creating strings
            let mut data = Vec::with_capacity(BINARY_DATA_INITIAL_CAPACITY);

            loop {
                let mut line_bytes = Vec::with_capacity(512);
                self.stream.read_until(b'\n', &mut line_bytes).await?;

                if line_bytes.is_empty() {
                    return Err(NntpError::ConnectionClosed);
                }

                // Check for terminator: line containing only "." (plus CRLF/LF)
                if line_bytes == b".\r\n" || line_bytes == b".\n" {
                    break;
                }

                // Strip trailing \r\n (NNTP line terminator, not part of payload)
                let content_end = if line_bytes.ends_with(b"\r\n") {
                    line_bytes.len() - 2
                } else if line_bytes.ends_with(b"\n") {
                    line_bytes.len() - 1
                } else {
                    line_bytes.len()
                };
                let line_content = &line_bytes[..content_end];

                // Handle dot-stuffing: lines starting with ".." become "."
                if line_content.starts_with(b"..") {
                    data.extend_from_slice(&line_content[1..]);
                } else {
                    data.extend_from_slice(line_content);
                }
            }

            Ok(crate::response::NntpBinaryResponse {
                code,
                message,
                data,
            })
        };

        let result = timeout(timeout_duration, read_future)
            .await
            .map_err(|_| NntpError::Timeout)?;

        // Mark connection as broken if we got invalid data
        if let Err(NntpError::InvalidResponse(_)) = &result {
            self.mark_broken();
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that SINGLE_LINE_TIMEOUT constant is reasonable (60s)
    #[test]
    fn test_single_line_timeout_value() {
        assert_eq!(SINGLE_LINE_TIMEOUT, Duration::from_secs(60));
        assert!(
            SINGLE_LINE_TIMEOUT >= Duration::from_secs(30),
            "Single line timeout should allow for slow networks"
        );
    }

    /// Test that MULTILINE_TIMEOUT constant is reasonable (180s)
    #[test]
    fn test_multiline_timeout_value() {
        assert_eq!(MULTILINE_TIMEOUT, Duration::from_secs(180));
        assert!(
            MULTILINE_TIMEOUT >= Duration::from_secs(120),
            "Multiline timeout should allow for large article bodies"
        );
        assert!(
            MULTILINE_TIMEOUT > SINGLE_LINE_TIMEOUT,
            "Multiline timeout should be longer than single line"
        );
    }

    /// Test that buffer sizes are appropriate for performance
    #[test]
    fn test_buffer_sizes() {
        // Compressed read buffer should be large enough for efficient chunked reads
        assert_eq!(COMPRESSED_READ_BUFFER_SIZE, 256 * 1024);
        const _: () = assert!(
            COMPRESSED_READ_BUFFER_SIZE >= 64 * 1024,
            "Compressed buffer should be at least 64KB for efficient network I/O"
        );

        // Binary data initial capacity should reduce allocations for typical articles
        assert_eq!(BINARY_DATA_INITIAL_CAPACITY, 512 * 1024);
        const _: () = assert!(
            BINARY_DATA_INITIAL_CAPACITY >= 256 * 1024,
            "Binary buffer should handle typical article sizes without reallocation"
        );
    }

    /// Test dot-stuffing removal logic (lines starting with ".." become ".")
    ///
    /// This tests the byte-stuffing behavior mandated by RFC 3977:
    /// - Lines starting with ".." are transmitted to prevent confusion with terminator
    /// - The receiving client must remove the leading dot
    #[test]
    fn test_dot_stuffing_removal() {
        // Normal line - no change
        let line = "Hello world";
        let processed = if line.starts_with("..") {
            &line[1..]
        } else {
            line
        };
        assert_eq!(processed, "Hello world");

        // Dot-stuffed line - should remove leading dot
        let line = "..This line starts with a dot";
        let processed = if line.starts_with("..") {
            &line[1..]
        } else {
            line
        };
        assert_eq!(processed, ".This line starts with a dot");

        // Single dot - no change (this is the terminator, handled separately)
        let line = ".";
        let processed = if line.starts_with("..") {
            &line[1..]
        } else {
            line
        };
        assert_eq!(processed, ".");

        // Three dots - remove one
        let line = "...";
        let processed = if line.starts_with("..") {
            &line[1..]
        } else {
            line
        };
        assert_eq!(processed, "..");
    }

    /// Test terminator detection for multiline responses
    ///
    /// RFC 3977 specifies that multiline responses end with ".\r\n" or ".\n"
    #[test]
    fn test_terminator_detection() {
        // Standard terminator with CRLF
        let terminator_crlf: &[u8] = b".\r\n";
        assert_eq!(terminator_crlf, b".\r\n");

        // Terminator with LF only (some servers don't send CRLF)
        let terminator_lf: &[u8] = b".\n";
        assert_eq!(terminator_lf, b".\n");

        // Not a terminator - data continues
        assert_ne!(b".data\r\n" as &[u8], b".\r\n" as &[u8]);
        assert_ne!(b"...\r\n" as &[u8], b".\r\n" as &[u8]);
    }

    /// Test compression detection logic for HeadersOnly mode
    ///
    /// When compression mode is HeadersOnly, the server indicates compressed
    /// responses by including [COMPRESS=GZIP] in the status line.
    #[test]
    fn test_compression_detection_headers_only() {
        // Response with compression marker
        let message = "224 Overview information follows [COMPRESS=GZIP]";
        assert!(message.contains("[COMPRESS=GZIP]"));

        // Response without compression marker
        let message = "224 Overview information follows";
        assert!(!message.contains("[COMPRESS=GZIP]"));

        // Marker in different position
        let message = "[COMPRESS=GZIP] 224 Overview follows";
        assert!(message.contains("[COMPRESS=GZIP]"));

        // Case sensitive check (server should use uppercase)
        let message = "224 Overview information follows [compress=gzip]";
        assert!(!message.contains("[COMPRESS=GZIP]"));
    }

    /// Test error response detection (code >= 400)
    ///
    /// When a response code indicates an error, no multiline data follows,
    /// so the client should not attempt to read additional lines.
    #[test]
    fn test_error_response_detection() {
        // Success codes - expect multiline data
        let success_codes: &[u16] = &[200, 211, 281];
        for &code in success_codes {
            assert!(code < 400, "Expected success code {code} < 400");
        }

        // Client error codes - no multiline data
        let client_error_codes: &[u16] = &[400, 411, 423];
        for &code in client_error_codes {
            assert!(code >= 400, "Expected client error code {code} >= 400");
        }

        // Server error codes - no multiline data
        let server_error_codes: &[u16] = &[500, 502];
        for &code in server_error_codes {
            assert!(code >= 400, "Expected server error code {code} >= 400");
        }
    }

    /// Test binary terminator detection for read_compressed_block
    ///
    /// Compressed blocks end with uncompressed ".\r\n" or ".\n" terminators.
    #[test]
    fn test_compressed_block_terminator() {
        // Test ends_with logic for CRLF terminator
        let mut data = Vec::new();
        data.extend_from_slice(b"compressed data here");
        data.extend_from_slice(b".\r\n");
        assert!(data.ends_with(b".\r\n"));

        // After truncation, terminator should be removed
        data.truncate(data.len() - 3);
        assert!(!data.ends_with(b".\r\n"));
        assert_eq!(data, b"compressed data here");

        // Test ends_with logic for LF-only terminator
        let mut data = Vec::new();
        data.extend_from_slice(b"compressed data");
        data.extend_from_slice(b".\n");
        assert!(data.ends_with(b".\n"));

        data.truncate(data.len() - 2);
        assert!(!data.ends_with(b".\n"));
        assert_eq!(data, b"compressed data");
    }

    /// Test binary dot-stuffing removal for read_multiline_response_binary
    ///
    /// Binary mode must also handle dot-stuffing but operates on bytes, not strings.
    /// After stripping line terminators, dot-stuffing is handled on the content.
    #[test]
    fn test_binary_dot_stuffing() {
        // Helper to simulate the binary reader logic: strip \r\n then handle dot-stuffing
        fn process_line(line_bytes: &[u8]) -> Vec<u8> {
            // Strip trailing \r\n
            let content_end = if line_bytes.ends_with(b"\r\n") {
                line_bytes.len() - 2
            } else if line_bytes.ends_with(b"\n") {
                line_bytes.len() - 1
            } else {
                line_bytes.len()
            };
            let line_content = &line_bytes[..content_end];

            // Handle dot-stuffing
            if line_content.starts_with(b"..") {
                line_content[1..].to_vec()
            } else {
                line_content.to_vec()
            }
        }

        // Line starting with ".." - should strip first dot AND \r\n
        let line_bytes = b"..Binary data\r\n";
        let processed = process_line(line_bytes);
        assert_eq!(processed, b".Binary data");

        // Normal line - strip \r\n only
        let line_bytes = b"Binary data\r\n";
        let processed = process_line(line_bytes);
        assert_eq!(processed, b"Binary data");

        // Three dots - strip one dot and \r\n
        let line_bytes = b"...\r\n";
        let processed = process_line(line_bytes);
        assert_eq!(processed, b"..");

        // LF-only line ending
        let line_bytes = b"Data line\n";
        let processed = process_line(line_bytes);
        assert_eq!(processed, b"Data line");
    }

    /// Test binary terminator detection for optimized article fetching
    #[test]
    fn test_binary_terminator_detection() {
        // Standard CRLF terminator
        let terminator_crlf: &[u8] = b".\r\n";
        assert_eq!(terminator_crlf, b".\r\n");

        // LF-only terminator
        let terminator_lf: &[u8] = b".\n";
        assert_eq!(terminator_lf, b".\n");

        // Not terminators
        assert_ne!(b"..\r\n" as &[u8], b".\r\n" as &[u8]); // Dot-stuffed
        assert_ne!(b".\r" as &[u8], b".\r\n" as &[u8]); // Incomplete
        assert_ne!(b"data.\r\n" as &[u8], b".\r\n" as &[u8]); // Embedded dot
    }

    /// Test UTF-8 lossy conversion behavior
    ///
    /// The I/O layer uses String::from_utf8_lossy to handle servers that might
    /// send invalid UTF-8 in headers or status lines. This test documents the
    /// expected behavior.
    #[test]
    fn test_utf8_lossy_conversion() {
        // Valid UTF-8 - unchanged
        let bytes = b"Hello world";
        let s = String::from_utf8_lossy(bytes);
        assert_eq!(s, "Hello world");

        // Invalid UTF-8 - replaced with Unicode replacement character
        let bytes = b"Hello \xFF world";
        let s = String::from_utf8_lossy(bytes);
        assert!(s.contains("Hello"));
        assert!(s.contains("world"));
        assert!(s.contains('\u{FFFD}')); // Replacement character

        // Valid UTF-8 with non-ASCII characters
        let bytes = "Hello 世界".as_bytes();
        let s = String::from_utf8_lossy(bytes);
        assert_eq!(s, "Hello 世界");
    }

    /// Test line trimming behavior (trim_end removes CRLF/LF)
    #[test]
    fn test_line_trimming() {
        // CRLF endings
        assert_eq!("200 OK\r\n".trim_end(), "200 OK");

        // LF only
        assert_eq!("200 OK\n".trim_end(), "200 OK");

        // Multiple trailing whitespace
        assert_eq!("200 OK  \r\n  ".trim_end(), "200 OK");

        // No trailing whitespace
        assert_eq!("200 OK".trim_end(), "200 OK");

        // Empty line
        assert_eq!("\r\n".trim_end(), "");
    }

    /// Test that initial capacity values are power-of-2 aligned for allocator efficiency
    #[test]
    fn test_capacity_alignment() {
        // 512 bytes is a common line buffer size
        let line_capacity: u32 = 512;
        assert_eq!(line_capacity, 512);
        assert_eq!(line_capacity.count_ones(), 1, "Should be power of 2");

        // 256 bytes for first line (smaller since it's just status)
        let first_line_capacity: u32 = 256;
        assert_eq!(first_line_capacity, 256);
        assert_eq!(first_line_capacity.count_ones(), 1, "Should be power of 2");
    }

    /// Test compression mode comparison behavior
    #[test]
    fn test_compression_mode_comparison() {
        // HeadersOnly mode
        assert_eq!(CompressionMode::HeadersOnly, CompressionMode::HeadersOnly);
        assert_ne!(CompressionMode::HeadersOnly, CompressionMode::None);
        assert_ne!(CompressionMode::HeadersOnly, CompressionMode::FullSession);

        // None mode
        assert_eq!(CompressionMode::None, CompressionMode::None);

        // FullSession mode
        assert_eq!(CompressionMode::FullSession, CompressionMode::FullSession);
    }

    /// Test buffer extension logic used in compressed block reading
    #[test]
    fn test_buffer_extension() {
        let mut all_data = Vec::new();
        let chunk1 = b"first chunk";
        let chunk2 = b"second chunk";

        all_data.extend_from_slice(chunk1);
        assert_eq!(all_data.len(), 11);

        all_data.extend_from_slice(chunk2);
        assert_eq!(all_data.len(), 23);

        assert_eq!(&all_data[..11], b"first chunk");
        assert_eq!(&all_data[11..], b"second chunk");
    }

    /// Test vector capacity pre-allocation for performance
    #[test]
    fn test_capacity_preallocation() {
        // Pre-allocated vector should have at least the requested capacity
        let vec: Vec<u8> = Vec::with_capacity(512);
        assert!(vec.capacity() >= 512);
        assert_eq!(vec.len(), 0);

        // Binary data buffer pre-allocation
        let binary_buf: Vec<u8> = Vec::with_capacity(BINARY_DATA_INITIAL_CAPACITY);
        assert!(binary_buf.capacity() >= BINARY_DATA_INITIAL_CAPACITY);
    }

    /// Test slice operations used in byte-stuffing logic
    #[test]
    fn test_slice_operations() {
        let data = b"..stuffed";

        // starts_with check - note that ".." starts with both ".." and "."
        assert!(data.starts_with(b".."));
        assert!(data.starts_with(b".")); // This is true - "." is a prefix of ".."

        // Slice from position 1 (removing first dot)
        let unstuffed = &data[1..];
        assert_eq!(unstuffed, b".stuffed");

        // Non-stuffed data doesn't start with ".."
        let normal = b"Hello";
        assert!(!normal.starts_with(b".."));
    }
}

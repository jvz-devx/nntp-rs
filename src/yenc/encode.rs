use crate::{NntpError, Result};
use crc32fast::Hasher;

/// Encode binary data to yEnc format
///
/// # Arguments
/// * `data` - Binary data to encode (the part data)
/// * `filename` - Original filename
/// * `line_length` - Maximum line length (default 128, max 997)
/// * `part_info` - Optional multi-part information (part number, total parts, begin offset, end offset, total file size)
///
/// # Returns
/// yEnc encoded data as bytes including =ybegin, data lines, and =yend
///
/// # Example
/// ```ignore
/// let data = b"Hello";
/// let encoded = encode(data, "test.bin", 128, None)?;
/// // Can round-trip: decode(encoded)? should give back original data
/// ```
pub fn encode(
    data: &[u8],
    filename: &str,
    line_length: usize,
    part_info: Option<(u32, u32, u64, u64, u64)>, // (part, total_parts, begin, end, total_file_size)
) -> Result<Vec<u8>> {
    // Validate line length
    if line_length == 0 || line_length > 997 {
        return Err(NntpError::InvalidResponse(format!(
            "Invalid line length: {} (must be 1-997)",
            line_length
        )));
    }

    let mut output = Vec::new();

    // Generate =ybegin header
    if let Some((part, total_parts, begin, end, total_file_size)) = part_info {
        // For multi-part files, size in =ybegin header is the TOTAL file size
        output.extend_from_slice(
            format!(
                "=ybegin part={} total={} line={} size={} name={}\r\n",
                part, total_parts, line_length, total_file_size, filename
            )
            .as_bytes(),
        );

        // Generate =ypart header for multi-part files
        output.extend_from_slice(format!("=ypart begin={} end={}\r\n", begin, end).as_bytes());
    } else {
        output.extend_from_slice(
            format!(
                "=ybegin line={} size={} name={}\r\n",
                line_length,
                data.len(),
                filename
            )
            .as_bytes(),
        );
    }

    // Encode data with line breaks
    let encoded_data = encode_data(data, line_length)?;
    output.extend_from_slice(&encoded_data);

    // Calculate CRC32
    let mut hasher = Hasher::new();
    hasher.update(data);
    let crc32 = hasher.finalize();

    // Generate =yend trailer
    if part_info.is_some() {
        // Multi-part: include pcrc32 (part CRC)
        output.extend_from_slice(
            format!("=yend size={} pcrc32={:08x}\r\n", data.len(), crc32).as_bytes(),
        );
    } else {
        // Single-part: include crc32
        output.extend_from_slice(
            format!("=yend size={} crc32={:08x}\r\n", data.len(), crc32).as_bytes(),
        );
    }

    Ok(output)
}

/// Encode binary data with proper escaping and line breaks
///
/// yEnc encoding: output = (input + 42) mod 256
/// Critical bytes that must be escaped:
/// - NUL (0x00)
/// - TAB (0x09) - at line start/end
/// - LF (0x0A)
/// - CR (0x0D)
/// - SPACE (0x20) - at line start/end
/// - '=' (0x3D)
///
/// Escape sequence: = followed by (byte + 64)
fn encode_data(data: &[u8], line_length: usize) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut current_line = Vec::new();

    for &byte in data {
        // Encode: (byte + 42) mod 256
        let encoded = byte.wrapping_add(42);

        // Check if this byte needs escaping
        let needs_escape = is_critical_byte(encoded)
            || (encoded == b'\t' && current_line.is_empty())
            || (encoded == b' ' && current_line.is_empty());

        if needs_escape {
            // Add escape sequence: = followed by (encoded + 64)
            let escaped = encoded.wrapping_add(64);

            // Check if adding escape sequence would exceed line length
            if current_line.len() + 2 > line_length {
                // Flush current line
                output.extend_from_slice(&current_line);
                output.extend_from_slice(b"\r\n");
                current_line.clear();
            }

            current_line.push(b'=');
            current_line.push(escaped);
        } else {
            // Check if we need to handle TAB or SPACE at end of line
            // We need to look ahead to see if this would be the last byte on the line
            let would_end_line = current_line.len() + 1 >= line_length;

            if would_end_line && (encoded == b'\t' || encoded == b' ') {
                // Escape TAB/SPACE at end of line
                if current_line.len() + 2 > line_length {
                    // Not enough room for escape sequence, flush line
                    output.extend_from_slice(&current_line);
                    output.extend_from_slice(b"\r\n");
                    current_line.clear();
                }

                let escaped = encoded.wrapping_add(64);
                current_line.push(b'=');
                current_line.push(escaped);
            } else {
                // Regular byte
                if current_line.len() >= line_length {
                    // Flush current line
                    output.extend_from_slice(&current_line);
                    output.extend_from_slice(b"\r\n");
                    current_line.clear();
                }

                current_line.push(encoded);
            }
        }
    }

    // Flush remaining line
    if !current_line.is_empty() {
        output.extend_from_slice(&current_line);
        output.extend_from_slice(b"\r\n");
    }

    Ok(output)
}

/// Check if a byte is a critical byte that must always be escaped
fn is_critical_byte(byte: u8) -> bool {
    matches!(
        byte,
        0x00 |  // NUL
        0x0A |  // LF
        0x0D |  // CR
        0x3D // '='
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::yenc::decode::decode;

    #[test]
    fn test_encode_simple() {
        let data = b"Test";
        let encoded = encode(data, "test.txt", 128, None).unwrap();

        // Verify header (check as bytes since encoded data may not be valid UTF-8)
        assert!(encoded.starts_with(b"=ybegin line=128 size=4 name=test.txt\r\n"));

        // Verify trailer is present (search in bytes)
        let encoded_str = String::from_utf8_lossy(&encoded);
        assert!(encoded_str.contains("=yend size=4 crc32="));

        // Verify round-trip
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded.data, data);
        assert!(decoded.verify_crc32());
    }

    #[test]
    fn test_encode_empty() {
        let data = b"";
        let encoded = encode(data, "empty.bin", 128, None).unwrap();

        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded.data, data);
        assert_eq!(decoded.data.len(), 0);
    }

    #[test]
    fn test_encode_with_critical_bytes() {
        // Test data with NUL, LF, CR, and '=' characters
        let data = b"\x00\n\r=";
        let encoded = encode(data, "critical.bin", 128, None).unwrap();

        // Verify round-trip
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded.data, data);
        assert!(decoded.verify_crc32());
    }

    #[test]
    fn test_encode_with_tab_and_space() {
        // Test TAB and SPACE at various positions
        let data = b"\tHello \t World \t";
        let encoded = encode(data, "whitespace.txt", 128, None).unwrap();

        // Verify round-trip
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded.data, data);
        assert!(decoded.verify_crc32());
    }

    #[test]
    fn test_encode_multipart() {
        // Test multi-part encoding
        let data = b"Part 1 data here";
        let total_file_size = 768000; // Entire file size
                                      // Format: (part, total_parts, begin, end, total_file_size)
        let encoded = encode(
            data,
            "file.rar",
            128,
            Some((1, 3, 1, 384000, total_file_size)),
        )
        .unwrap();

        let encoded_str = String::from_utf8_lossy(&encoded);
        assert!(encoded_str.contains("=ybegin part=1 total=3"));
        assert!(encoded_str.contains(&format!("size={}", total_file_size)));
        assert!(encoded_str.contains("=ypart begin=1 end=384000"));
        assert!(encoded_str.contains("pcrc32=")); // multi-part uses pcrc32

        // Verify round-trip
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded.data, data);
        assert!(decoded.is_multipart());
        assert_eq!(decoded.header.part, Some(1));
        assert_eq!(decoded.header.total, Some(3));
    }

    #[test]
    fn test_encode_line_length() {
        // Test custom line length
        let data = b"This is a longer test string to verify line breaks work correctly with custom lengths";
        let encoded = encode(data, "test.txt", 20, None).unwrap();

        // Verify lines are not longer than specified (excluding \r\n and header/trailer)
        // Split by \n and check each line
        let mut start = 0;
        for (i, &byte) in encoded.iter().enumerate() {
            if byte == b'\n' {
                let line = &encoded[start..i];
                let line_without_cr = if line.ends_with(b"\r") {
                    &line[..line.len() - 1]
                } else {
                    line
                };

                // Skip header/trailer lines (start with =)
                if !line_without_cr.is_empty() && line_without_cr[0] != b'=' {
                    assert!(
                        line_without_cr.len() <= 20,
                        "Line too long: {} bytes",
                        line_without_cr.len()
                    );
                }

                start = i + 1;
            }
        }

        // Verify round-trip
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded.data, data);
        assert!(decoded.verify_crc32());
    }

    #[test]
    fn test_encode_invalid_line_length() {
        let data = b"test";

        // Line length 0 should fail
        assert!(encode(data, "test.txt", 0, None).is_err());

        // Line length > 997 should fail
        assert!(encode(data, "test.txt", 1000, None).is_err());
    }

    #[test]
    fn test_encode_large_data() {
        // Test with larger data (1KB)
        let data: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();
        let encoded = encode(&data, "large.bin", 128, None).unwrap();

        // Verify round-trip
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded.data, data);
        assert!(decoded.verify_crc32());
    }

    #[test]
    fn test_encode_all_bytes() {
        // Test encoding all possible byte values
        let data: Vec<u8> = (0..=255).collect();
        let encoded = encode(&data, "allbytes.bin", 128, None).unwrap();

        // Verify round-trip
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded.data, data);
        assert!(decoded.verify_crc32());
    }

    #[test]
    fn test_encode_decode_round_trip() {
        // Test various data patterns for round-trip encoding/decoding
        let test_cases = vec![
            b"Hello, World!".to_vec(),
            b"\x00\x01\x02\x03\x04\x05".to_vec(),
            b"Line\nBreak\rTest\r\n".to_vec(),
            b"Equals=Sign=Test".to_vec(),
            vec![0xDE, 0xAD, 0xBE, 0xEF],
            (0..255).collect::<Vec<u8>>(),
        ];

        for data in test_cases {
            let encoded = encode(&data, "test.bin", 128, None).unwrap();
            let decoded = decode(&encoded).unwrap();
            assert_eq!(decoded.data, data, "Round-trip failed for data: {:?}", data);
            assert!(decoded.verify_crc32(), "CRC32 verification failed");
        }
    }

    #[test]
    fn test_is_critical_byte() {
        assert!(is_critical_byte(0x00)); // NUL
        assert!(is_critical_byte(0x0A)); // LF
        assert!(is_critical_byte(0x0D)); // CR
        assert!(is_critical_byte(0x3D)); // '='

        assert!(!is_critical_byte(0x09)); // TAB (not always critical)
        assert!(!is_critical_byte(0x20)); // SPACE (not always critical)
        assert!(!is_critical_byte(b'A')); // Regular character
    }
}

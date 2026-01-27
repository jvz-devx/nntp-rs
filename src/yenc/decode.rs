use crate::{NntpError, Result};
use crc32fast::Hasher;

use super::params::{parse_ybegin, parse_yend, parse_ypart};
use super::types::YencDecoded;

/// Decode yEnc encoded data
///
/// # Arguments
/// * `input` - yEnc encoded data as bytes including =ybegin, data lines, and =yend
///
/// # Returns
/// Decoded binary data with header/trailer information
///
/// # Example
/// ```ignore
/// let encoded = b"=ybegin line=128 size=5 name=test.bin\r\n\
///                 Hello\r\n\
///                 =yend size=5 crc32=abcdef12\r\n";
/// let decoded = decode(encoded)?;
/// assert_eq!(decoded.data, b"Hello");
/// ```
pub fn decode(input: &[u8]) -> Result<YencDecoded> {
    // Split into lines by looking for LF
    let mut lines = Vec::new();
    let mut start = 0;

    for (i, &byte) in input.iter().enumerate() {
        if byte == b'\n' {
            lines.push(&input[start..i]);
            start = i + 1;
        }
    }

    // Add last line if it doesn't end with \n
    if start < input.len() {
        lines.push(&input[start..]);
    }

    if lines.is_empty() {
        return Err(NntpError::InvalidResponse("Empty yEnc input".to_string()));
    }

    // Parse header (must be ASCII)
    let header_str = std::str::from_utf8(lines[0])
        .map_err(|_| NntpError::InvalidResponse("Invalid UTF-8 in header".to_string()))?
        .trim_end_matches('\r');
    let header = parse_ybegin(header_str)?;

    // Check for multi-part
    let mut part = None;
    let mut data_start = 1;

    if lines.len() > 1
        && let Ok(line_str) = std::str::from_utf8(lines[1])
        && line_str.starts_with("=ypart ")
    {
        part = Some(parse_ypart(line_str.trim_end_matches('\r'))?);
        data_start = 2;
    }

    // Find trailer line (=yend)
    let trailer_idx = lines
        .iter()
        .rposition(|line| {
            std::str::from_utf8(line)
                .map(|s| s.starts_with("=yend "))
                .unwrap_or(false)
        })
        .ok_or_else(|| NntpError::InvalidResponse("Missing =yend trailer".to_string()))?;

    // Parse trailer (must be ASCII)
    let trailer_str = std::str::from_utf8(lines[trailer_idx])
        .map_err(|_| NntpError::InvalidResponse("Invalid UTF-8 in trailer".to_string()))?
        .trim_end_matches('\r');
    let trailer = parse_yend(trailer_str)?;

    // Decode data lines
    let mut decoded = Vec::with_capacity(trailer.size as usize);
    let mut hasher = Hasher::new();

    for line in &lines[data_start..trailer_idx] {
        decode_line_bytes(line, &mut decoded)?;
    }

    // Calculate CRC32
    hasher.update(&decoded);
    let calculated_crc32 = hasher.finalize();

    Ok(YencDecoded {
        header,
        part,
        trailer,
        data: decoded,
        calculated_crc32,
    })
}

/// Decode a single yEnc encoded line (bytes version)
///
/// yEnc encoding: output = (input + 42) mod 256
/// yEnc decoding: output = (input - 42) mod 256
///
/// Escape sequences: =X means (X - 64 - 42) mod 256
/// Critical escapes: NUL(0), TAB(9), LF(10), CR(13), SPACE(32), '='(61)
fn decode_line_bytes(line: &[u8], output: &mut Vec<u8>) -> Result<()> {
    let mut i = 0;

    while i < line.len() {
        let byte = line[i];

        // Skip CR if it appears at end of line
        if byte == b'\r' {
            i += 1;
            continue;
        }

        if byte == b'=' {
            // Escape sequence
            if i + 1 >= line.len() {
                return Err(NntpError::InvalidResponse(
                    "Incomplete escape sequence at end of line".to_string(),
                ));
            }

            i += 1;
            let escaped = line[i];

            // Decode: (escaped - 64 - 42) mod 256
            let decoded = escaped.wrapping_sub(64).wrapping_sub(42);
            output.push(decoded);
        } else {
            // Regular byte: decode as (byte - 42) mod 256
            let decoded = byte.wrapping_sub(42);
            output.push(decoded);
        }

        i += 1;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Convenience wrapper for tests: decode a yEnc line from a string.
    fn decode_line(line: &str, output: &mut Vec<u8>) -> Result<()> {
        decode_line_bytes(line.as_bytes(), output)
    }

    #[test]
    fn test_decode_simple() {
        // "Test" encoded: T(84) e(101) s(115) t(116)
        // Encoding: (byte + 42) mod 256
        // T: (84 + 42) = 126 = ~
        // e: (101 + 42) = 143
        // s: (115 + 42) = 157
        // t: (116 + 42) = 158
        // Build the input as bytes to avoid UTF-8 issues
        let mut input_bytes = Vec::new();
        input_bytes.extend_from_slice(b"=ybegin line=128 size=4 name=test.txt\n");
        input_bytes.push(126); // ~
        input_bytes.push(143);
        input_bytes.push(157);
        input_bytes.push(158);
        input_bytes.push(b'\n');
        input_bytes.extend_from_slice(b"=yend size=4 crc32=0e7e1273\n");

        let result = decode(&input_bytes).unwrap();
        assert_eq!(result.data, b"Test");
        assert_eq!(result.header.name, "test.txt");
        assert_eq!(result.header.size, 4);
        assert_eq!(result.trailer.size, 4);
    }

    #[test]
    fn test_decode_with_escape() {
        // Byte 214 encodes to 0 (NULL), which is critical and must be escaped
        // Encoding byte 214: (214 + 42) mod 256 = 0
        // Since 0 is critical, escape: = + (0 + 64) = =@ (@ = 64)
        // Decoding =@: (64 - 64 - 42) mod 256 = wrapping_sub gives 214 ✓
        let input = b"=ybegin line=128 size=1 name=test.bin\n\
                      =@\n\
                      =yend size=1\n";

        let result = decode(input).unwrap();
        assert_eq!(result.data, b"\xd6"); // 214 = 0xd6
    }

    #[test]
    fn test_decode_multipart() {
        let input = b"=ybegin part=1 total=2 line=128 size=768000 name=file.rar\n\
                      =ypart begin=1 end=384000\n\
                      test_data_here\n\
                      =yend size=384000 pcrc32=abcd1234\n";

        let result = decode(input).unwrap();
        assert!(result.is_multipart());
        assert_eq!(result.header.part, Some(1));
        assert_eq!(result.header.total, Some(2));
        assert_eq!(result.part.as_ref().unwrap().begin, 1);
        assert_eq!(result.part.as_ref().unwrap().end, 384000);
    }

    #[test]
    fn test_decode_line_basic() {
        let mut output = Vec::new();
        // "A" = 65, encoded: (65 + 42) = 107 = 'k'
        decode_line("k", &mut output).unwrap();
        assert_eq!(output, b"A");
    }

    #[test]
    fn test_decode_line_with_escape() {
        let mut output = Vec::new();
        // Null byte: =@ (@ = 64, decoded: 64 - 64 - 42 = -42 mod 256 = 214... wait)
        // Actually: @ = 64, (64 - 64 - 42) mod 256 = -42 mod 256 = 256 - 42 = 214
        // That's wrong. Let me recalculate:
        // To encode 0x00: (0 + 42) = 42, then escape: 42 + 64 = 106 = 'j'
        // To decode =j: (106 - 64 - 42) = 0
        decode_line("=j", &mut output).unwrap();
        assert_eq!(output, b"\x00");
    }
}

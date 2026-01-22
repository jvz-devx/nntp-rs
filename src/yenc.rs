//! yEnc binary encoding/decoding for Usenet
//!
//! yEnc is a binary-to-text encoding scheme designed specifically for Usenet.
//! It has only 1-2% overhead compared to 33-40% for Base64.
//!
//! Reference: http://www.yenc.org/yenc-draft.1.3.txt

use crate::{NntpError, Result};
use crc32fast::Hasher;
use std::collections::HashMap;

/// yEnc header from =ybegin line
#[derive(Debug, Clone, PartialEq)]
pub struct YencHeader {
    /// Line length (typically 128, max 997)
    pub line: usize,
    /// Total file size in bytes
    pub size: u64,
    /// Original filename
    pub name: String,
    /// Part number (for multi-part files)
    pub part: Option<u32>,
    /// Total number of parts (for multi-part files)
    pub total: Option<u32>,
}

/// yEnc part header from =ypart line (for multi-part files)
#[derive(Debug, Clone, PartialEq)]
pub struct YencPart {
    /// Byte offset where this part begins in the original file
    pub begin: u64,
    /// Byte offset where this part ends in the original file
    pub end: u64,
}

/// yEnc trailer from =yend line
#[derive(Debug, Clone, PartialEq)]
pub struct YencEnd {
    /// Size of decoded data in bytes
    pub size: u64,
    /// CRC32 of entire decoded file (for single-part) or this part (for multi-part)
    pub crc32: Option<u32>,
    /// CRC32 of this part only (for multi-part files)
    pub pcrc32: Option<u32>,
}

/// Complete yEnc decoded result
#[derive(Debug, Clone)]
pub struct YencDecoded {
    /// Parsed header information
    pub header: YencHeader,
    /// Part information (for multi-part files)
    pub part: Option<YencPart>,
    /// Trailer information
    pub trailer: YencEnd,
    /// Decoded binary data
    pub data: Vec<u8>,
    /// Calculated CRC32 of decoded data
    pub calculated_crc32: u32,
}

impl YencDecoded {
    /// Verify CRC32 matches expected value
    pub fn verify_crc32(&self) -> bool {
        // For multi-part files, check pcrc32 (part CRC)
        if let Some(expected) = self.trailer.pcrc32 {
            return self.calculated_crc32 == expected;
        }
        // For single-part files, check crc32
        if let Some(expected) = self.trailer.crc32 {
            return self.calculated_crc32 == expected;
        }
        // No CRC to verify
        false
    }

    /// Check if this is a multi-part file
    pub fn is_multipart(&self) -> bool {
        self.header.part.is_some() && self.header.total.is_some()
    }
}

/// Parse yEnc =ybegin header line
///
/// Format: =ybegin line=128 size=123456 name=file.bin [part=1 total=5]
fn parse_ybegin(line: &str) -> Result<YencHeader> {
    if !line.starts_with("=ybegin ") {
        return Err(NntpError::InvalidResponse(format!(
            "Invalid yEnc header: {}",
            line
        )));
    }

    let params = parse_yenc_params(&line[8..])?;

    let line_len = params
        .get("line")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| NntpError::InvalidResponse("Missing 'line' parameter".to_string()))?;

    let size = params
        .get("size")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| NntpError::InvalidResponse("Missing 'size' parameter".to_string()))?;

    let name = params
        .get("name")
        .ok_or_else(|| NntpError::InvalidResponse("Missing 'name' parameter".to_string()))?
        .to_string();

    let part = params.get("part").and_then(|s| s.parse().ok());
    let total = params.get("total").and_then(|s| s.parse().ok());

    Ok(YencHeader {
        line: line_len,
        size,
        name,
        part,
        total,
    })
}

/// Parse yEnc =ypart line
///
/// Format: =ypart begin=1 end=123456
fn parse_ypart(line: &str) -> Result<YencPart> {
    if !line.starts_with("=ypart ") {
        return Err(NntpError::InvalidResponse(format!(
            "Invalid yEnc part header: {}",
            line
        )));
    }

    let params = parse_yenc_params(&line[7..])?;

    let begin = params
        .get("begin")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| NntpError::InvalidResponse("Missing 'begin' parameter".to_string()))?;

    let end = params
        .get("end")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| NntpError::InvalidResponse("Missing 'end' parameter".to_string()))?;

    Ok(YencPart { begin, end })
}

/// Parse yEnc =yend line
///
/// Format: =yend size=123456 [crc32=12345678] [pcrc32=87654321]
fn parse_yend(line: &str) -> Result<YencEnd> {
    if !line.starts_with("=yend ") {
        return Err(NntpError::InvalidResponse(format!(
            "Invalid yEnc trailer: {}",
            line
        )));
    }

    let params = parse_yenc_params(&line[6..])?;

    let size = params
        .get("size")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| NntpError::InvalidResponse("Missing 'size' parameter".to_string()))?;

    let crc32 = params.get("crc32").and_then(|s| {
        // CRC32 is in hex format
        u32::from_str_radix(s, 16).ok()
    });

    let pcrc32 = params
        .get("pcrc32")
        .and_then(|s| u32::from_str_radix(s, 16).ok());

    Ok(YencEnd {
        size,
        crc32,
        pcrc32,
    })
}

/// Parse yEnc key=value parameters
fn parse_yenc_params(params: &str) -> Result<HashMap<String, String>> {
    let mut result = HashMap::new();
    let mut chars = params.chars().peekable();

    while chars.peek().is_some() {
        // Skip whitespace
        while chars.peek() == Some(&' ') {
            chars.next();
        }

        if chars.peek().is_none() {
            break;
        }

        // Parse key
        let mut key = String::new();
        while let Some(&ch) = chars.peek() {
            if ch == '=' {
                chars.next(); // consume '='
                break;
            }
            key.push(ch);
            chars.next();
        }

        if key.is_empty() {
            break;
        }

        // Parse value (everything until next space or end)
        let mut value = String::new();
        while let Some(&ch) = chars.peek() {
            if ch == ' ' {
                break;
            }
            value.push(ch);
            chars.next();
        }

        result.insert(key, value);
    }

    Ok(result)
}

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

    if lines.len() > 1 {
        if let Ok(line_str) = std::str::from_utf8(lines[1]) {
            if line_str.starts_with("=ypart ") {
                part = Some(parse_ypart(line_str.trim_end_matches('\r'))?);
                data_start = 2;
            }
        }
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

/// Decode a single yEnc encoded line (string version, for backwards compatibility)
#[allow(dead_code)]
fn decode_line(line: &str, output: &mut Vec<u8>) -> Result<()> {
    decode_line_bytes(line.as_bytes(), output)
}

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

/// Multi-part yEnc file assembler
///
/// Collects and assembles multi-part yEnc encoded files into a single file.
/// Validates part ranges don't overlap and verifies the final CRC32.
///
/// # Example
/// ```ignore
/// let mut assembler = YencMultipartAssembler::new();
///
/// // Add parts as they're received
/// assembler.add_part(decoded_part1)?;
/// assembler.add_part(decoded_part2)?;
///
/// // Check if complete and assemble
/// if assembler.is_complete() {
///     let final_data = assembler.assemble()?;
///     assert!(assembler.verify_final_crc32(&final_data));
/// }
/// ```
#[derive(Debug, Clone)]
pub struct YencMultipartAssembler {
    /// Expected total number of parts
    total_parts: Option<u32>,
    /// Expected total file size
    total_size: Option<u64>,
    /// Expected filename
    filename: Option<String>,
    /// Expected final CRC32 (from header)
    expected_crc32: Option<u32>,
    /// Collected parts indexed by part number
    parts: std::collections::HashMap<u32, YencDecoded>,
}

impl YencMultipartAssembler {
    /// Create a new multi-part assembler
    pub fn new() -> Self {
        Self {
            total_parts: None,
            total_size: None,
            filename: None,
            expected_crc32: None,
            parts: std::collections::HashMap::new(),
        }
    }

    /// Add a decoded part to the assembler
    ///
    /// # Errors
    /// Returns an error if:
    /// - The part is not a multi-part file
    /// - The part overlaps with an existing part
    /// - The part has inconsistent metadata
    pub fn add_part(&mut self, decoded: YencDecoded) -> Result<()> {
        // Validate this is a multi-part file
        if !decoded.is_multipart() {
            return Err(NntpError::InvalidResponse(
                "Cannot add single-part file to multi-part assembler".to_string(),
            ));
        }

        let part_num = decoded.header.part.unwrap();
        let total = decoded.header.total.unwrap();

        // Initialize or validate metadata
        // Note: decoded.header.size is the TOTAL file size, not the part size
        if self.total_parts.is_none() {
            self.total_parts = Some(total);
            self.total_size = Some(decoded.header.size);
            self.filename = Some(decoded.header.name.clone());
            // The crc32 from header is the full file CRC32 (only present in some implementations)
            // For multi-part files, we typically don't have the full file CRC32 in individual parts
            if decoded.trailer.crc32.is_some() {
                self.expected_crc32 = decoded.trailer.crc32;
            }
        } else {
            // Validate consistency
            if self.total_parts != Some(total) {
                return Err(NntpError::InvalidResponse(format!(
                    "Inconsistent total parts: expected {}, got {}",
                    self.total_parts.unwrap(),
                    total
                )));
            }
            if self.total_size != Some(decoded.header.size) {
                return Err(NntpError::InvalidResponse(format!(
                    "Inconsistent total size: expected {}, got {}",
                    self.total_size.unwrap(),
                    decoded.header.size
                )));
            }
            if self.filename.as_ref() != Some(&decoded.header.name) {
                return Err(NntpError::InvalidResponse(format!(
                    "Inconsistent filename: expected {}, got {}",
                    self.filename.as_ref().unwrap(),
                    decoded.header.name
                )));
            }
        }

        // Validate part CRC32
        if !decoded.verify_crc32() {
            return Err(NntpError::InvalidResponse(format!(
                "Part {} CRC32 verification failed",
                part_num
            )));
        }

        // Check for overlapping ranges
        if let Some(part_info) = &decoded.part {
            for (existing_num, existing) in &self.parts {
                if let Some(existing_info) = &existing.part {
                    // Check if ranges overlap
                    let overlaps = !(part_info.end < existing_info.begin
                        || part_info.begin > existing_info.end);
                    if overlaps {
                        return Err(NntpError::InvalidResponse(format!(
                            "Part {} range ({}-{}) overlaps with part {} range ({}-{})",
                            part_num,
                            part_info.begin,
                            part_info.end,
                            existing_num,
                            existing_info.begin,
                            existing_info.end
                        )));
                    }
                }
            }
        }

        // Check if part already exists
        if self.parts.contains_key(&part_num) {
            return Err(NntpError::InvalidResponse(format!(
                "Part {} already added",
                part_num
            )));
        }

        // Add the part
        self.parts.insert(part_num, decoded);

        Ok(())
    }

    /// Check if all parts have been received
    pub fn is_complete(&self) -> bool {
        if let Some(total) = self.total_parts {
            self.parts.len() == total as usize
        } else {
            false
        }
    }

    /// Get the number of parts received
    pub fn parts_received(&self) -> usize {
        self.parts.len()
    }

    /// Get the total number of expected parts
    pub fn total_parts(&self) -> Option<u32> {
        self.total_parts
    }

    /// Get list of missing part numbers
    pub fn missing_parts(&self) -> Vec<u32> {
        if let Some(total) = self.total_parts {
            (1..=total)
                .filter(|n| !self.parts.contains_key(n))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Assemble all parts into final file data
    ///
    /// # Errors
    /// Returns an error if:
    /// - Not all parts have been received
    /// - Parts have gaps or overlaps
    pub fn assemble(&self) -> Result<Vec<u8>> {
        if !self.is_complete() {
            return Err(NntpError::InvalidResponse(format!(
                "Cannot assemble: missing {} parts",
                self.total_parts.unwrap_or(0) as usize - self.parts.len()
            )));
        }

        let total_size = self
            .total_size
            .ok_or_else(|| NntpError::InvalidResponse("No parts added yet".to_string()))?;

        let mut result = vec![0u8; total_size as usize];

        // Sort parts by part number and assemble
        let mut sorted_parts: Vec<_> = self.parts.iter().collect();
        sorted_parts.sort_by_key(|(num, _)| *num);

        for (_part_num, decoded) in sorted_parts {
            if let Some(part_info) = &decoded.part {
                // yEnc uses 1-based offsets
                let begin = (part_info.begin - 1) as usize;
                let end = part_info.end as usize;

                // Validate range
                if end > total_size as usize {
                    return Err(NntpError::InvalidResponse(format!(
                        "Part range {}-{} exceeds total size {}",
                        part_info.begin, part_info.end, total_size
                    )));
                }

                let expected_len = end - begin;
                if decoded.data.len() != expected_len {
                    return Err(NntpError::InvalidResponse(format!(
                        "Part data length {} doesn't match range {}-{} (expected {})",
                        decoded.data.len(),
                        part_info.begin,
                        part_info.end,
                        expected_len
                    )));
                }

                // Copy data to correct offset
                result[begin..end].copy_from_slice(&decoded.data);
            } else {
                return Err(NntpError::InvalidResponse(
                    "Part missing part info".to_string(),
                ));
            }
        }

        Ok(result)
    }

    /// Verify the final CRC32 of assembled data
    pub fn verify_final_crc32(&self, data: &[u8]) -> bool {
        if let Some(expected) = self.expected_crc32 {
            let mut hasher = Hasher::new();
            hasher.update(data);
            let calculated = hasher.finalize();
            calculated == expected
        } else {
            // No expected CRC32 to verify against
            false
        }
    }

    /// Get expected filename
    pub fn filename(&self) -> Option<&str> {
        self.filename.as_deref()
    }

    /// Get expected total size
    pub fn expected_size(&self) -> Option<u64> {
        self.total_size
    }
}

impl Default for YencMultipartAssembler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ybegin_simple() {
        let line = "=ybegin line=128 size=123456 name=test.bin";
        let header = parse_ybegin(line).unwrap();

        assert_eq!(header.line, 128);
        assert_eq!(header.size, 123456);
        assert_eq!(header.name, "test.bin");
        assert_eq!(header.part, None);
        assert_eq!(header.total, None);
    }

    #[test]
    fn test_parse_ybegin_multipart() {
        let line = "=ybegin part=1 total=5 line=128 size=123456 name=file.rar";
        let header = parse_ybegin(line).unwrap();

        assert_eq!(header.line, 128);
        assert_eq!(header.size, 123456);
        assert_eq!(header.name, "file.rar");
        assert_eq!(header.part, Some(1));
        assert_eq!(header.total, Some(5));
    }

    #[test]
    fn test_parse_ypart() {
        let line = "=ypart begin=1 end=384000";
        let part = parse_ypart(line).unwrap();

        assert_eq!(part.begin, 1);
        assert_eq!(part.end, 384000);
    }

    #[test]
    fn test_parse_yend() {
        let line = "=yend size=384000 pcrc32=12345678";
        let end = parse_yend(line).unwrap();

        assert_eq!(end.size, 384000);
        assert_eq!(end.pcrc32, Some(0x12345678));
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

    #[test]
    fn test_parse_yenc_params() {
        let params = parse_yenc_params("line=128 size=123456 name=test.bin").unwrap();
        assert_eq!(params.get("line"), Some(&"128".to_string()));
        assert_eq!(params.get("size"), Some(&"123456".to_string()));
        assert_eq!(params.get("name"), Some(&"test.bin".to_string()));
    }

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

    #[test]
    fn test_assembler_two_parts() {
        // Create a file with two parts
        let full_data = b"Hello World! This is a test of multi-part yEnc files.";
        let total_size = full_data.len() as u64;

        let part1_data = &full_data[0..28]; // First 28 bytes
        let part2_data = &full_data[28..]; // Remaining bytes

        // Encode each part with correct offsets (yEnc uses 1-based indexing)
        // Format: (part, total_parts, begin, end, total_file_size)
        let part1 = encode(part1_data, "test.txt", 128, Some((1, 2, 1, 28, total_size))).unwrap();
        let part2 = encode(
            part2_data,
            "test.txt",
            128,
            Some((2, 2, 29, total_size, total_size)),
        )
        .unwrap();

        // Decode parts
        let decoded1 = decode(&part1).unwrap();
        let decoded2 = decode(&part2).unwrap();

        // Assemble
        let mut assembler = YencMultipartAssembler::new();
        assembler.add_part(decoded1).unwrap();
        assert_eq!(assembler.parts_received(), 1);
        assert!(!assembler.is_complete());
        assert_eq!(assembler.missing_parts(), vec![2]);

        assembler.add_part(decoded2).unwrap();
        assert_eq!(assembler.parts_received(), 2);
        assert!(assembler.is_complete());
        assert!(assembler.missing_parts().is_empty());

        let assembled = assembler.assemble().unwrap();
        assert_eq!(assembled, full_data);
    }

    #[test]
    fn test_assembler_three_parts_out_of_order() {
        // Create a file with three parts
        let full_data = b"Part1Part2Part3";
        let total_size = full_data.len() as u64; // 15 bytes
        let part1_data = &full_data[0..5]; // "Part1"
        let part2_data = &full_data[5..10]; // "Part2"
        let part3_data = &full_data[10..15]; // "Part3"

        // Encode each part - ALL parts must have the same total_size in their headers
        // Format: (part, total_parts, begin, end, total_file_size)
        let part1 = encode(part1_data, "file.bin", 128, Some((1, 3, 1, 5, total_size))).unwrap();
        let part2 = encode(part2_data, "file.bin", 128, Some((2, 3, 6, 10, total_size))).unwrap();
        let part3 = encode(
            part3_data,
            "file.bin",
            128,
            Some((3, 3, 11, 15, total_size)),
        )
        .unwrap();

        // Decode parts
        let decoded1 = decode(&part1).unwrap();
        let decoded2 = decode(&part2).unwrap();
        let decoded3 = decode(&part3).unwrap();

        // Add parts out of order
        let mut assembler = YencMultipartAssembler::new();
        assembler.add_part(decoded2).unwrap(); // Add part 2 first
        assembler.add_part(decoded3).unwrap(); // Then part 3
        assembler.add_part(decoded1).unwrap(); // Then part 1

        assert!(assembler.is_complete());
        let assembled = assembler.assemble().unwrap();
        assert_eq!(assembled, full_data);
    }

    #[test]
    fn test_assembler_missing_parts() {
        let data = b"Test";
        let total_size = 12; // Assume 3 parts of 4 bytes each
                             // Format: (part, total_parts, begin, end, total_file_size)
        let part1 = encode(data, "test.bin", 128, Some((1, 3, 1, 4, total_size))).unwrap();
        let decoded1 = decode(&part1).unwrap();

        let mut assembler = YencMultipartAssembler::new();
        assembler.add_part(decoded1).unwrap();

        assert!(!assembler.is_complete());
        assert_eq!(assembler.total_parts(), Some(3));
        assert_eq!(assembler.missing_parts(), vec![2, 3]);

        // Try to assemble before complete
        assert!(assembler.assemble().is_err());
    }

    #[test]
    fn test_assembler_duplicate_part() {
        let data = b"Test";
        let total_size = 8; // 2 parts of 4 bytes each
                            // Format: (part, total_parts, begin, end, total_file_size)
        let part1 = encode(data, "test.bin", 128, Some((1, 2, 1, 4, total_size))).unwrap();
        let decoded1 = decode(&part1).unwrap();
        let decoded1_dup = decode(&part1).unwrap();

        let mut assembler = YencMultipartAssembler::new();
        assembler.add_part(decoded1).unwrap();

        // Try to add same part again
        let result = assembler.add_part(decoded1_dup);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        // Can be rejected for either duplicate part or overlapping range
        assert!(
            err.contains("Part 1 already added")
                || err.contains("CRC32 verification failed")
                || err.contains("overlaps"),
            "Unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_assembler_overlapping_ranges() {
        // Create two parts with overlapping ranges
        let data1 = b"Test1";
        let data2 = b"Test2";

        // Part 1: bytes 1-5, Part 2: bytes 3-7 (overlap!)
        // Total size would be 7
        // Format: (part, total_parts, begin, end, total_file_size)
        let part1 = encode(data1, "test.bin", 128, Some((1, 2, 1, 5, 7))).unwrap();
        let part2 = encode(data2, "test.bin", 128, Some((2, 2, 3, 7, 7))).unwrap();

        let decoded1 = decode(&part1).unwrap();
        let decoded2 = decode(&part2).unwrap();

        let mut assembler = YencMultipartAssembler::new();
        assembler.add_part(decoded1).unwrap();

        // Try to add overlapping part
        let result = assembler.add_part(decoded2);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("overlaps"));
    }

    #[test]
    fn test_assembler_inconsistent_metadata() {
        let data = b"Test";

        // Create parts with inconsistent total counts
        // Format: (part, total_parts, begin, end, total_file_size)
        let part1 = encode(data, "test.bin", 128, Some((1, 2, 1, 4, 8))).unwrap();
        let part2 = encode(data, "test.bin", 128, Some((2, 3, 5, 8, 8))).unwrap(); // Wrong total parts (3 vs 2)!

        let decoded1 = decode(&part1).unwrap();
        let decoded2 = decode(&part2).unwrap();

        let mut assembler = YencMultipartAssembler::new();
        assembler.add_part(decoded1).unwrap();

        // Try to add part with inconsistent metadata
        let result = assembler.add_part(decoded2);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Inconsistent total parts"));
    }

    #[test]
    fn test_assembler_single_part_rejected() {
        let data = b"Test";
        let single = encode(data, "test.txt", 128, None).unwrap();
        let decoded = decode(&single).unwrap();

        let mut assembler = YencMultipartAssembler::new();
        let result = assembler.add_part(decoded);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot add single-part file"));
    }

    #[test]
    fn test_assembler_getters() {
        let data = b"Hello";
        let total_size = 15; // 3 parts of 5 bytes each
                             // Format: (part, total_parts, begin, end, total_file_size)
        let part1 = encode(data, "test.bin", 128, Some((1, 3, 1, 5, total_size))).unwrap();
        let decoded1 = decode(&part1).unwrap();

        let mut assembler = YencMultipartAssembler::new();
        assembler.add_part(decoded1).unwrap();

        assert_eq!(assembler.filename(), Some("test.bin"));
        assert_eq!(assembler.expected_size(), Some(total_size));
        assert_eq!(assembler.total_parts(), Some(3));
        assert_eq!(assembler.parts_received(), 1);
    }

    #[test]
    fn test_assembler_large_multipart() {
        // Test with larger data split into multiple parts
        let full_data: Vec<u8> = (0..=255).cycle().take(1000).collect();
        let total_size = full_data.len() as u64; // 1000 bytes

        // Split into 4 parts
        let part1_data = &full_data[0..250];
        let part2_data = &full_data[250..500];
        let part3_data = &full_data[500..750];
        let part4_data = &full_data[750..1000];

        // ALL parts must have the same total_size in headers
        // Format: (part, total_parts, begin, end, total_file_size)
        let part1 = encode(
            part1_data,
            "large.bin",
            128,
            Some((1, 4, 1, 250, total_size)),
        )
        .unwrap();
        let part2 = encode(
            part2_data,
            "large.bin",
            128,
            Some((2, 4, 251, 500, total_size)),
        )
        .unwrap();
        let part3 = encode(
            part3_data,
            "large.bin",
            128,
            Some((3, 4, 501, 750, total_size)),
        )
        .unwrap();
        let part4 = encode(
            part4_data,
            "large.bin",
            128,
            Some((4, 4, 751, 1000, total_size)),
        )
        .unwrap();

        let decoded1 = decode(&part1).unwrap();
        let decoded2 = decode(&part2).unwrap();
        let decoded3 = decode(&part3).unwrap();
        let decoded4 = decode(&part4).unwrap();

        let mut assembler = YencMultipartAssembler::new();
        assembler.add_part(decoded1).unwrap();
        assembler.add_part(decoded2).unwrap();
        assembler.add_part(decoded3).unwrap();
        assembler.add_part(decoded4).unwrap();

        assert!(assembler.is_complete());
        let assembled = assembler.assemble().unwrap();
        assert_eq!(assembled, full_data);
    }

    #[test]
    fn test_assembler_with_all_byte_values() {
        // Test with data containing all possible byte values
        let full_data: Vec<u8> = (0..=255).collect();
        let total_size = full_data.len() as u64; // 256 bytes

        // Split into 2 parts
        let part1_data = &full_data[0..128];
        let part2_data = &full_data[128..256];

        // ALL parts must have the same total_size in headers
        // Format: (part, total_parts, begin, end, total_file_size)
        let part1 = encode(
            part1_data,
            "bytes.bin",
            128,
            Some((1, 2, 1, 128, total_size)),
        )
        .unwrap();
        let part2 = encode(
            part2_data,
            "bytes.bin",
            128,
            Some((2, 2, 129, 256, total_size)),
        )
        .unwrap();

        let decoded1 = decode(&part1).unwrap();
        let decoded2 = decode(&part2).unwrap();

        let mut assembler = YencMultipartAssembler::new();
        assembler.add_part(decoded1).unwrap();
        assembler.add_part(decoded2).unwrap();

        let assembled = assembler.assemble().unwrap();
        assert_eq!(assembled, full_data);
    }
}

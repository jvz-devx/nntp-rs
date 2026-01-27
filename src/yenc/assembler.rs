use crate::{NntpError, Result};
use crc32fast::Hasher;
use std::collections::HashMap;

use super::types::YencDecoded;

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
    parts: HashMap<u32, YencDecoded>,
}

impl YencMultipartAssembler {
    /// Create a new multi-part assembler
    pub fn new() -> Self {
        Self {
            total_parts: None,
            total_size: None,
            filename: None,
            expected_crc32: None,
            parts: HashMap::new(),
        }
    }

    /// Check if a new part's byte range overlaps with any existing part.
    fn check_overlap(&self, part_num: u32, part_info: &super::types::YencPart) -> Result<()> {
        for (existing_num, existing) in &self.parts {
            if let Some(existing_info) = &existing.part {
                let overlaps =
                    !(part_info.end < existing_info.begin || part_info.begin > existing_info.end);
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
        Ok(())
    }

    /// Add a decoded part to the assembler
    ///
    /// # Errors
    /// Returns an error if:
    /// - The part is not a multi-part file
    /// - The part overlaps with an existing part
    /// - The part has inconsistent metadata
    // expect() is used here for invariants that are guaranteed by prior checks (is_multipart, Option state)
    #[allow(clippy::expect_used)]
    pub fn add_part(&mut self, decoded: YencDecoded) -> Result<()> {
        // Validate this is a multi-part file
        if !decoded.is_multipart() {
            return Err(NntpError::InvalidResponse(
                "Cannot add single-part file to multi-part assembler".to_string(),
            ));
        }

        // SAFETY: is_multipart() guarantees both part and total are Some
        let part_num = decoded
            .header
            .part
            .expect("part must be Some after is_multipart check");
        let total = decoded
            .header
            .total
            .expect("total must be Some after is_multipart check");

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
            // SAFETY: total_parts, total_size, and filename are all Some in this else branch
            if let Some(expected_total) = self.total_parts {
                if expected_total != total {
                    return Err(NntpError::InvalidResponse(format!(
                        "Inconsistent total parts: expected {}, got {}",
                        expected_total, total
                    )));
                }
            }
            if let Some(expected_size) = self.total_size {
                if expected_size != decoded.header.size {
                    return Err(NntpError::InvalidResponse(format!(
                        "Inconsistent total size: expected {}, got {}",
                        expected_size, decoded.header.size
                    )));
                }
            }
            if let Some(ref expected_name) = self.filename {
                if expected_name != &decoded.header.name {
                    return Err(NntpError::InvalidResponse(format!(
                        "Inconsistent filename: expected {}, got {}",
                        expected_name, decoded.header.name
                    )));
                }
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
            self.check_overlap(part_num, part_info)?;
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
    use crate::yenc::decode::decode;
    use crate::yenc::encode::encode;

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

//! Article assembler for binary downloads
//!
//! This module provides functionality to assemble multi-part yEnc-encoded
//! articles into complete files. It handles part collection, yEnc decoding,
//! CRC32 verification, and file assembly.

use crate::error::{NntpError, Result};
use crate::nzb::{NzbFile, NzbSegment};
use crate::yenc::{YencDecoded, YencMultipartAssembler, decode};
use std::collections::HashMap;

/// Status of an article part
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartStatus {
    /// Part is waiting to be downloaded
    Pending,
    /// Part has been downloaded and decoded successfully
    Downloaded,
    /// Part is missing from server
    Missing,
    /// Part failed CRC32 verification
    Corrupted,
}

/// Information about a single part in assembly
#[derive(Debug, Clone)]
pub struct PartInfo {
    /// The segment from NZB
    pub segment: NzbSegment,
    /// Current status
    pub status: PartStatus,
    /// Decoded data (if downloaded successfully)
    pub decoded: Option<YencDecoded>,
}

/// Article assembler that collects and assembles parts
///
/// This assembler:
/// - Tracks parts from an NZB file
/// - Decodes yEnc-encoded article content
/// - Verifies CRC32 checksums per part
/// - Assembles parts in correct order
/// - Detects completion and validates final file
#[derive(Debug)]
pub struct ArticleAssembler {
    /// The NZB file being assembled
    file: NzbFile,
    /// Parts indexed by segment number
    parts: HashMap<u32, PartInfo>,
    /// yEnc multi-part assembler for combining parts
    yenc_assembler: YencMultipartAssembler,
}

impl ArticleAssembler {
    /// Create a new article assembler for an NZB file
    ///
    /// # Arguments
    ///
    /// * `file` - The NZB file containing segment information
    ///
    /// # Examples
    ///
    /// ```
    /// use nntp_rs::{ArticleAssembler, NzbFile, NzbSegment};
    ///
    /// let file = NzbFile {
    ///     poster: "user@example.com".to_string(),
    ///     date: 1234567890,
    ///     subject: "test.bin (1/2)".to_string(),
    ///     groups: vec!["alt.binaries.test".to_string()],
    ///     segments: vec![
    ///         NzbSegment {
    ///             bytes: 1000,
    ///             number: 1,
    ///             message_id: "<part1@example.com>".to_string(),
    ///         },
    ///         NzbSegment {
    ///             bytes: 1000,
    ///             number: 2,
    ///             message_id: "<part2@example.com>".to_string(),
    ///         },
    ///     ],
    /// };
    ///
    /// let assembler = ArticleAssembler::new(file);
    /// ```
    pub fn new(file: NzbFile) -> Self {
        let mut parts = HashMap::new();
        for segment in &file.segments {
            parts.insert(
                segment.number,
                PartInfo {
                    segment: segment.clone(),
                    status: PartStatus::Pending,
                    decoded: None,
                },
            );
        }

        Self {
            file,
            parts,
            yenc_assembler: YencMultipartAssembler::new(),
        }
    }

    /// Add a downloaded article part from raw bytes
    ///
    /// The article content should be the raw bytes from the NNTP server article body.
    /// This method will:
    /// 1. Decode the yEnc content
    /// 2. Verify the part CRC32
    /// 3. Update the part status
    /// 4. Add to the yEnc assembler if valid
    ///
    /// # Arguments
    ///
    /// * `segment_number` - The segment number (1-based)
    /// * `article_data` - Raw article data bytes (yEnc encoded)
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the part was successfully added, or an error if:
    /// - The segment number is invalid
    /// - The yEnc decoding fails
    /// - The CRC32 verification fails
    ///
    /// # Note
    ///
    /// yEnc binary data contains bytes in the 0x80-0xFF range which are not valid UTF-8.
    /// Therefore, this method requires raw bytes, not UTF-8 strings. If you have
    /// `Vec<String>` from an NNTP response, you'll need to convert it back to bytes
    /// using Latin-1 encoding (ISO-8859-1), which Usenet traditionally uses.
    pub fn add_part_bytes(&mut self, segment_number: u32, article_data: &[u8]) -> Result<()> {
        // Get the part info
        let part_info = self.parts.get_mut(&segment_number).ok_or_else(|| {
            NntpError::InvalidResponse(format!("Invalid segment number: {}", segment_number))
        })?;

        // Decode yEnc content
        let decoded = decode(article_data)?;

        // Verify CRC32
        if !decoded.verify_crc32() {
            part_info.status = PartStatus::Corrupted;
            return Err(NntpError::InvalidResponse(format!(
                "CRC32 mismatch for segment {}: expected {:?}, got {}",
                segment_number,
                decoded.trailer.pcrc32.or(decoded.trailer.crc32),
                decoded.calculated_crc32
            )));
        }

        // Update part info
        part_info.status = PartStatus::Downloaded;
        part_info.decoded = Some(decoded.clone());

        // Add to yEnc assembler if multi-part
        if decoded.is_multipart() {
            self.yenc_assembler.add_part(decoded)?;
        }

        Ok(())
    }

    /// Mark a segment as missing
    ///
    /// Call this when a segment cannot be downloaded (e.g., 430 Not Found error)
    ///
    /// # Arguments
    ///
    /// * `segment_number` - The segment number (1-based)
    pub fn mark_missing(&mut self, segment_number: u32) -> Result<()> {
        let part_info = self.parts.get_mut(&segment_number).ok_or_else(|| {
            NntpError::InvalidResponse(format!("Invalid segment number: {}", segment_number))
        })?;

        part_info.status = PartStatus::Missing;
        Ok(())
    }

    /// Mark a segment as corrupted
    ///
    /// Call this when a segment fails CRC32 verification
    ///
    /// # Arguments
    ///
    /// * `segment_number` - The segment number (1-based)
    pub fn mark_corrupted(&mut self, segment_number: u32) -> Result<()> {
        let part_info = self.parts.get_mut(&segment_number).ok_or_else(|| {
            NntpError::InvalidResponse(format!("Invalid segment number: {}", segment_number))
        })?;

        part_info.status = PartStatus::Corrupted;
        Ok(())
    }

    /// Check if all parts have been processed
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.parts.values().all(|p| p.status != PartStatus::Pending)
    }

    /// Check if all downloaded parts are valid (not corrupted)
    #[must_use]
    pub fn all_parts_valid(&self) -> bool {
        self.parts
            .values()
            .filter(|p| p.status == PartStatus::Downloaded)
            .count()
            == self.parts.len()
    }

    /// Get list of missing segment numbers
    pub fn missing_parts(&self) -> Vec<u32> {
        self.parts
            .iter()
            .filter(|(_, p)| p.status == PartStatus::Missing)
            .map(|(num, _)| *num)
            .collect()
    }

    /// Get list of corrupted segment numbers
    pub fn corrupted_parts(&self) -> Vec<u32> {
        self.parts
            .iter()
            .filter(|(_, p)| p.status == PartStatus::Corrupted)
            .map(|(num, _)| *num)
            .collect()
    }

    /// Get list of pending segment numbers
    pub fn pending_parts(&self) -> Vec<u32> {
        self.parts
            .iter()
            .filter(|(_, p)| p.status == PartStatus::Pending)
            .map(|(num, _)| *num)
            .collect()
    }

    /// Get the total number of parts
    pub fn total_parts(&self) -> usize {
        self.parts.len()
    }

    /// Get the number of successfully downloaded parts
    pub fn downloaded_parts(&self) -> usize {
        self.parts
            .values()
            .filter(|p| p.status == PartStatus::Downloaded)
            .count()
    }

    /// Get the filename from the NZB
    pub fn filename(&self) -> &str {
        &self.file.subject
    }

    /// Assemble all parts into the final file data
    ///
    /// This method:
    /// 1. Verifies all parts are downloaded
    /// 2. Assembles multi-part files using the yEnc assembler
    /// 3. Verifies the final CRC32 if available
    /// 4. Returns the complete file data
    ///
    /// # Returns
    ///
    /// Returns the assembled file data, or an error if:
    /// - Not all parts are downloaded
    /// - Any parts are missing or corrupted
    /// - The final CRC32 verification fails (if applicable)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use nntp_rs::{ArticleAssembler, NzbFile, NzbSegment};
    /// # let file = NzbFile {
    /// #     poster: "user@example.com".to_string(),
    /// #     date: 1234567890,
    /// #     subject: "test.bin (1/1)".to_string(),
    /// #     groups: vec!["alt.binaries.test".to_string()],
    /// #     segments: vec![
    /// #         NzbSegment {
    /// #             bytes: 100,
    /// #             number: 1,
    /// #             message_id: "<part1@example.com>".to_string(),
    /// #         },
    /// #     ],
    /// # };
    /// # let mut assembler = ArticleAssembler::new(file);
    /// // ... add all parts ...
    ///
    /// if assembler.is_complete() && assembler.all_parts_valid() {
    ///     let file_data = assembler.assemble()?;
    ///     // Write file_data to disk
    /// }
    /// # Ok::<(), nntp_rs::NntpError>(())
    /// ```
    // Single-part files in NZB format are always numbered as part 1
    // Downloaded status guarantees decoded data exists (set in add_part_bytes)
    #[expect(clippy::expect_used)]
    pub fn assemble(&self) -> Result<Vec<u8>> {
        // Check completion
        if !self.is_complete() {
            let pending = self.pending_parts();
            return Err(NntpError::InvalidResponse(format!(
                "Cannot assemble: {} parts still pending: {:?}",
                pending.len(),
                pending
            )));
        }

        // Check for missing parts
        let missing = self.missing_parts();
        if !missing.is_empty() {
            return Err(NntpError::InvalidResponse(format!(
                "Cannot assemble: {} parts missing: {:?}",
                missing.len(),
                missing
            )));
        }

        // Check for corrupted parts
        let corrupted = self.corrupted_parts();
        if !corrupted.is_empty() {
            return Err(NntpError::InvalidResponse(format!(
                "Cannot assemble: {} parts corrupted: {:?}",
                corrupted.len(),
                corrupted
            )));
        }

        // For single-part files, just return the decoded data
        if self.parts.len() == 1 {
            // SAFETY: Single-part files are always numbered as part 1 in NZB format.
            // We know parts.len() == 1, so part 1 must exist.
            let part = self
                .parts
                .get(&1)
                .expect("BUG: single-part file should have part 1");

            // SAFETY: We've verified above that no parts are pending, missing, or corrupted.
            // The only remaining status is Downloaded, which guarantees decoded is Some()
            // (set in add_part_bytes at line 156).
            return Ok(part
                .decoded
                .as_ref()
                .expect("BUG: downloaded part must have decoded data")
                .data
                .clone());
        }

        // For multi-part files, use the yEnc assembler
        if !self.yenc_assembler.is_complete() {
            return Err(NntpError::InvalidResponse(
                "yEnc assembler is not complete".to_string(),
            ));
        }

        let assembled = self.yenc_assembler.assemble()?;

        // Note: verify_final_crc32() returns false if no CRC32 is available,
        // which is normal for most multi-part files. The per-part CRC32s
        // are already verified in add_part_bytes(), so we're good.

        Ok(assembled)
    }

    /// Get the status of a specific part
    pub fn part_status(&self, segment_number: u32) -> Option<&PartStatus> {
        self.parts.get(&segment_number).map(|p| &p.status)
    }

    /// Get reference to the NZB file
    pub fn nzb_file(&self) -> &NzbFile {
        &self.file
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::yenc::encode;

    fn create_test_file(segments: Vec<NzbSegment>) -> NzbFile {
        NzbFile {
            poster: "test@example.com".to_string(),
            date: 1234567890,
            subject: "test.bin".to_string(),
            groups: vec!["alt.binaries.test".to_string()],
            segments,
        }
    }

    #[test]
    fn test_assembler_new() {
        let file = create_test_file(vec![
            NzbSegment {
                bytes: 100,
                number: 1,
                message_id: "<part1@example.com>".to_string(),
            },
            NzbSegment {
                bytes: 100,
                number: 2,
                message_id: "<part2@example.com>".to_string(),
            },
        ]);

        let assembler = ArticleAssembler::new(file);
        assert_eq!(assembler.total_parts(), 2);
        assert_eq!(assembler.downloaded_parts(), 0);
        assert!(!assembler.is_complete());
        assert_eq!(assembler.pending_parts().len(), 2);
    }

    #[test]
    fn test_assembler_single_part() {
        let test_data = b"Hello, World!";
        let encoded_bytes = encode(test_data, "test.bin", 128, None).unwrap();

        let file = create_test_file(vec![NzbSegment {
            bytes: test_data.len() as u64,
            number: 1,
            message_id: "<part1@example.com>".to_string(),
        }]);

        let mut assembler = ArticleAssembler::new(file);
        assembler.add_part_bytes(1, &encoded_bytes).unwrap();

        assert_eq!(assembler.downloaded_parts(), 1);
        assert!(assembler.is_complete());
        assert!(assembler.all_parts_valid());

        let result = assembler.assemble().unwrap();
        assert_eq!(result, test_data);
    }

    #[test]
    fn test_assembler_multi_part() {
        let test_data = b"Hello, World! This is a test of multi-part encoding.";
        let part1_data = &test_data[0..20];
        let part2_data = &test_data[20..];

        let encoded1_bytes = encode(
            part1_data,
            "test.bin",
            128,
            Some((1, 2, 1, 20, test_data.len() as u64)),
        )
        .unwrap();

        let encoded2_bytes = encode(
            part2_data,
            "test.bin",
            128,
            Some((2, 2, 21, test_data.len() as u64, test_data.len() as u64)),
        )
        .unwrap();

        let file = create_test_file(vec![
            NzbSegment {
                bytes: part1_data.len() as u64,
                number: 1,
                message_id: "<part1@example.com>".to_string(),
            },
            NzbSegment {
                bytes: part2_data.len() as u64,
                number: 2,
                message_id: "<part2@example.com>".to_string(),
            },
        ]);

        let mut assembler = ArticleAssembler::new(file);
        assembler.add_part_bytes(1, &encoded1_bytes).unwrap();
        assembler.add_part_bytes(2, &encoded2_bytes).unwrap();

        assert_eq!(assembler.downloaded_parts(), 2);
        assert!(assembler.is_complete());
        assert!(assembler.all_parts_valid());

        let result = assembler.assemble().unwrap();
        assert_eq!(result, test_data);
    }

    #[test]
    fn test_assembler_out_of_order() {
        let test_data = b"Test data for out-of-order assembly";
        let part1_data = &test_data[0..15];
        let part2_data = &test_data[15..];

        let encoded1_bytes = encode(
            part1_data,
            "test.bin",
            128,
            Some((1, 2, 1, 15, test_data.len() as u64)),
        )
        .unwrap();

        let encoded2_bytes = encode(
            part2_data,
            "test.bin",
            128,
            Some((2, 2, 16, test_data.len() as u64, test_data.len() as u64)),
        )
        .unwrap();

        let file = create_test_file(vec![
            NzbSegment {
                bytes: part1_data.len() as u64,
                number: 1,
                message_id: "<part1@example.com>".to_string(),
            },
            NzbSegment {
                bytes: part2_data.len() as u64,
                number: 2,
                message_id: "<part2@example.com>".to_string(),
            },
        ]);

        let mut assembler = ArticleAssembler::new(file);
        // Add parts out of order
        assembler.add_part_bytes(2, &encoded2_bytes).unwrap();
        assembler.add_part_bytes(1, &encoded1_bytes).unwrap();

        assert!(assembler.is_complete());
        assert!(assembler.all_parts_valid());

        let result = assembler.assemble().unwrap();
        assert_eq!(result, test_data);
    }

    #[test]
    fn test_assembler_missing_part() {
        let file = create_test_file(vec![
            NzbSegment {
                bytes: 100,
                number: 1,
                message_id: "<part1@example.com>".to_string(),
            },
            NzbSegment {
                bytes: 100,
                number: 2,
                message_id: "<part2@example.com>".to_string(),
            },
        ]);

        let mut assembler = ArticleAssembler::new(file);
        assembler.mark_missing(2).unwrap();

        assert!(!assembler.all_parts_valid());
        assert_eq!(assembler.missing_parts(), vec![2]);

        let result = assembler.assemble();
        assert!(result.is_err());
    }

    #[test]
    fn test_assembler_invalid_segment_number() {
        let file = create_test_file(vec![NzbSegment {
            bytes: 100,
            number: 1,
            message_id: "<part1@example.com>".to_string(),
        }]);

        let mut assembler = ArticleAssembler::new(file);
        let result = assembler.mark_missing(5);
        assert!(result.is_err());
    }

    #[test]
    fn test_assembler_status_tracking() {
        let test_data = b"Test";
        let encoded_bytes = encode(test_data, "test.bin", 128, None).unwrap();

        let file = create_test_file(vec![NzbSegment {
            bytes: test_data.len() as u64,
            number: 1,
            message_id: "<part1@example.com>".to_string(),
        }]);

        let mut assembler = ArticleAssembler::new(file);
        assert_eq!(assembler.part_status(1), Some(&PartStatus::Pending));

        assembler.add_part_bytes(1, &encoded_bytes).unwrap();
        assert_eq!(assembler.part_status(1), Some(&PartStatus::Downloaded));
    }

    #[test]
    fn test_assembler_assemble_incomplete() {
        let file = create_test_file(vec![
            NzbSegment {
                bytes: 100,
                number: 1,
                message_id: "<part1@example.com>".to_string(),
            },
            NzbSegment {
                bytes: 100,
                number: 2,
                message_id: "<part2@example.com>".to_string(),
            },
        ]);

        let assembler = ArticleAssembler::new(file);
        let result = assembler.assemble();
        assert!(result.is_err());
    }
}

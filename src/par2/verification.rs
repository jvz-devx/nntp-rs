//! PAR2 file verification
//!
//! This module contains all PAR2 file verification logic, including:
//! - File verification against MD5 hashes and CRC32 checksums
//! - Slice mapping and damage identification
//! - Comprehensive slice summaries for recovery planning

use super::*;
use crate::error::{NntpError, Result};
use crc32fast::Hasher as Crc32;
use md5::{Digest, Md5};
use std::collections::HashMap;

/// Check a single slice's CRC32 against expected value.
/// Returns `true` if the slice is damaged (CRC mismatch or truncated).
fn is_slice_damaged(file_data: &[u8], ifsc: &IfscPacket, mapping: &SliceMapping) -> Option<bool> {
    let checksums = ifsc.checksums.get(mapping.file_slice_index)?;
    let slice_start = mapping.offset as usize;
    if slice_start >= file_data.len() {
        return Some(true); // truncated
    }
    let slice_end = std::cmp::min(slice_start + mapping.size as usize, file_data.len());
    let mut hasher = Crc32::new();
    hasher.update(&file_data[slice_start..slice_end]);
    Some(hasher.finalize() != *checksums)
}

impl Par2File {
    /// Map slices to files
    ///
    /// Creates a mapping of slice indices to the files that own them.
    /// Returns a vector where the index is the global slice index and the
    /// value is the `SliceMapping` describing which file owns that slice.
    ///
    /// # Returns
    /// Vector of slice mappings, or error if Main packet is missing
    pub fn map_slices(&self) -> Result<Vec<SliceMapping>> {
        let slice_size = self.slice_size().ok_or_else(|| {
            NntpError::InvalidResponse("No main packet found in PAR2".to_string())
        })?;

        let mut slice_mappings = Vec::new();

        // Iterate through files in the order specified by the Main packet
        let main = self.main.as_ref().ok_or_else(|| {
            NntpError::InvalidResponse("No main packet found in PAR2".to_string())
        })?;
        for file_id in &main.file_ids {
            // Get file description
            let file_desc = self.file_descriptions.get(file_id).ok_or_else(|| {
                NntpError::InvalidResponse(format!(
                    "File ID {:?} not found in file descriptions",
                    file_id
                ))
            })?;

            // Calculate number of slices for this file
            let file_slices = if file_desc.length == 0 {
                0
            } else {
                file_desc.length.div_ceil(slice_size) as usize
            };

            // Create mapping for each slice
            for file_slice_idx in 0..file_slices {
                let offset = file_slice_idx as u64 * slice_size;
                let remaining = file_desc.length.saturating_sub(offset);
                let size = std::cmp::min(slice_size, remaining);

                slice_mappings.push(SliceMapping {
                    file_id: *file_id,
                    // Convert Arc<str> to String - cheap clone converted to owned String
                    filename: file_desc.name.to_string(),
                    file_slice_index: file_slice_idx,
                    offset,
                    size,
                });
            }
        }

        Ok(slice_mappings)
    }

    /// Identify damaged and missing slices across all files
    ///
    /// # Arguments
    /// * `file_data_map` - HashMap mapping file IDs to their actual data
    ///
    /// # Returns
    /// Tuple of (damaged_slice_indices, missing_slice_indices) using global slice indices
    pub fn identify_damaged_slices(
        &self,
        file_data_map: &HashMap<[u8; 16], Vec<u8>>,
    ) -> Result<(Vec<usize>, Vec<usize>)> {
        let slice_mappings = self.map_slices()?;
        let mut damaged_slices = Vec::new();
        let mut missing_slices = Vec::new();

        // Track which file we're currently processing
        let mut current_file_id: Option<[u8; 16]> = None;
        let mut file_damaged_slices: Vec<usize> = Vec::new();

        for (global_idx, mapping) in slice_mappings.iter().enumerate() {
            // If we've moved to a new file, verify the previous file's slices
            if let Some(prev_id) = current_file_id {
                if prev_id != mapping.file_id {
                    // Process completed file - already handled by file_damaged_slices
                    file_damaged_slices.clear();
                }
            }

            current_file_id = Some(mapping.file_id);

            // Check if file exists
            let Some(file_data) = file_data_map.get(&mapping.file_id) else {
                // File is missing - all its slices are missing
                missing_slices.push(global_idx);
                continue;
            };

            // Check if file is empty (missing)
            if file_data.is_empty() {
                missing_slices.push(global_idx);
                continue;
            }

            // Verify this slice if IFSC packet exists
            if let Some(ifsc) = self.ifsc_packets.get(&mapping.file_id) {
                if is_slice_damaged(file_data, ifsc, mapping) == Some(true) {
                    damaged_slices.push(global_idx);
                }
            }
        }

        Ok((damaged_slices, missing_slices))
    }

    /// Get a comprehensive summary of all slices in the PAR2 set
    ///
    /// # Arguments
    /// * `file_data_map` - HashMap mapping file IDs to their actual data
    ///
    /// # Returns
    /// `SliceSummary` containing complete slice information
    pub fn slice_summary(
        &self,
        file_data_map: &HashMap<[u8; 16], Vec<u8>>,
    ) -> Result<SliceSummary> {
        let slice_mappings = self.map_slices()?;
        let (damaged_slices, missing_slices) = self.identify_damaged_slices(file_data_map)?;
        let recovery_slice_count = self.recovery_slice_count();

        Ok(SliceSummary {
            total_data_slices: slice_mappings.len(),
            recovery_slice_count,
            slice_mappings,
            damaged_slices,
            missing_slices,
        })
    }

    /// Verify a file against PAR2 metadata
    ///
    /// # Arguments
    /// * `file_data` - The actual file data to verify
    /// * `file_id` - The file ID from the PAR2 file description
    ///
    /// # Returns
    /// `FileVerification` struct with verification results
    pub fn verify_file(&self, file_data: &[u8], file_id: &[u8; 16]) -> Result<FileVerification> {
        // Get file description
        let file_desc = self
            .file_descriptions
            .get(file_id)
            .ok_or_else(|| NntpError::InvalidResponse("File ID not found in PAR2".to_string()))?;

        // Check if file is missing
        if file_data.is_empty() {
            return Ok(FileVerification {
                file_id: *file_id,
                // Convert Arc<str> to String - cheap clone converted to owned String
                filename: file_desc.name.to_string(),
                expected_size: file_desc.length,
                status: FileStatus::Missing,
                hash_match: None,
                hash_16k_match: None,
            });
        }

        // Verify file size
        if file_data.len() as u64 != file_desc.length {
            return Ok(FileVerification {
                file_id: *file_id,
                // Convert Arc<str> to String - cheap clone converted to owned String
                filename: file_desc.name.to_string(),
                expected_size: file_desc.length,
                status: FileStatus::Damaged(vec![]), // Size mismatch
                hash_match: Some(false),
                hash_16k_match: None,
            });
        }

        // Calculate MD5 hash of entire file
        let mut hasher = Md5::new();
        hasher.update(file_data);
        let file_hash: [u8; 16] = hasher.finalize().into();
        let hash_match = file_hash == file_desc.hash;

        // Calculate MD5 hash of first 16k
        let hash_16k_len = std::cmp::min(16384, file_data.len());
        let mut hasher_16k = Md5::new();
        hasher_16k.update(&file_data[..hash_16k_len]);
        let file_hash_16k: [u8; 16] = hasher_16k.finalize().into();
        let hash_16k_match = file_hash_16k == file_desc.hash_16k;

        // If hashes match, file is complete
        if hash_match && hash_16k_match {
            return Ok(FileVerification {
                file_id: *file_id,
                // Convert Arc<str> to String - cheap clone converted to owned String
                filename: file_desc.name.to_string(),
                expected_size: file_desc.length,
                status: FileStatus::Complete,
                hash_match: Some(true),
                hash_16k_match: Some(true),
            });
        }

        // Verify slices if IFSC packet exists
        let damaged_slices = if let Some(ifsc) = self.ifsc_packets.get(file_id) {
            self.verify_slices(file_data, ifsc)?
        } else {
            vec![] // No slice checksums available
        };

        let status = if damaged_slices.is_empty() && !hash_match {
            // Hashes don't match but slices are OK - shouldn't happen
            FileStatus::Damaged(vec![])
        } else if !damaged_slices.is_empty() {
            FileStatus::Damaged(damaged_slices)
        } else {
            FileStatus::Complete
        };

        Ok(FileVerification {
            file_id: *file_id,
            // Convert Arc<str> to String - cheap clone converted to owned String
            filename: file_desc.name.to_string(),
            expected_size: file_desc.length,
            status,
            hash_match: Some(hash_match),
            hash_16k_match: Some(hash_16k_match),
        })
    }

    /// Verify file slices using IFSC packet
    ///
    /// # Arguments
    /// * `file_data` - The file data to verify
    /// * `ifsc` - The IFSC packet with CRC32 checksums
    ///
    /// # Returns
    /// Vector of damaged slice indices (0-based)
    // Main packet existence is validated by slice_size() check
    #[allow(clippy::expect_used)]
    fn verify_slices(&self, file_data: &[u8], ifsc: &IfscPacket) -> Result<Vec<usize>> {
        let slice_size = self
            .slice_size()
            .ok_or_else(|| NntpError::InvalidResponse("No main packet found in PAR2".to_string()))?
            as usize;

        let mut damaged = Vec::new();

        // Verify each slice
        for (slice_idx, &expected_crc) in ifsc.checksums.iter().enumerate() {
            let slice_start = slice_idx * slice_size;
            let slice_end = std::cmp::min(slice_start + slice_size, file_data.len());

            if slice_start >= file_data.len() {
                // Slice is beyond file size - file is truncated
                damaged.push(slice_idx);
                continue;
            }

            let slice_data = &file_data[slice_start..slice_end];

            // Calculate CRC32 of slice
            let mut hasher = Crc32::new();
            hasher.update(slice_data);
            let actual_crc = hasher.finalize();

            if actual_crc != expected_crc {
                damaged.push(slice_idx);
            }
        }

        Ok(damaged)
    }

    /// Verify all files in the PAR2 set
    ///
    /// # Arguments
    /// * `file_data_map` - HashMap mapping file IDs to their actual data
    ///
    /// # Returns
    /// Vector of verification results for all files
    pub fn verify_all(
        &self,
        file_data_map: &HashMap<[u8; 16], Vec<u8>>,
    ) -> Result<Vec<FileVerification>> {
        let mut results = Vec::new();

        for file_id in self.file_descriptions.keys() {
            let file_data = file_data_map
                .get(file_id)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            let verification = self.verify_file(file_data, file_id)?;
            results.push(verification);
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_status_complete() {
        let status = FileStatus::Complete;
        assert_eq!(status, FileStatus::Complete);
    }

    #[test]
    fn test_file_status_damaged() {
        let status = FileStatus::Damaged(vec![0, 2, 5]);
        match status {
            FileStatus::Damaged(indices) => {
                assert_eq!(indices, vec![0, 2, 5]);
            }
            _ => panic!("Expected Damaged status"),
        }
    }

    #[test]
    fn test_file_status_missing() {
        let status = FileStatus::Missing;
        assert_eq!(status, FileStatus::Missing);
    }

    #[test]
    fn test_verify_file_missing() {
        let mut par2 = Par2File::new();
        par2.set_id = [1; 16];

        let file_id = [1; 16];
        par2.file_descriptions.insert(
            file_id,
            FileDescriptionPacket {
                file_id,
                hash: [0; 16],
                hash_16k: [0; 16],
                length: 1000,
                name: "test.bin".into(),
            },
        );

        let file_data = b"";
        let verification = par2.verify_file(file_data, &file_id).unwrap();

        assert_eq!(verification.status, FileStatus::Missing);
        assert_eq!(verification.filename, "test.bin");
        assert_eq!(verification.expected_size, 1000);
    }

    #[test]
    fn test_verify_file_size_mismatch() {
        let mut par2 = Par2File::new();
        par2.set_id = [1; 16];

        let file_id = [1; 16];
        par2.file_descriptions.insert(
            file_id,
            FileDescriptionPacket {
                file_id,
                hash: [0; 16],
                hash_16k: [0; 16],
                length: 1000,
                name: "test.bin".into(),
            },
        );

        let file_data = b"too short";
        let verification = par2.verify_file(file_data, &file_id).unwrap();

        match verification.status {
            FileStatus::Damaged(_) => {}
            _ => panic!("Expected Damaged status for size mismatch"),
        }
        assert_eq!(verification.hash_match, Some(false));
    }

    #[test]
    fn test_map_slices_no_main_packet() {
        let par2 = Par2File::new();
        let result = par2.map_slices();
        assert!(result.is_err());
    }

    #[test]
    fn test_map_slices_empty_files() {
        let mut par2 = Par2File::new();
        par2.main = Some(MainPacket {
            slice_size: 1024,
            file_count: 0,
            file_ids: vec![],
            non_recoverable_file_ids: vec![],
        });

        let slices = par2.map_slices().unwrap();
        assert_eq!(slices.len(), 0);
    }

    #[test]
    fn test_map_slices_single_file() {
        let mut par2 = Par2File::new();

        let file_id = [1; 16];
        par2.main = Some(MainPacket {
            slice_size: 1024,
            file_count: 1,
            file_ids: vec![file_id],
            non_recoverable_file_ids: vec![],
        });

        par2.file_descriptions.insert(
            file_id,
            FileDescriptionPacket {
                file_id,
                hash: [0; 16],
                hash_16k: [0; 16],
                length: 2048, // 2 slices
                name: "test.bin".into(),
            },
        );

        let slices = par2.map_slices().unwrap();
        assert_eq!(slices.len(), 2);
        assert_eq!(slices[0].file_id, file_id);
        assert_eq!(slices[0].file_slice_index, 0);
        assert_eq!(slices[0].offset, 0);
        assert_eq!(slices[0].size, 1024);
        assert_eq!(slices[1].file_slice_index, 1);
        assert_eq!(slices[1].offset, 1024);
        assert_eq!(slices[1].size, 1024);
    }

    #[test]
    fn test_verify_all_no_files() {
        let par2 = Par2File::new();
        let file_data_map = HashMap::new();
        let results = par2.verify_all(&file_data_map).unwrap();
        assert_eq!(results.len(), 0);
    }
}

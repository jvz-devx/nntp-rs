//! PAR2 (Parity Archive 2) file format parsing and verification
//!
//! This module implements parsing of PAR2 files used for error correction
//! and recovery of Usenet binary downloads.
//!
//! Reference: [Parity Volume Set Specification 2.0](https://parchive.sourceforge.net/docs/specifications/parity-volume-spec/article-spec.html)

use crate::error::{NntpError, Result};
use crc32fast::Hasher as Crc32;
use md5::{Digest, Md5};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// PAR2 packet magic bytes: "PAR2\0PKT"
pub const PAR2_MAGIC: &[u8; 8] = b"PAR2\0PKT";

/// PAR2 packet header (64 bytes)
#[derive(Debug, Clone)]
pub struct PacketHeader {
    /// Packet length including header (8 bytes)
    pub length: u64,
    /// MD5 hash of entire packet except first 32 bytes (16 bytes)
    pub hash: [u8; 16],
    /// Recovery Set ID - identifies the set of files (16 bytes)
    pub set_id: [u8; 16],
    /// Packet type identifier (16 bytes)
    pub packet_type: [u8; 16],
}

/// Main packet - describes the recovery set
#[derive(Debug, Clone)]
pub struct MainPacket {
    /// Slice size in bytes
    pub slice_size: u64,
    /// Number of recoverable files
    pub file_count: u32,
    /// File IDs in the recovery set
    pub file_ids: Vec<[u8; 16]>,
    /// Non-recoverable file IDs
    pub non_recoverable_file_ids: Vec<[u8; 16]>,
}

/// File Description packet - describes a single file
#[derive(Debug, Clone)]
pub struct FileDescriptionPacket {
    /// File ID (16 bytes)
    pub file_id: [u8; 16],
    /// MD5 hash of entire file (16 bytes)
    pub hash: [u8; 16],
    /// MD5 hash of first 16k of file (16 bytes)
    pub hash_16k: [u8; 16],
    /// File length in bytes
    pub length: u64,
    /// File name (null-terminated, padded to multiple of 4)
    pub name: String,
}

/// Input File Slice Checksum packet - CRC32 checksums for file slices
#[derive(Debug, Clone)]
pub struct IfscPacket {
    /// File ID (16 bytes)
    pub file_id: [u8; 16],
    /// CRC32 checksums for each slice (4 bytes each)
    pub checksums: Vec<u32>,
}

/// Recovery Slice packet - contains parity data for recovery
#[derive(Debug, Clone)]
pub struct RecoverySlicePacket {
    /// Exponent value
    pub exponent: u32,
    /// Recovery data
    pub data: Vec<u8>,
}

/// Creator packet - identifies PAR2 creator software
#[derive(Debug, Clone)]
pub struct CreatorPacket {
    /// Client identifier (e.g., "par2cmdline")
    pub client: String,
}

/// File verification status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    /// File is complete and matches checksums
    Complete,
    /// File has damaged slices (can potentially be repaired)
    Damaged(Vec<usize>), // damaged slice indices
    /// File is missing
    Missing,
}

/// File verification result
#[derive(Debug, Clone)]
pub struct FileVerification {
    /// File ID
    pub file_id: [u8; 16],
    /// File name
    pub filename: String,
    /// Expected file size
    pub expected_size: u64,
    /// Verification status
    pub status: FileStatus,
    /// MD5 hash match (if file data provided)
    pub hash_match: Option<bool>,
    /// MD5 hash of first 16k match (if file data provided)
    pub hash_16k_match: Option<bool>,
}

/// Slice mapping - maps a slice index to its owning file
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceMapping {
    /// File ID that owns this slice
    pub file_id: [u8; 16],
    /// File name
    pub filename: String,
    /// Slice index within the file (0-based)
    pub file_slice_index: usize,
    /// Offset of this slice within the file
    pub offset: u64,
    /// Size of this slice in bytes (may be smaller than slice_size for last slice)
    pub size: u64,
}

/// Summary of all slices in the PAR2 set
#[derive(Debug, Clone)]
pub struct SliceSummary {
    /// Total number of data slices across all files
    pub total_data_slices: usize,
    /// Number of available recovery slices
    pub recovery_slice_count: usize,
    /// Slice mappings (indexed by global slice index)
    pub slice_mappings: Vec<SliceMapping>,
    /// Damaged slice indices (global indices)
    pub damaged_slices: Vec<usize>,
    /// Missing slice indices (global indices)
    pub missing_slices: Vec<usize>,
}

/// PAR2 packet types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PacketType {
    /// Main packet
    Main,
    /// File Description packet
    FileDescription,
    /// Input File Slice Checksum packet
    Ifsc,
    /// Recovery Slice packet
    RecoverySlice,
    /// Creator packet
    Creator,
    /// Unknown packet type
    Unknown([u8; 16]),
}

impl PacketType {
    /// Create PacketType from 16-byte type identifier
    pub fn from_bytes(bytes: &[u8; 16]) -> Self {
        match bytes {
            b"PAR 2.0\0Main\0\0\0\0" => PacketType::Main,
            b"PAR 2.0\0FileDesc" => PacketType::FileDescription,
            b"PAR 2.0\0IFSC\0\0\0\0" => PacketType::Ifsc,
            b"PAR 2.0\0RecvSlic" => PacketType::RecoverySlice,
            b"PAR 2.0\0Creator\0" => PacketType::Creator,
            _ => PacketType::Unknown(*bytes),
        }
    }

    /// Get the 16-byte type identifier
    pub fn to_bytes(&self) -> [u8; 16] {
        match self {
            PacketType::Main => *b"PAR 2.0\0Main\0\0\0\0",
            PacketType::FileDescription => *b"PAR 2.0\0FileDesc",
            PacketType::Ifsc => *b"PAR 2.0\0IFSC\0\0\0\0",
            PacketType::RecoverySlice => *b"PAR 2.0\0RecvSlic",
            PacketType::Creator => *b"PAR 2.0\0Creator\0",
            PacketType::Unknown(bytes) => *bytes,
        }
    }
}

/// PAR2 file parser
#[derive(Debug, Clone)]
pub struct Par2File {
    /// Recovery set ID
    pub set_id: [u8; 16],
    /// Main packet
    pub main: Option<MainPacket>,
    /// File description packets (by file ID)
    pub file_descriptions: HashMap<[u8; 16], FileDescriptionPacket>,
    /// IFSC packets (by file ID)
    pub ifsc_packets: HashMap<[u8; 16], IfscPacket>,
    /// Recovery slice packets
    pub recovery_slices: Vec<RecoverySlicePacket>,
    /// Creator packet
    pub creator: Option<CreatorPacket>,
}

impl Par2File {
    /// Create a new empty PAR2 file structure
    pub fn new() -> Self {
        Self {
            set_id: [0; 16],
            main: None,
            file_descriptions: HashMap::new(),
            ifsc_packets: HashMap::new(),
            recovery_slices: Vec::new(),
            creator: None,
        }
    }

    /// Parse a PAR2 file from bytes
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut par2 = Par2File::new();
        let mut offset = 0;

        // Read all packets
        while offset < data.len() {
            // Need at least 64 bytes for header
            if offset + 64 > data.len() {
                break;
            }

            // Parse packet header
            let header = parse_packet_header(&data[offset..])?;

            // Check magic bytes
            if &data[offset..offset + 8] != PAR2_MAGIC {
                return Err(NntpError::InvalidResponse(format!(
                    "Invalid PAR2 magic bytes at offset {}",
                    offset
                )));
            }

            // Store set ID from first packet
            if par2.set_id == [0; 16] {
                par2.set_id = header.set_id;
            }

            // Verify set ID matches
            if header.set_id != par2.set_id {
                return Err(NntpError::InvalidResponse(
                    "Packet set ID does not match".to_string(),
                ));
            }

            // Parse packet body based on type
            let packet_type = PacketType::from_bytes(&header.packet_type);
            let body_offset = offset + 64;
            let body_len = header.length as usize - 64;

            if body_offset + body_len > data.len() {
                return Err(NntpError::InvalidResponse(format!(
                    "Packet body extends beyond file at offset {}",
                    offset
                )));
            }

            let body = &data[body_offset..body_offset + body_len];

            match packet_type {
                PacketType::Main => {
                    par2.main = Some(parse_main_packet(body)?);
                }
                PacketType::FileDescription => {
                    let file_desc = parse_file_description_packet(body)?;
                    par2.file_descriptions.insert(file_desc.file_id, file_desc);
                }
                PacketType::Ifsc => {
                    let ifsc = parse_ifsc_packet(body)?;
                    par2.ifsc_packets.insert(ifsc.file_id, ifsc);
                }
                PacketType::RecoverySlice => {
                    let recovery = parse_recovery_slice_packet(body)?;
                    par2.recovery_slices.push(recovery);
                }
                PacketType::Creator => {
                    par2.creator = Some(parse_creator_packet(body)?);
                }
                PacketType::Unknown(_) => {
                    // Skip unknown packet types
                }
            }

            offset += header.length as usize;
        }

        Ok(par2)
    }

    /// Get slice size from main packet
    pub fn slice_size(&self) -> Option<u64> {
        self.main.as_ref().map(|m| m.slice_size)
    }

    /// Get the number of recovery slices available
    pub fn recovery_slice_count(&self) -> usize {
        self.recovery_slices.len()
    }

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
        let main = self.main.as_ref().unwrap();
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
                    filename: file_desc.name.clone(),
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
            if current_file_id.is_some() && current_file_id.unwrap() != mapping.file_id {
                // Process completed file - already handled by file_damaged_slices
                file_damaged_slices.clear();
            }

            current_file_id = Some(mapping.file_id);

            // Check if file exists
            let file_data = file_data_map.get(&mapping.file_id);

            if file_data.is_none() {
                // File is missing - all its slices are missing
                missing_slices.push(global_idx);
                continue;
            }

            let file_data = file_data.unwrap();

            // Check if file is empty (missing)
            if file_data.is_empty() {
                missing_slices.push(global_idx);
                continue;
            }

            // Verify this slice if IFSC packet exists
            if let Some(ifsc) = self.ifsc_packets.get(&mapping.file_id) {
                // Check if we have a checksum for this slice
                if mapping.file_slice_index < ifsc.checksums.len() {
                    let expected_crc = ifsc.checksums[mapping.file_slice_index];
                    let slice_start = mapping.offset as usize;
                    let slice_end =
                        std::cmp::min(slice_start + mapping.size as usize, file_data.len());

                    if slice_start >= file_data.len() {
                        // Slice is beyond file size
                        damaged_slices.push(global_idx);
                        continue;
                    }

                    let slice_data = &file_data[slice_start..slice_end];

                    // Calculate CRC32 of slice
                    let mut hasher = Crc32::new();
                    hasher.update(slice_data);
                    let actual_crc = hasher.finalize();

                    if actual_crc != expected_crc {
                        damaged_slices.push(global_idx);
                    }
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
                filename: file_desc.name.clone(),
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
                filename: file_desc.name.clone(),
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
                filename: file_desc.name.clone(),
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
            filename: file_desc.name.clone(),
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

    /// Merge recovery slices from another PAR2 file (typically from a volume file)
    ///
    /// This is used when loading a PAR2 set with multiple volume files.
    /// Only recovery slices are merged; file descriptions and main packet
    /// must be consistent across all files in the set.
    ///
    /// # Arguments
    /// * `other` - Another PAR2File to merge recovery slices from
    ///
    /// # Returns
    /// Error if set IDs don't match
    pub fn merge_recovery_slices(&mut self, other: &Par2File) -> Result<()> {
        // Verify set IDs match
        if self.set_id != other.set_id {
            return Err(NntpError::InvalidResponse(
                "Cannot merge PAR2 files with different set IDs".to_string(),
            ));
        }

        // Merge recovery slices
        self.recovery_slices
            .extend(other.recovery_slices.iter().cloned());

        Ok(())
    }
}

/// PAR2 set - collection of PAR2 files for a single recovery set
///
/// A PAR2 set typically consists of:
/// - A main .par2 file containing file descriptions and some recovery slices
/// - Volume files (.vol00+01.par2, .vol01+02.par2, etc.) containing additional recovery slices
#[derive(Debug, Clone)]
pub struct Par2Set {
    /// Main PAR2 file containing all metadata
    pub main: Par2File,
    /// Paths to all PAR2 files in the set
    pub files: Vec<PathBuf>,
    /// Total number of recovery slices available
    pub total_recovery_slices: usize,
}

impl Par2Set {
    /// Discover and load a PAR2 set from a directory
    ///
    /// This function:
    /// 1. Finds all .par2 files in the directory
    /// 2. Identifies the main .par2 file (without .vol in name)
    /// 3. Parses all PAR2 files and merges recovery slices
    /// 4. Calculates total recovery capacity
    ///
    /// # Arguments
    /// * `dir` - Directory path to search for PAR2 files
    /// * `base_name` - Base name of the PAR2 set (e.g., "myfile" for "myfile.par2")
    ///
    /// # Returns
    /// Par2Set with merged recovery slices from all volumes
    ///
    /// # Example
    /// ```no_run
    /// use nntp_rs::Par2Set;
    ///
    /// let set = Par2Set::discover("/downloads", "myfile").unwrap();
    /// println!("Found {} PAR2 files", set.files.len());
    /// println!("Total recovery slices: {}", set.total_recovery_slices);
    /// ```
    pub fn discover<P: AsRef<Path>>(dir: P, base_name: &str) -> Result<Self> {
        let dir = dir.as_ref();

        // Find all .par2 files matching the base name
        let mut par2_files = Vec::new();
        let mut main_file: Option<PathBuf> = None;

        // Read directory entries
        let entries = fs::read_dir(dir)
            .map_err(|e| NntpError::InvalidResponse(format!("Failed to read directory: {}", e)))?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                NntpError::InvalidResponse(format!("Failed to read directory entry: {}", e))
            })?;

            let path = entry.path();

            // Check if it's a file with .par2 extension
            if !path.is_file() {
                continue;
            }

            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name,
                None => continue,
            };

            // Check if file matches our base name
            if !file_name.starts_with(base_name) || !file_name.ends_with(".par2") {
                continue;
            }

            // Determine if this is the main file (no .vol in name)
            if file_name.contains(".vol") {
                // This is a volume file
                par2_files.push(path);
            } else {
                // This is the main file
                main_file = Some(path.clone());
                par2_files.push(path);
            }
        }

        // Ensure we found a main file
        let main_path = main_file.ok_or_else(|| {
            NntpError::InvalidResponse(format!("No main PAR2 file found for '{}'", base_name))
        })?;

        // Parse the main file
        let main_data = fs::read(&main_path).map_err(|e| {
            NntpError::InvalidResponse(format!("Failed to read main PAR2 file: {}", e))
        })?;

        let mut main_par2 = Par2File::parse(&main_data)?;

        // Parse and merge all volume files
        for path in &par2_files {
            // Skip the main file (already parsed)
            if path == &main_path {
                continue;
            }

            let data = fs::read(path).map_err(|e| {
                NntpError::InvalidResponse(format!(
                    "Failed to read PAR2 file '{}': {}",
                    path.display(),
                    e
                ))
            })?;

            let volume = Par2File::parse(&data)?;
            main_par2.merge_recovery_slices(&volume)?;
        }

        let total_recovery_slices = main_par2.recovery_slice_count();

        Ok(Par2Set {
            main: main_par2,
            files: par2_files,
            total_recovery_slices,
        })
    }

    /// Get the recovery capacity as a percentage
    ///
    /// This compares the number of recovery slices to the total number of
    /// data slices in the PAR2 set.
    ///
    /// # Arguments
    /// * `file_data_map` - HashMap mapping file IDs to their actual data
    ///
    /// # Returns
    /// Percentage of data slices that can be recovered (0.0 to 100.0+)
    pub fn recovery_percentage(&self, file_data_map: &HashMap<[u8; 16], Vec<u8>>) -> Result<f64> {
        let summary = self.main.slice_summary(file_data_map)?;
        let total_data_slices = summary.total_data_slices;

        if total_data_slices == 0 {
            return Ok(0.0);
        }

        Ok((self.total_recovery_slices as f64 / total_data_slices as f64) * 100.0)
    }

    /// Check if the PAR2 set has enough recovery slices to repair all damaged/missing slices
    ///
    /// # Arguments
    /// * `file_data_map` - HashMap mapping file IDs to their actual data
    ///
    /// # Returns
    /// true if recovery is possible, false otherwise
    pub fn can_recover(&self, file_data_map: &HashMap<[u8; 16], Vec<u8>>) -> Result<bool> {
        let summary = self.main.slice_summary(file_data_map)?;
        let damaged_count = summary.damaged_slices.len() + summary.missing_slices.len();
        Ok(self.total_recovery_slices >= damaged_count)
    }
}

impl Default for Par2File {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a packet header from bytes
fn parse_packet_header(data: &[u8]) -> Result<PacketHeader> {
    if data.len() < 64 {
        return Err(NntpError::InvalidResponse(
            "Packet header too short".to_string(),
        ));
    }

    // Skip magic (first 8 bytes)
    let length = u64::from_le_bytes(data[8..16].try_into().unwrap());

    let mut hash = [0u8; 16];
    hash.copy_from_slice(&data[16..32]);

    let mut set_id = [0u8; 16];
    set_id.copy_from_slice(&data[32..48]);

    let mut packet_type = [0u8; 16];
    packet_type.copy_from_slice(&data[48..64]);

    Ok(PacketHeader {
        length,
        hash,
        set_id,
        packet_type,
    })
}

/// Parse Main packet body
fn parse_main_packet(data: &[u8]) -> Result<MainPacket> {
    if data.len() < 12 {
        return Err(NntpError::InvalidResponse(
            "Main packet body too short".to_string(),
        ));
    }

    let slice_size = u64::from_le_bytes(data[0..8].try_into().unwrap());
    let file_count = u32::from_le_bytes(data[8..12].try_into().unwrap());

    // Read file IDs (16 bytes each)
    let mut offset = 12;
    let mut file_ids = Vec::new();

    for _ in 0..file_count {
        if offset + 16 > data.len() {
            return Err(NntpError::InvalidResponse(
                "Main packet file ID out of bounds".to_string(),
            ));
        }
        let mut file_id = [0u8; 16];
        file_id.copy_from_slice(&data[offset..offset + 16]);
        file_ids.push(file_id);
        offset += 16;
    }

    // Non-recoverable file IDs follow (if any remaining)
    let mut non_recoverable_file_ids = Vec::new();
    while offset + 16 <= data.len() {
        let mut file_id = [0u8; 16];
        file_id.copy_from_slice(&data[offset..offset + 16]);
        non_recoverable_file_ids.push(file_id);
        offset += 16;
    }

    Ok(MainPacket {
        slice_size,
        file_count,
        file_ids,
        non_recoverable_file_ids,
    })
}

/// Parse File Description packet body
fn parse_file_description_packet(data: &[u8]) -> Result<FileDescriptionPacket> {
    if data.len() < 56 {
        return Err(NntpError::InvalidResponse(
            "File Description packet body too short".to_string(),
        ));
    }

    let mut file_id = [0u8; 16];
    file_id.copy_from_slice(&data[0..16]);

    let mut hash = [0u8; 16];
    hash.copy_from_slice(&data[16..32]);

    let mut hash_16k = [0u8; 16];
    hash_16k.copy_from_slice(&data[32..48]);

    let length = u64::from_le_bytes(data[48..56].try_into().unwrap());

    // File name is null-terminated and padded to multiple of 4
    let name_bytes = &data[56..];
    let name = match name_bytes.iter().position(|&b| b == 0) {
        Some(null_pos) => String::from_utf8_lossy(&name_bytes[..null_pos]).to_string(),
        None => String::from_utf8_lossy(name_bytes).to_string(),
    };

    Ok(FileDescriptionPacket {
        file_id,
        hash,
        hash_16k,
        length,
        name,
    })
}

/// Parse IFSC packet body
fn parse_ifsc_packet(data: &[u8]) -> Result<IfscPacket> {
    if data.len() < 16 {
        return Err(NntpError::InvalidResponse(
            "IFSC packet body too short".to_string(),
        ));
    }

    let mut file_id = [0u8; 16];
    file_id.copy_from_slice(&data[0..16]);

    // CRC32 checksums follow (4 bytes each)
    let checksum_data = &data[16..];
    if checksum_data.len() % 4 != 0 {
        return Err(NntpError::InvalidResponse(
            "IFSC packet checksum data not aligned".to_string(),
        ));
    }

    let mut checksums = Vec::new();
    for chunk in checksum_data.chunks_exact(4) {
        let crc = u32::from_le_bytes(chunk.try_into().unwrap());
        checksums.push(crc);
    }

    Ok(IfscPacket { file_id, checksums })
}

/// Parse Recovery Slice packet body
fn parse_recovery_slice_packet(data: &[u8]) -> Result<RecoverySlicePacket> {
    if data.len() < 4 {
        return Err(NntpError::InvalidResponse(
            "Recovery Slice packet body too short".to_string(),
        ));
    }

    let exponent = u32::from_le_bytes(data[0..4].try_into().unwrap());
    let recovery_data = data[4..].to_vec();

    Ok(RecoverySlicePacket {
        exponent,
        data: recovery_data,
    })
}

/// Parse Creator packet body
fn parse_creator_packet(data: &[u8]) -> Result<CreatorPacket> {
    // Client identifier is null-terminated ASCII
    let client = match data.iter().position(|&b| b == 0) {
        Some(null_pos) => String::from_utf8_lossy(&data[..null_pos]).to_string(),
        None => String::from_utf8_lossy(data).to_string(),
    };

    Ok(CreatorPacket { client })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_par2_magic() {
        assert_eq!(PAR2_MAGIC, b"PAR2\0PKT");
    }

    #[test]
    fn test_packet_type_main() {
        let bytes = b"PAR 2.0\0Main\0\0\0\0";
        let ptype = PacketType::from_bytes(bytes);
        assert_eq!(ptype, PacketType::Main);
        assert_eq!(ptype.to_bytes(), *bytes);
    }

    #[test]
    fn test_packet_type_file_description() {
        let bytes = b"PAR 2.0\0FileDesc";
        let ptype = PacketType::from_bytes(bytes);
        assert_eq!(ptype, PacketType::FileDescription);
        assert_eq!(ptype.to_bytes(), *bytes);
    }

    #[test]
    fn test_packet_type_ifsc() {
        let bytes = b"PAR 2.0\0IFSC\0\0\0\0";
        let ptype = PacketType::from_bytes(bytes);
        assert_eq!(ptype, PacketType::Ifsc);
        assert_eq!(ptype.to_bytes(), *bytes);
    }

    #[test]
    fn test_packet_type_recovery_slice() {
        let bytes = b"PAR 2.0\0RecvSlic";
        let ptype = PacketType::from_bytes(bytes);
        assert_eq!(ptype, PacketType::RecoverySlice);
        assert_eq!(ptype.to_bytes(), *bytes);
    }

    #[test]
    fn test_packet_type_creator() {
        let bytes = b"PAR 2.0\0Creator\0";
        let ptype = PacketType::from_bytes(bytes);
        assert_eq!(ptype, PacketType::Creator);
        assert_eq!(ptype.to_bytes(), *bytes);
    }

    #[test]
    fn test_packet_type_unknown() {
        let bytes = b"PAR 2.0\0Unknown\0";
        let ptype = PacketType::from_bytes(bytes);
        match ptype {
            PacketType::Unknown(_) => {}
            _ => panic!("Expected Unknown packet type"),
        }
    }

    #[test]
    fn test_par2_file_new() {
        let par2 = Par2File::new();
        assert_eq!(par2.set_id, [0; 16]);
        assert!(par2.main.is_none());
        assert!(par2.file_descriptions.is_empty());
        assert!(par2.ifsc_packets.is_empty());
        assert!(par2.recovery_slices.is_empty());
        assert!(par2.creator.is_none());
    }

    #[test]
    fn test_parse_packet_header_too_short() {
        let data = vec![0u8; 32]; // Only 32 bytes, need 64
        let result = parse_packet_header(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_main_packet_too_short() {
        let data = vec![0u8; 4]; // Only 4 bytes, need at least 12
        let result = parse_main_packet(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_file_description_packet_too_short() {
        let data = vec![0u8; 32]; // Only 32 bytes, need at least 56
        let result = parse_file_description_packet(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_ifsc_packet_too_short() {
        let data = vec![0u8; 8]; // Only 8 bytes, need at least 16
        let result = parse_ifsc_packet(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_recovery_slice_packet_too_short() {
        let data = vec![0u8; 2]; // Only 2 bytes, need at least 4
        let result = parse_recovery_slice_packet(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_creator_packet_empty() {
        let data = b"par2cmdline\0";
        let result = parse_creator_packet(data).unwrap();
        assert_eq!(result.client, "par2cmdline");
    }

    #[test]
    fn test_parse_creator_packet_no_null() {
        let data = b"testclient";
        let result = parse_creator_packet(data).unwrap();
        assert_eq!(result.client, "testclient");
    }

    // File verification tests

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

        // Create a file description
        let file_id = [1u8; 16];
        let file_desc = FileDescriptionPacket {
            file_id,
            hash: [0u8; 16],
            hash_16k: [0u8; 16],
            length: 1000,
            name: "test.bin".to_string(),
        };
        par2.file_descriptions.insert(file_id, file_desc);

        // Verify with empty data (missing file)
        let result = par2.verify_file(&[], &file_id).unwrap();
        assert_eq!(result.status, FileStatus::Missing);
        assert_eq!(result.filename, "test.bin");
        assert_eq!(result.expected_size, 1000);
        assert!(result.hash_match.is_none());
    }

    #[test]
    fn test_verify_file_complete() {
        let mut par2 = Par2File::new();

        // Create test data
        let file_data = b"Hello, World!";
        let file_id = [2u8; 16];

        // Calculate MD5 hashes
        let mut hasher = Md5::new();
        hasher.update(file_data);
        let hash: [u8; 16] = hasher.finalize().into();

        let mut hasher_16k = Md5::new();
        hasher_16k.update(file_data);
        let hash_16k: [u8; 16] = hasher_16k.finalize().into();

        // Create file description with correct hashes
        let file_desc = FileDescriptionPacket {
            file_id,
            hash,
            hash_16k,
            length: file_data.len() as u64,
            name: "hello.txt".to_string(),
        };
        par2.file_descriptions.insert(file_id, file_desc);

        // Verify file
        let result = par2.verify_file(file_data, &file_id).unwrap();
        assert_eq!(result.status, FileStatus::Complete);
        assert_eq!(result.hash_match, Some(true));
        assert_eq!(result.hash_16k_match, Some(true));
    }

    #[test]
    fn test_verify_file_hash_mismatch() {
        let mut par2 = Par2File::new();

        // Create test data
        let file_data = b"Hello, World!";
        let file_id = [3u8; 16];

        // Use wrong hash
        let wrong_hash = [0xFFu8; 16];

        let file_desc = FileDescriptionPacket {
            file_id,
            hash: wrong_hash,
            hash_16k: wrong_hash,
            length: file_data.len() as u64,
            name: "corrupted.txt".to_string(),
        };
        par2.file_descriptions.insert(file_id, file_desc);

        // Verify file
        let result = par2.verify_file(file_data, &file_id).unwrap();
        match result.status {
            FileStatus::Damaged(_) => {} // Expected
            _ => panic!("Expected Damaged status for hash mismatch"),
        }
        assert_eq!(result.hash_match, Some(false));
    }

    #[test]
    fn test_verify_file_size_mismatch() {
        let mut par2 = Par2File::new();

        let file_data = b"Hello";
        let file_id = [4u8; 16];

        let file_desc = FileDescriptionPacket {
            file_id,
            hash: [0u8; 16],
            hash_16k: [0u8; 16],
            length: 1000, // Wrong size
            name: "size_mismatch.txt".to_string(),
        };
        par2.file_descriptions.insert(file_id, file_desc);

        let result = par2.verify_file(file_data, &file_id).unwrap();
        match result.status {
            FileStatus::Damaged(_) => {} // Expected
            _ => panic!("Expected Damaged status for size mismatch"),
        }
        assert_eq!(result.hash_match, Some(false));
    }

    #[test]
    fn test_verify_slices_all_good() {
        let mut par2 = Par2File::new();
        par2.main = Some(MainPacket {
            slice_size: 5,
            file_count: 1,
            file_ids: vec![[5u8; 16]],
            non_recoverable_file_ids: vec![],
        });

        let file_data = b"Hello World!"; // 12 bytes = 3 slices of 5, 5, 2

        // Calculate CRC32 for each slice
        let mut crc1 = Crc32::new();
        crc1.update(b"Hello");
        let checksum1 = crc1.finalize();

        let mut crc2 = Crc32::new();
        crc2.update(b" Worl");
        let checksum2 = crc2.finalize();

        let mut crc3 = Crc32::new();
        crc3.update(b"d!");
        let checksum3 = crc3.finalize();

        let ifsc = IfscPacket {
            file_id: [5u8; 16],
            checksums: vec![checksum1, checksum2, checksum3],
        };

        let damaged = par2.verify_slices(file_data, &ifsc).unwrap();
        assert!(damaged.is_empty(), "Expected no damaged slices");
    }

    #[test]
    fn test_verify_slices_with_damage() {
        let mut par2 = Par2File::new();
        par2.main = Some(MainPacket {
            slice_size: 4,
            file_count: 1,
            file_ids: vec![[6u8; 16]],
            non_recoverable_file_ids: vec![],
        });

        let file_data = b"TestData"; // 8 bytes = 2 slices of 4 bytes each

        // Calculate correct CRC for first slice, wrong CRC for second
        let mut crc1 = Crc32::new();
        crc1.update(b"Test");
        let checksum1 = crc1.finalize();

        let wrong_checksum = 0xDEADBEEF;

        let ifsc = IfscPacket {
            file_id: [6u8; 16],
            checksums: vec![checksum1, wrong_checksum],
        };

        let damaged = par2.verify_slices(file_data, &ifsc).unwrap();
        assert_eq!(damaged, vec![1], "Expected slice 1 to be damaged");
    }

    #[test]
    fn test_verify_all_files() {
        let mut par2 = Par2File::new();

        // File 1: Complete
        let file1_id = [10u8; 16];
        let file1_data = b"File 1 data";

        let mut hash1 = Md5::new();
        hash1.update(file1_data);
        let file1_hash: [u8; 16] = hash1.finalize().into();

        par2.file_descriptions.insert(
            file1_id,
            FileDescriptionPacket {
                file_id: file1_id,
                hash: file1_hash,
                hash_16k: file1_hash,
                length: file1_data.len() as u64,
                name: "file1.txt".to_string(),
            },
        );

        // File 2: Missing
        let file2_id = [11u8; 16];
        par2.file_descriptions.insert(
            file2_id,
            FileDescriptionPacket {
                file_id: file2_id,
                hash: [0u8; 16],
                hash_16k: [0u8; 16],
                length: 100,
                name: "file2.txt".to_string(),
            },
        );

        // Create file data map
        let mut file_data_map = HashMap::new();
        file_data_map.insert(file1_id, file1_data.to_vec());
        // file2 not included (missing)

        // Verify all
        let results = par2.verify_all(&file_data_map).unwrap();
        assert_eq!(results.len(), 2);

        // Check file1 is complete
        let file1_result = results.iter().find(|r| r.file_id == file1_id).unwrap();
        assert_eq!(file1_result.status, FileStatus::Complete);

        // Check file2 is missing
        let file2_result = results.iter().find(|r| r.file_id == file2_id).unwrap();
        assert_eq!(file2_result.status, FileStatus::Missing);
    }

    #[test]
    fn test_slice_size() {
        let mut par2 = Par2File::new();
        assert!(par2.slice_size().is_none());

        par2.main = Some(MainPacket {
            slice_size: 1024,
            file_count: 1,
            file_ids: vec![[0u8; 16]],
            non_recoverable_file_ids: vec![],
        });

        assert_eq!(par2.slice_size(), Some(1024));
    }

    #[test]
    fn test_verify_file_not_found() {
        let par2 = Par2File::new();
        let file_id = [99u8; 16];
        let result = par2.verify_file(b"data", &file_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_large_file_first_16k() {
        let mut par2 = Par2File::new();

        // Create a large file (20KB)
        let mut file_data = vec![0x42u8; 20480];
        // Make first 16k different
        for (i, byte) in file_data.iter_mut().enumerate().take(16384) {
            *byte = (i % 256) as u8;
        }

        let file_id = [20u8; 16];

        // Calculate full hash
        let mut hasher = Md5::new();
        hasher.update(&file_data);
        let hash: [u8; 16] = hasher.finalize().into();

        // Calculate 16k hash
        let mut hasher_16k = Md5::new();
        hasher_16k.update(&file_data[..16384]);
        let hash_16k: [u8; 16] = hasher_16k.finalize().into();

        par2.file_descriptions.insert(
            file_id,
            FileDescriptionPacket {
                file_id,
                hash,
                hash_16k,
                length: file_data.len() as u64,
                name: "large.bin".to_string(),
            },
        );

        let result = par2.verify_file(&file_data, &file_id).unwrap();
        assert_eq!(result.status, FileStatus::Complete);
        assert_eq!(result.hash_match, Some(true));
        assert_eq!(result.hash_16k_match, Some(true));
    }

    // Slice management tests

    #[test]
    fn test_recovery_slice_count() {
        let mut par2 = Par2File::new();
        assert_eq!(par2.recovery_slice_count(), 0);

        par2.recovery_slices.push(RecoverySlicePacket {
            exponent: 0,
            data: vec![1, 2, 3],
        });
        assert_eq!(par2.recovery_slice_count(), 1);

        par2.recovery_slices.push(RecoverySlicePacket {
            exponent: 1,
            data: vec![4, 5, 6],
        });
        assert_eq!(par2.recovery_slice_count(), 2);
    }

    #[test]
    fn test_map_slices_no_main_packet() {
        let par2 = Par2File::new();
        let result = par2.map_slices();
        assert!(result.is_err());
    }

    #[test]
    fn test_map_slices_single_file() {
        let mut par2 = Par2File::new();
        let file_id = [1u8; 16];

        // Set up main packet with slice size 100
        par2.main = Some(MainPacket {
            slice_size: 100,
            file_count: 1,
            file_ids: vec![file_id],
            non_recoverable_file_ids: vec![],
        });

        // File is 250 bytes (3 slices: 100, 100, 50)
        par2.file_descriptions.insert(
            file_id,
            FileDescriptionPacket {
                file_id,
                hash: [0u8; 16],
                hash_16k: [0u8; 16],
                length: 250,
                name: "file1.bin".to_string(),
            },
        );

        let mappings = par2.map_slices().unwrap();
        assert_eq!(mappings.len(), 3);

        // Check first slice
        assert_eq!(mappings[0].file_id, file_id);
        assert_eq!(mappings[0].filename, "file1.bin");
        assert_eq!(mappings[0].file_slice_index, 0);
        assert_eq!(mappings[0].offset, 0);
        assert_eq!(mappings[0].size, 100);

        // Check second slice
        assert_eq!(mappings[1].file_slice_index, 1);
        assert_eq!(mappings[1].offset, 100);
        assert_eq!(mappings[1].size, 100);

        // Check third slice (partial)
        assert_eq!(mappings[2].file_slice_index, 2);
        assert_eq!(mappings[2].offset, 200);
        assert_eq!(mappings[2].size, 50);
    }

    #[test]
    fn test_map_slices_multiple_files() {
        let mut par2 = Par2File::new();
        let file_id1 = [1u8; 16];
        let file_id2 = [2u8; 16];

        // Set up main packet
        par2.main = Some(MainPacket {
            slice_size: 50,
            file_count: 2,
            file_ids: vec![file_id1, file_id2],
            non_recoverable_file_ids: vec![],
        });

        // File 1: 120 bytes (3 slices: 50, 50, 20)
        par2.file_descriptions.insert(
            file_id1,
            FileDescriptionPacket {
                file_id: file_id1,
                hash: [0u8; 16],
                hash_16k: [0u8; 16],
                length: 120,
                name: "file1.bin".to_string(),
            },
        );

        // File 2: 75 bytes (2 slices: 50, 25)
        par2.file_descriptions.insert(
            file_id2,
            FileDescriptionPacket {
                file_id: file_id2,
                hash: [0u8; 16],
                hash_16k: [0u8; 16],
                length: 75,
                name: "file2.bin".to_string(),
            },
        );

        let mappings = par2.map_slices().unwrap();
        assert_eq!(mappings.len(), 5); // 3 + 2

        // Check file 1 slices
        assert_eq!(mappings[0].file_id, file_id1);
        assert_eq!(mappings[0].filename, "file1.bin");
        assert_eq!(mappings[1].file_id, file_id1);
        assert_eq!(mappings[2].file_id, file_id1);

        // Check file 2 slices
        assert_eq!(mappings[3].file_id, file_id2);
        assert_eq!(mappings[3].filename, "file2.bin");
        assert_eq!(mappings[4].file_id, file_id2);
    }

    #[test]
    fn test_map_slices_empty_file() {
        let mut par2 = Par2File::new();
        let file_id = [1u8; 16];

        par2.main = Some(MainPacket {
            slice_size: 100,
            file_count: 1,
            file_ids: vec![file_id],
            non_recoverable_file_ids: vec![],
        });

        // Empty file (0 bytes)
        par2.file_descriptions.insert(
            file_id,
            FileDescriptionPacket {
                file_id,
                hash: [0u8; 16],
                hash_16k: [0u8; 16],
                length: 0,
                name: "empty.bin".to_string(),
            },
        );

        let mappings = par2.map_slices().unwrap();
        assert_eq!(mappings.len(), 0);
    }

    #[test]
    fn test_map_slices_exact_multiple() {
        let mut par2 = Par2File::new();
        let file_id = [1u8; 16];

        par2.main = Some(MainPacket {
            slice_size: 100,
            file_count: 1,
            file_ids: vec![file_id],
            non_recoverable_file_ids: vec![],
        });

        // File is exactly 200 bytes (2 slices of 100 each)
        par2.file_descriptions.insert(
            file_id,
            FileDescriptionPacket {
                file_id,
                hash: [0u8; 16],
                hash_16k: [0u8; 16],
                length: 200,
                name: "exact.bin".to_string(),
            },
        );

        let mappings = par2.map_slices().unwrap();
        assert_eq!(mappings.len(), 2);
        assert_eq!(mappings[0].size, 100);
        assert_eq!(mappings[1].size, 100);
    }

    #[test]
    fn test_identify_damaged_slices_all_good() {
        let mut par2 = Par2File::new();
        let file_id = [1u8; 16];

        par2.main = Some(MainPacket {
            slice_size: 5,
            file_count: 1,
            file_ids: vec![file_id],
            non_recoverable_file_ids: vec![],
        });

        par2.file_descriptions.insert(
            file_id,
            FileDescriptionPacket {
                file_id,
                hash: [0u8; 16],
                hash_16k: [0u8; 16],
                length: 12,
                name: "test.bin".to_string(),
            },
        );

        let file_data = b"Hello World!"; // 12 bytes = 3 slices

        // Calculate CRC32 for each slice
        let mut crc1 = Crc32::new();
        crc1.update(b"Hello");
        let checksum1 = crc1.finalize();

        let mut crc2 = Crc32::new();
        crc2.update(b" Worl");
        let checksum2 = crc2.finalize();

        let mut crc3 = Crc32::new();
        crc3.update(b"d!");
        let checksum3 = crc3.finalize();

        par2.ifsc_packets.insert(
            file_id,
            IfscPacket {
                file_id,
                checksums: vec![checksum1, checksum2, checksum3],
            },
        );

        let mut file_data_map = HashMap::new();
        file_data_map.insert(file_id, file_data.to_vec());

        let (damaged, missing) = par2.identify_damaged_slices(&file_data_map).unwrap();
        assert!(damaged.is_empty());
        assert!(missing.is_empty());
    }

    #[test]
    fn test_identify_damaged_slices_with_damage() {
        let mut par2 = Par2File::new();
        let file_id = [1u8; 16];

        par2.main = Some(MainPacket {
            slice_size: 4,
            file_count: 1,
            file_ids: vec![file_id],
            non_recoverable_file_ids: vec![],
        });

        par2.file_descriptions.insert(
            file_id,
            FileDescriptionPacket {
                file_id,
                hash: [0u8; 16],
                hash_16k: [0u8; 16],
                length: 8,
                name: "test.bin".to_string(),
            },
        );

        let file_data = b"TestData"; // 8 bytes = 2 slices

        // Calculate correct CRC for first slice, wrong CRC for second
        let mut crc1 = Crc32::new();
        crc1.update(b"Test");
        let checksum1 = crc1.finalize();

        let wrong_checksum = 0xDEADBEEF;

        par2.ifsc_packets.insert(
            file_id,
            IfscPacket {
                file_id,
                checksums: vec![checksum1, wrong_checksum],
            },
        );

        let mut file_data_map = HashMap::new();
        file_data_map.insert(file_id, file_data.to_vec());

        let (damaged, missing) = par2.identify_damaged_slices(&file_data_map).unwrap();
        assert_eq!(damaged, vec![1]); // Global slice index 1 is damaged
        assert!(missing.is_empty());
    }

    #[test]
    fn test_identify_damaged_slices_missing_file() {
        let mut par2 = Par2File::new();
        let file_id = [1u8; 16];

        par2.main = Some(MainPacket {
            slice_size: 100,
            file_count: 1,
            file_ids: vec![file_id],
            non_recoverable_file_ids: vec![],
        });

        par2.file_descriptions.insert(
            file_id,
            FileDescriptionPacket {
                file_id,
                hash: [0u8; 16],
                hash_16k: [0u8; 16],
                length: 250, // 3 slices
                name: "test.bin".to_string(),
            },
        );

        // File is missing from map
        let file_data_map = HashMap::new();

        let (damaged, missing) = par2.identify_damaged_slices(&file_data_map).unwrap();
        assert!(damaged.is_empty());
        assert_eq!(missing, vec![0, 1, 2]); // All 3 slices are missing
    }

    #[test]
    fn test_identify_damaged_slices_multiple_files() {
        let mut par2 = Par2File::new();
        let file_id1 = [1u8; 16];
        let file_id2 = [2u8; 16];

        par2.main = Some(MainPacket {
            slice_size: 5,
            file_count: 2,
            file_ids: vec![file_id1, file_id2],
            non_recoverable_file_ids: vec![],
        });

        // File 1: 10 bytes (2 slices)
        par2.file_descriptions.insert(
            file_id1,
            FileDescriptionPacket {
                file_id: file_id1,
                hash: [0u8; 16],
                hash_16k: [0u8; 16],
                length: 10,
                name: "file1.bin".to_string(),
            },
        );

        // File 2: 7 bytes (2 slices)
        par2.file_descriptions.insert(
            file_id2,
            FileDescriptionPacket {
                file_id: file_id2,
                hash: [0u8; 16],
                hash_16k: [0u8; 16],
                length: 7,
                name: "file2.bin".to_string(),
            },
        );

        let file1_data = b"HelloWorld";
        let file2_data = b"Testing";

        // CRCs for file1
        let mut crc1_1 = Crc32::new();
        crc1_1.update(b"Hello");
        let mut crc1_2 = Crc32::new();
        crc1_2.update(b"World");

        par2.ifsc_packets.insert(
            file_id1,
            IfscPacket {
                file_id: file_id1,
                checksums: vec![crc1_1.finalize(), crc1_2.finalize()],
            },
        );

        // CRCs for file2 - second slice has wrong checksum
        let mut crc2_1 = Crc32::new();
        crc2_1.update(b"Testi");

        par2.ifsc_packets.insert(
            file_id2,
            IfscPacket {
                file_id: file_id2,
                checksums: vec![crc2_1.finalize(), 0xBADBAD],
            },
        );

        let mut file_data_map = HashMap::new();
        file_data_map.insert(file_id1, file1_data.to_vec());
        file_data_map.insert(file_id2, file2_data.to_vec());

        let (damaged, missing) = par2.identify_damaged_slices(&file_data_map).unwrap();
        assert_eq!(damaged, vec![3]); // Global slice 3 (file2's second slice) is damaged
        assert!(missing.is_empty());
    }

    #[test]
    fn test_slice_summary() {
        let mut par2 = Par2File::new();
        let file_id = [1u8; 16];

        par2.main = Some(MainPacket {
            slice_size: 100,
            file_count: 1,
            file_ids: vec![file_id],
            non_recoverable_file_ids: vec![],
        });

        par2.file_descriptions.insert(
            file_id,
            FileDescriptionPacket {
                file_id,
                hash: [0u8; 16],
                hash_16k: [0u8; 16],
                length: 250, // 3 slices
                name: "test.bin".to_string(),
            },
        );

        // Add 2 recovery slices
        par2.recovery_slices.push(RecoverySlicePacket {
            exponent: 0,
            data: vec![0; 100],
        });
        par2.recovery_slices.push(RecoverySlicePacket {
            exponent: 1,
            data: vec![0; 100],
        });

        let file_data_map = HashMap::new(); // File is missing

        let summary = par2.slice_summary(&file_data_map).unwrap();
        assert_eq!(summary.total_data_slices, 3);
        assert_eq!(summary.recovery_slice_count, 2);
        assert_eq!(summary.slice_mappings.len(), 3);
        assert_eq!(summary.missing_slices, vec![0, 1, 2]);
        assert!(summary.damaged_slices.is_empty());
    }

    #[test]
    fn test_slice_summary_recoverable() {
        let mut par2 = Par2File::new();
        let file_id = [1u8; 16];

        par2.main = Some(MainPacket {
            slice_size: 50,
            file_count: 1,
            file_ids: vec![file_id],
            non_recoverable_file_ids: vec![],
        });

        par2.file_descriptions.insert(
            file_id,
            FileDescriptionPacket {
                file_id,
                hash: [0u8; 16],
                hash_16k: [0u8; 16],
                length: 100, // 2 slices
                name: "test.bin".to_string(),
            },
        );

        // Add 3 recovery slices (more than enough to recover 1 missing slice)
        for i in 0..3 {
            par2.recovery_slices.push(RecoverySlicePacket {
                exponent: i,
                data: vec![0; 50],
            });
        }

        let file_data_map = HashMap::new(); // File is missing

        let summary = par2.slice_summary(&file_data_map).unwrap();
        assert_eq!(summary.total_data_slices, 2);
        assert_eq!(summary.recovery_slice_count, 3);
        // With 2 missing slices and 3 recovery slices, we can recover
        assert!(summary.recovery_slice_count >= summary.missing_slices.len());
    }

    // PAR2 set discovery tests

    #[test]
    fn test_merge_recovery_slices_success() {
        let set_id = [1u8; 16];

        let mut par2_main = Par2File::new();
        par2_main.set_id = set_id;
        par2_main.recovery_slices.push(RecoverySlicePacket {
            exponent: 0,
            data: vec![1, 2, 3],
        });

        let mut par2_volume = Par2File::new();
        par2_volume.set_id = set_id;
        par2_volume.recovery_slices.push(RecoverySlicePacket {
            exponent: 1,
            data: vec![4, 5, 6],
        });

        assert_eq!(par2_main.recovery_slice_count(), 1);
        par2_main.merge_recovery_slices(&par2_volume).unwrap();
        assert_eq!(par2_main.recovery_slice_count(), 2);
    }

    #[test]
    fn test_merge_recovery_slices_mismatched_set_id() {
        let mut par2_main = Par2File::new();
        par2_main.set_id = [1u8; 16];

        let mut par2_volume = Par2File::new();
        par2_volume.set_id = [2u8; 16];

        let result = par2_main.merge_recovery_slices(&par2_volume);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("different set IDs"));
    }

    #[test]
    fn test_merge_recovery_slices_empty() {
        let set_id = [1u8; 16];

        let mut par2_main = Par2File::new();
        par2_main.set_id = set_id;
        par2_main.recovery_slices.push(RecoverySlicePacket {
            exponent: 0,
            data: vec![1, 2, 3],
        });

        let mut par2_volume = Par2File::new();
        par2_volume.set_id = set_id;
        // No recovery slices

        assert_eq!(par2_main.recovery_slice_count(), 1);
        par2_main.merge_recovery_slices(&par2_volume).unwrap();
        assert_eq!(par2_main.recovery_slice_count(), 1); // Unchanged
    }

    #[test]
    fn test_merge_recovery_slices_multiple() {
        let set_id = [1u8; 16];

        let mut par2_main = Par2File::new();
        par2_main.set_id = set_id;

        let mut par2_volume = Par2File::new();
        par2_volume.set_id = set_id;
        par2_volume.recovery_slices.push(RecoverySlicePacket {
            exponent: 0,
            data: vec![1, 2, 3],
        });
        par2_volume.recovery_slices.push(RecoverySlicePacket {
            exponent: 1,
            data: vec![4, 5, 6],
        });
        par2_volume.recovery_slices.push(RecoverySlicePacket {
            exponent: 2,
            data: vec![7, 8, 9],
        });

        assert_eq!(par2_main.recovery_slice_count(), 0);
        par2_main.merge_recovery_slices(&par2_volume).unwrap();
        assert_eq!(par2_main.recovery_slice_count(), 3);
    }

    #[test]
    fn test_par2_set_recovery_percentage() {
        let mut par2 = Par2File::new();
        let set_id = [1u8; 16];
        let file_id = [2u8; 16];

        par2.set_id = set_id;
        par2.main = Some(MainPacket {
            slice_size: 100,
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
                length: 200, // 2 slices
                name: "test.dat".to_string(),
            },
        );

        // Add 1 recovery slice (50% recovery)
        par2.recovery_slices.push(RecoverySlicePacket {
            exponent: 0,
            data: vec![0; 100],
        });

        let set = Par2Set {
            main: par2,
            files: vec![],
            total_recovery_slices: 1,
        };

        let file_data_map = HashMap::new();
        let percentage = set.recovery_percentage(&file_data_map).unwrap();
        assert_eq!(percentage, 50.0); // 1 recovery slice / 2 data slices = 50%
    }

    #[test]
    fn test_par2_set_recovery_percentage_over_100() {
        let mut par2 = Par2File::new();
        let set_id = [1u8; 16];
        let file_id = [2u8; 16];

        par2.set_id = set_id;
        par2.main = Some(MainPacket {
            slice_size: 100,
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
                length: 100, // 1 slice
                name: "test.dat".to_string(),
            },
        );

        // Add 2 recovery slices (200% recovery)
        par2.recovery_slices.push(RecoverySlicePacket {
            exponent: 0,
            data: vec![0; 100],
        });
        par2.recovery_slices.push(RecoverySlicePacket {
            exponent: 1,
            data: vec![0; 100],
        });

        let set = Par2Set {
            main: par2,
            files: vec![],
            total_recovery_slices: 2,
        };

        let file_data_map = HashMap::new();
        let percentage = set.recovery_percentage(&file_data_map).unwrap();
        assert_eq!(percentage, 200.0); // 2 recovery slices / 1 data slice = 200%
    }

    #[test]
    fn test_par2_set_recovery_percentage_no_files() {
        let mut par2 = Par2File::new();
        let set_id = [1u8; 16];

        par2.set_id = set_id;
        // Main packet with no files
        par2.main = Some(MainPacket {
            slice_size: 100,
            file_count: 0,
            file_ids: vec![],
            non_recoverable_file_ids: vec![],
        });

        let set = Par2Set {
            main: par2,
            files: vec![],
            total_recovery_slices: 5,
        };

        let file_data_map = HashMap::new();
        let percentage = set.recovery_percentage(&file_data_map).unwrap();
        assert_eq!(percentage, 0.0); // No data slices = 0%
    }

    #[test]
    fn test_par2_set_can_recover_yes() {
        let mut par2 = Par2File::new();
        let set_id = [1u8; 16];
        let file_id = [2u8; 16];

        par2.set_id = set_id;
        par2.main = Some(MainPacket {
            slice_size: 5,
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
                length: 10, // 2 slices
                name: "test.dat".to_string(),
            },
        );

        par2.ifsc_packets.insert(
            file_id,
            IfscPacket {
                file_id,
                checksums: vec![0x12345678, 0xabcdef00], // 2 slices
            },
        );

        // Add 2 recovery slices (enough to recover all data)
        par2.recovery_slices.push(RecoverySlicePacket {
            exponent: 0,
            data: vec![0; 5],
        });
        par2.recovery_slices.push(RecoverySlicePacket {
            exponent: 1,
            data: vec![0; 5],
        });

        let set = Par2Set {
            main: par2,
            files: vec![],
            total_recovery_slices: 2,
        };

        let file_data_map = HashMap::new(); // File is missing
        let can_recover = set.can_recover(&file_data_map).unwrap();
        assert!(can_recover); // 2 recovery slices >= 2 missing slices
    }

    #[test]
    fn test_par2_set_can_recover_no() {
        let mut par2 = Par2File::new();
        let set_id = [1u8; 16];
        let file_id = [2u8; 16];

        par2.set_id = set_id;
        par2.main = Some(MainPacket {
            slice_size: 5,
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
                length: 10, // 2 slices
                name: "test.dat".to_string(),
            },
        );

        par2.ifsc_packets.insert(
            file_id,
            IfscPacket {
                file_id,
                checksums: vec![0x12345678, 0xabcdef00], // 2 slices
            },
        );

        // Add only 1 recovery slice (not enough)
        par2.recovery_slices.push(RecoverySlicePacket {
            exponent: 0,
            data: vec![0; 5],
        });

        let set = Par2Set {
            main: par2,
            files: vec![],
            total_recovery_slices: 1,
        };

        let file_data_map = HashMap::new(); // File is missing
        let can_recover = set.can_recover(&file_data_map).unwrap();
        assert!(!can_recover); // 1 recovery slice < 2 missing slices
    }
}

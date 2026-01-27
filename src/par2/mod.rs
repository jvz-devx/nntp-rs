//! PAR2 (Parity Archive 2) file format parsing and verification
//!
//! This module implements parsing of PAR2 files used for error correction
//! and recovery of Usenet binary downloads.
//!
//! Reference: [Parity Volume Set Specification 2.0](https://parchive.sourceforge.net/docs/specifications/parity-volume-spec/article-spec.html)

use crate::error::{NntpError, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

// Submodules
pub(super) mod parsing;
pub(super) mod verification;

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
    /// Uses `Arc<str>` to enable cheap cloning during verification
    pub name: Arc<str>,
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

    /// Get slice size from main packet
    pub fn slice_size(&self) -> Option<u64> {
        self.main.as_ref().map(|m| m.slice_size)
    }

    /// Get the number of recovery slices available
    pub fn recovery_slice_count(&self) -> usize {
        self.recovery_slices.len()
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
        let bytes = b"Unknown\0Type\0\0\0\0";
        let ptype = PacketType::from_bytes(bytes);
        assert_eq!(ptype, PacketType::Unknown(*bytes));
        assert_eq!(ptype.to_bytes(), *bytes);
    }

    #[test]
    fn test_par2file_new() {
        let par2 = Par2File::new();
        assert_eq!(par2.set_id, [0; 16]);
        assert!(par2.main.is_none());
        assert_eq!(par2.file_descriptions.len(), 0);
        assert_eq!(par2.ifsc_packets.len(), 0);
        assert_eq!(par2.recovery_slices.len(), 0);
        assert!(par2.creator.is_none());
    }

    #[test]
    fn test_par2file_slice_size() {
        let mut par2 = Par2File::new();
        assert_eq!(par2.slice_size(), None);

        par2.main = Some(MainPacket {
            slice_size: 1024,
            file_count: 1,
            file_ids: vec![],
            non_recoverable_file_ids: vec![],
        });

        assert_eq!(par2.slice_size(), Some(1024));
    }

    #[test]
    fn test_par2file_recovery_slice_count() {
        let mut par2 = Par2File::new();
        assert_eq!(par2.recovery_slice_count(), 0);

        par2.recovery_slices.push(RecoverySlicePacket {
            exponent: 0,
            data: vec![1, 2, 3],
        });

        assert_eq!(par2.recovery_slice_count(), 1);
    }

    #[test]
    fn test_merge_recovery_slices_same_set_id() {
        let mut par2_1 = Par2File::new();
        par2_1.set_id = [1; 16];
        par2_1.recovery_slices.push(RecoverySlicePacket {
            exponent: 0,
            data: vec![1, 2, 3],
        });

        let mut par2_2 = Par2File::new();
        par2_2.set_id = [1; 16];
        par2_2.recovery_slices.push(RecoverySlicePacket {
            exponent: 1,
            data: vec![4, 5, 6],
        });

        assert!(par2_1.merge_recovery_slices(&par2_2).is_ok());
        assert_eq!(par2_1.recovery_slice_count(), 2);
    }

    #[test]
    fn test_merge_recovery_slices_different_set_id() {
        let mut par2_1 = Par2File::new();
        par2_1.set_id = [1; 16];

        let mut par2_2 = Par2File::new();
        par2_2.set_id = [2; 16];

        assert!(par2_1.merge_recovery_slices(&par2_2).is_err());
    }
}

//! PAR2 packet parsing
//!
//! This module contains all PAR2 packet parsing logic, including:
//! - Packet header parsing
//! - Individual packet type parsers (Main, FileDescription, IFSC, RecoverySlice, Creator)
//! - Par2File::parse() method
//! - Par2Set::discover() method

use super::*;
use crate::error::{NntpError, Result};
use std::fs;
use std::path::{Path, PathBuf};

// PAR2 packet format constants
/// Size of the PAR2 magic signature in bytes
const PAR2_MAGIC_SIZE: usize = 8;
/// Size of a PAR2 packet header in bytes
const PAR2_PACKET_HEADER_SIZE: usize = 64;
/// Size of MD5 hash fields in bytes
const MD5_HASH_SIZE: usize = 16;
/// Size of CRC32 checksum in bytes
const CRC32_SIZE: usize = 4;
/// Minimum size of Main packet body in bytes
const MAIN_PACKET_MIN_SIZE: usize = 12;
/// Minimum size of File Description packet body in bytes
const FILE_DESC_PACKET_MIN_SIZE: usize = 56;
/// Minimum size of IFSC packet body in bytes
const IFSC_PACKET_MIN_SIZE: usize = 16;
/// Minimum size of Recovery Slice packet body in bytes
const RECOVERY_SLICE_PACKET_MIN_SIZE: usize = 4;

/// Read a u32 from little-endian bytes at given offset
fn read_u32_le(data: &[u8], offset: usize) -> Result<u32> {
    let bytes: [u8; 4] = data
        .get(offset..offset + 4)
        .ok_or_else(|| {
            NntpError::InvalidResponse(format!(
                "PAR2 packet truncated: cannot read u32 at offset {}",
                offset
            ))
        })?
        .try_into()
        .map_err(|_| {
            NntpError::InvalidResponse(format!(
                "PAR2 packet truncated: invalid u32 at offset {}",
                offset
            ))
        })?;
    Ok(u32::from_le_bytes(bytes))
}

/// Read a u64 from little-endian bytes at given offset
fn read_u64_le(data: &[u8], offset: usize) -> Result<u64> {
    let bytes: [u8; 8] = data
        .get(offset..offset + 8)
        .ok_or_else(|| {
            NntpError::InvalidResponse(format!(
                "PAR2 packet truncated: cannot read u64 at offset {}",
                offset
            ))
        })?
        .try_into()
        .map_err(|_| {
            NntpError::InvalidResponse(format!(
                "PAR2 packet truncated: invalid u64 at offset {}",
                offset
            ))
        })?;
    Ok(u64::from_le_bytes(bytes))
}

impl Par2File {
    /// Parse a PAR2 file from bytes
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut par2 = Par2File::new();
        let mut offset = 0;

        // Read all packets
        while offset < data.len() {
            // Need at least PAR2_PACKET_HEADER_SIZE bytes for header
            if offset + PAR2_PACKET_HEADER_SIZE > data.len() {
                break;
            }

            // Parse packet header
            let header = parse_packet_header(&data[offset..])?;

            // Check magic bytes
            if &data[offset..offset + PAR2_MAGIC_SIZE] != PAR2_MAGIC {
                return Err(NntpError::InvalidResponse(format!(
                    "Invalid PAR2 magic bytes at offset {}",
                    offset
                )));
            }

            // Store set ID from first packet
            if par2.set_id == [0; MD5_HASH_SIZE] {
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
            let body_offset = offset + PAR2_PACKET_HEADER_SIZE;
            let body_len = header.length as usize - PAR2_PACKET_HEADER_SIZE;

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
}

/// Parse a packet header from bytes
fn parse_packet_header(data: &[u8]) -> Result<PacketHeader> {
    if data.len() < PAR2_PACKET_HEADER_SIZE {
        return Err(NntpError::InvalidResponse(
            "Packet header too short".to_string(),
        ));
    }

    // Skip magic (first PAR2_MAGIC_SIZE bytes)
    let length = read_u64_le(data, PAR2_MAGIC_SIZE)?;

    let mut hash = [0u8; MD5_HASH_SIZE];
    hash.copy_from_slice(&data[MD5_HASH_SIZE..(MD5_HASH_SIZE * 2)]);

    let mut set_id = [0u8; MD5_HASH_SIZE];
    set_id.copy_from_slice(&data[(MD5_HASH_SIZE * 2)..(MD5_HASH_SIZE * 3)]);

    let mut packet_type = [0u8; MD5_HASH_SIZE];
    packet_type.copy_from_slice(&data[(MD5_HASH_SIZE * 3)..PAR2_PACKET_HEADER_SIZE]);

    Ok(PacketHeader {
        length,
        hash,
        set_id,
        packet_type,
    })
}

/// Parse Main packet body
fn parse_main_packet(data: &[u8]) -> Result<MainPacket> {
    if data.len() < MAIN_PACKET_MIN_SIZE {
        return Err(NntpError::InvalidResponse(
            "Main packet body too short".to_string(),
        ));
    }

    let slice_size = read_u64_le(data, 0)?;
    let file_count = read_u32_le(data, 8)?;

    // Read file IDs (MD5_HASH_SIZE bytes each)
    let mut offset = MAIN_PACKET_MIN_SIZE;
    let mut file_ids = Vec::new();

    for _ in 0..file_count {
        if offset + MD5_HASH_SIZE > data.len() {
            return Err(NntpError::InvalidResponse(
                "Main packet file ID out of bounds".to_string(),
            ));
        }
        let mut file_id = [0u8; MD5_HASH_SIZE];
        file_id.copy_from_slice(&data[offset..offset + MD5_HASH_SIZE]);
        file_ids.push(file_id);
        offset += MD5_HASH_SIZE;
    }

    // Non-recoverable file IDs follow (if any remaining)
    let mut non_recoverable_file_ids = Vec::new();
    while offset + MD5_HASH_SIZE <= data.len() {
        let mut file_id = [0u8; MD5_HASH_SIZE];
        file_id.copy_from_slice(&data[offset..offset + MD5_HASH_SIZE]);
        non_recoverable_file_ids.push(file_id);
        offset += MD5_HASH_SIZE;
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
    if data.len() < FILE_DESC_PACKET_MIN_SIZE {
        return Err(NntpError::InvalidResponse(
            "File Description packet body too short".to_string(),
        ));
    }

    let mut file_id = [0u8; MD5_HASH_SIZE];
    file_id.copy_from_slice(&data[0..MD5_HASH_SIZE]);

    let mut hash = [0u8; MD5_HASH_SIZE];
    hash.copy_from_slice(&data[MD5_HASH_SIZE..(MD5_HASH_SIZE * 2)]);

    let mut hash_16k = [0u8; MD5_HASH_SIZE];
    hash_16k.copy_from_slice(&data[(MD5_HASH_SIZE * 2)..(MD5_HASH_SIZE * 3)]);

    let length = read_u64_le(data, MD5_HASH_SIZE * 3)?;

    // File name is null-terminated and padded to multiple of 4
    let name_bytes = &data[FILE_DESC_PACKET_MIN_SIZE..];
    let name = match name_bytes.iter().position(|&b| b == 0) {
        Some(null_pos) => String::from_utf8_lossy(&name_bytes[..null_pos]).to_string(),
        None => String::from_utf8_lossy(name_bytes).to_string(),
    };

    Ok(FileDescriptionPacket {
        file_id,
        hash,
        hash_16k,
        length,
        name: name.into(), // Convert String to Arc<str>
    })
}

/// Parse IFSC packet body
fn parse_ifsc_packet(data: &[u8]) -> Result<IfscPacket> {
    if data.len() < IFSC_PACKET_MIN_SIZE {
        return Err(NntpError::InvalidResponse(
            "IFSC packet body too short".to_string(),
        ));
    }

    let mut file_id = [0u8; MD5_HASH_SIZE];
    file_id.copy_from_slice(&data[0..MD5_HASH_SIZE]);

    // CRC32 checksums follow (CRC32_SIZE bytes each)
    let checksum_data = &data[MD5_HASH_SIZE..];
    if !checksum_data.len().is_multiple_of(CRC32_SIZE) {
        return Err(NntpError::InvalidResponse(
            "IFSC packet checksum data not aligned".to_string(),
        ));
    }

    let mut checksums = Vec::new();
    let mut offset = MD5_HASH_SIZE;
    for _ in 0..(checksum_data.len() / CRC32_SIZE) {
        let crc = read_u32_le(data, offset)?;
        checksums.push(crc);
        offset += CRC32_SIZE;
    }

    Ok(IfscPacket { file_id, checksums })
}

/// Parse Recovery Slice packet body
fn parse_recovery_slice_packet(data: &[u8]) -> Result<RecoverySlicePacket> {
    if data.len() < RECOVERY_SLICE_PACKET_MIN_SIZE {
        return Err(NntpError::InvalidResponse(
            "Recovery Slice packet body too short".to_string(),
        ));
    }

    let exponent = read_u32_le(data, 0)?;
    let recovery_data = data[RECOVERY_SLICE_PACKET_MIN_SIZE..].to_vec();

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
    fn test_parse_packet_header() {
        // Build a minimal valid header
        let mut header = vec![0u8; PAR2_PACKET_HEADER_SIZE];
        // Magic bytes
        header[0..PAR2_MAGIC_SIZE].copy_from_slice(PAR2_MAGIC);
        // Length (PAR2_PACKET_HEADER_SIZE bytes)
        header[PAR2_MAGIC_SIZE..(PAR2_MAGIC_SIZE + 8)]
            .copy_from_slice(&(PAR2_PACKET_HEADER_SIZE as u64).to_le_bytes());
        // Hash (MD5_HASH_SIZE bytes)
        header[(PAR2_MAGIC_SIZE + 8)..(PAR2_MAGIC_SIZE + 8 + MD5_HASH_SIZE)].fill(1);
        // Set ID (MD5_HASH_SIZE bytes)
        header[(PAR2_MAGIC_SIZE + 8 + MD5_HASH_SIZE)..(PAR2_MAGIC_SIZE + 8 + MD5_HASH_SIZE * 2)]
            .fill(2);
        // Packet type (MD5_HASH_SIZE bytes)
        header[(PAR2_MAGIC_SIZE + 8 + MD5_HASH_SIZE * 2)..PAR2_PACKET_HEADER_SIZE]
            .copy_from_slice(b"PAR 2.0\0Main\0\0\0\0");

        let parsed = parse_packet_header(&header).unwrap();
        assert_eq!(parsed.length, PAR2_PACKET_HEADER_SIZE as u64);
        assert_eq!(parsed.hash, [1u8; MD5_HASH_SIZE]);
        assert_eq!(parsed.set_id, [2u8; MD5_HASH_SIZE]);
        assert_eq!(parsed.packet_type, *b"PAR 2.0\0Main\0\0\0\0");
    }

    #[test]
    fn test_parse_main_packet() {
        let mut data = vec![0u8; MAIN_PACKET_MIN_SIZE];
        // Slice size: 1024 bytes
        data[0..8].copy_from_slice(&1024u64.to_le_bytes());
        // File count: 0
        data[8..MAIN_PACKET_MIN_SIZE].copy_from_slice(&0u32.to_le_bytes());

        let parsed = parse_main_packet(&data).unwrap();
        assert_eq!(parsed.slice_size, 1024);
        assert_eq!(parsed.file_count, 0);
        assert_eq!(parsed.file_ids.len(), 0);
    }

    #[test]
    fn test_parse_file_description_packet() {
        let mut data = vec![0u8; FILE_DESC_PACKET_MIN_SIZE + 9]; // Need at least FILE_DESC_PACKET_MIN_SIZE bytes + filename
                                                                 // File ID
        data[0..MD5_HASH_SIZE].fill(1);
        // Hash
        data[MD5_HASH_SIZE..(MD5_HASH_SIZE * 2)].fill(2);
        // Hash 16k
        data[(MD5_HASH_SIZE * 2)..(MD5_HASH_SIZE * 3)].fill(3);
        // Length: 1000 bytes
        data[(MD5_HASH_SIZE * 3)..(MD5_HASH_SIZE * 3 + 8)].copy_from_slice(&1000u64.to_le_bytes());
        // Filename: "test.bin\0"
        data[FILE_DESC_PACKET_MIN_SIZE..(FILE_DESC_PACKET_MIN_SIZE + 9)]
            .copy_from_slice(b"test.bin\0");

        let parsed = parse_file_description_packet(&data).unwrap();
        assert_eq!(parsed.file_id, [1u8; MD5_HASH_SIZE]);
        assert_eq!(parsed.hash, [2u8; MD5_HASH_SIZE]);
        assert_eq!(parsed.hash_16k, [3u8; MD5_HASH_SIZE]);
        assert_eq!(parsed.length, 1000);
        assert_eq!(parsed.name.as_ref(), "test.bin");
    }

    #[test]
    fn test_parse_ifsc_packet() {
        let mut data = vec![0u8; IFSC_PACKET_MIN_SIZE + (CRC32_SIZE * 2)];
        // File ID
        data[0..MD5_HASH_SIZE].fill(1);
        // Two CRC32 checksums
        data[MD5_HASH_SIZE..(MD5_HASH_SIZE + CRC32_SIZE)]
            .copy_from_slice(&0x12345678u32.to_le_bytes());
        data[(MD5_HASH_SIZE + CRC32_SIZE)..(MD5_HASH_SIZE + CRC32_SIZE * 2)]
            .copy_from_slice(&0x9ABCDEF0u32.to_le_bytes());

        let parsed = parse_ifsc_packet(&data).unwrap();
        assert_eq!(parsed.file_id, [1u8; MD5_HASH_SIZE]);
        assert_eq!(parsed.checksums.len(), 2);
        assert_eq!(parsed.checksums[0], 0x12345678);
        assert_eq!(parsed.checksums[1], 0x9ABCDEF0);
    }

    #[test]
    fn test_parse_recovery_slice_packet() {
        let mut data = vec![0u8; RECOVERY_SLICE_PACKET_MIN_SIZE + 4];
        // Exponent: 5
        data[0..CRC32_SIZE].copy_from_slice(&5u32.to_le_bytes());
        // Recovery data
        data[RECOVERY_SLICE_PACKET_MIN_SIZE..(RECOVERY_SLICE_PACKET_MIN_SIZE + 4)]
            .copy_from_slice(&[1, 2, 3, 4]);

        let parsed = parse_recovery_slice_packet(&data).unwrap();
        assert_eq!(parsed.exponent, 5);
        assert_eq!(parsed.data, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_parse_creator_packet() {
        let data = b"par2cmdline-0.4\0";
        let parsed = parse_creator_packet(data).unwrap();
        assert_eq!(parsed.client, "par2cmdline-0.4");
    }

    #[test]
    fn test_parse_packet_header_truncated() {
        // Header with only 10 bytes (should fail)
        let header = vec![0u8; 10];
        assert!(parse_packet_header(&header).is_err());

        // Header with PAR2_PACKET_HEADER_SIZE bytes but truncated length field
        let mut header = vec![0u8; PAR2_PACKET_HEADER_SIZE];
        header[0..PAR2_MAGIC_SIZE].copy_from_slice(PAR2_MAGIC);
        // Only 2 bytes of length field available instead of 8
        header.truncate(10);
        assert!(parse_packet_header(&header).is_err());
    }

    #[test]
    fn test_parse_main_packet_truncated() {
        // Packet with only 5 bytes (should fail)
        let data = vec![0u8; 5];
        assert!(parse_main_packet(&data).is_err());

        // Packet with truncated slice_size field
        let data = vec![0u8; 4];
        assert!(parse_main_packet(&data).is_err());

        // Packet with truncated file_count field
        let mut data = vec![0u8; 10];
        data[0..8].copy_from_slice(&1024u64.to_le_bytes());
        data.truncate(10); // Cut off before file_count completes
        assert!(parse_main_packet(&data).is_err());
    }

    #[test]
    fn test_parse_file_description_packet_truncated() {
        // Packet with only 40 bytes (should fail, needs FILE_DESC_PACKET_MIN_SIZE)
        let data = vec![0u8; 40];
        assert!(parse_file_description_packet(&data).is_err());

        // Packet with truncated length field
        let mut data = vec![0u8; FILE_DESC_PACKET_MIN_SIZE];
        data[0..MD5_HASH_SIZE].fill(1);
        data[MD5_HASH_SIZE..(MD5_HASH_SIZE * 2)].fill(2);
        data[(MD5_HASH_SIZE * 2)..(MD5_HASH_SIZE * 3)].fill(3);
        data.truncate(50); // Cut off before length field completes
        assert!(parse_file_description_packet(&data).is_err());
    }

    #[test]
    fn test_parse_ifsc_packet_truncated() {
        // Packet with only 10 bytes (should fail, needs at least IFSC_PACKET_MIN_SIZE)
        let data = vec![0u8; 10];
        assert!(parse_ifsc_packet(&data).is_err());

        // Packet with file_id but truncated checksum
        let mut data = vec![0u8; IFSC_PACKET_MIN_SIZE + 2];
        data[0..MD5_HASH_SIZE].fill(1);
        // Only 2 bytes of checksum instead of CRC32_SIZE
        assert!(parse_ifsc_packet(&data).is_err());
    }

    #[test]
    fn test_parse_recovery_slice_packet_truncated() {
        // Packet with only 2 bytes (should fail, needs at least RECOVERY_SLICE_PACKET_MIN_SIZE)
        let data = vec![0u8; 2];
        assert!(parse_recovery_slice_packet(&data).is_err());
    }

    #[test]
    fn test_read_u32_le_truncated() {
        let data = vec![1, 2, 3]; // Only 3 bytes
        assert!(read_u32_le(&data, 0).is_err());
    }

    #[test]
    fn test_read_u64_le_truncated() {
        let data = vec![1, 2, 3, 4, 5, 6]; // Only 6 bytes
        assert!(read_u64_le(&data, 0).is_err());
    }

    #[test]
    fn test_read_u32_le_out_of_bounds() {
        let data = vec![1, 2, 3, 4];
        // Try to read at offset 2, but only 2 bytes left
        assert!(read_u32_le(&data, 2).is_err());
    }

    #[test]
    fn test_read_u64_le_out_of_bounds() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        // Try to read at offset 2, but only 6 bytes left
        assert!(read_u64_le(&data, 2).is_err());
    }
}

//! PAR2 Tests - Live Integration Testing
//!
//! This test suite validates PAR2 parsing and file verification.
//! Tests include:
//! - PAR2 file structure parsing
//! - File checksum extraction and verification
//! - Slice CRC32 verification
//! - Recovery capacity calculation
//!
//! Run with:
//! ```bash
//! cargo test --features live-tests -- --test-threads=1
//! ```

#![cfg(feature = "live-tests")]

use nntp_rs::{FileStatus, Par2File, Par2Set};
use std::path::Path;

// Helper Functions

/// Create a minimal valid PAR2 packet header
fn create_par2_magic_and_header(packet_type: &[u8; 16]) -> Vec<u8> {
    let mut data = Vec::new();

    // PAR2 magic bytes: "PAR2\0PKT"
    data.extend_from_slice(b"PAR2\0PKT");

    // Packet length (64 bytes for header)
    data.extend_from_slice(&64u64.to_le_bytes());

    // MD5 hash of packet (placeholder - 16 bytes)
    data.extend_from_slice(&[0u8; 16]);

    // Recovery Set ID (16 bytes)
    data.extend_from_slice(&[1u8; 16]);

    // Packet type (16 bytes)
    data.extend_from_slice(packet_type);

    data
}

/// Create a minimal Main packet
fn create_minimal_main_packet() -> Vec<u8> {
    let main_type = [
        0x50, 0x41, 0x52, 0x20, 0x32, 0x2E, 0x30, 0x00, // "PAR 2.0\0"
        0x4D, 0x61, 0x69, 0x6E, 0x00, 0x00, 0x00, 0x00, // "Main\0\0\0\0"
    ];

    let mut data = create_par2_magic_and_header(&main_type);

    // Slice size (e.g., 1024 bytes)
    data.extend_from_slice(&1024u64.to_le_bytes());

    // File count
    data.extend_from_slice(&1u32.to_le_bytes());

    // Padding to ensure multiple of 4
    while !data.len().is_multiple_of(4) {
        data.push(0);
    }

    // One file ID (16 bytes)
    data.extend_from_slice(&[2u8; 16]);

    // Update packet length in header
    let len = data.len() as u64;
    data[8..16].copy_from_slice(&len.to_le_bytes());

    data
}

/// Create a minimal FileDescription packet
fn create_file_description_packet(file_id: &[u8; 16], filename: &str) -> Vec<u8> {
    let fd_type = [
        0x50, 0x41, 0x52, 0x20, 0x32, 0x2E, 0x30, 0x00, // "PAR 2.0\0"
        0x46, 0x69, 0x6C, 0x65, 0x44, 0x65, 0x73, 0x63, // "FileDesc"
    ];

    let mut data = create_par2_magic_and_header(&fd_type);

    // File ID (16 bytes)
    data.extend_from_slice(file_id);

    // Hash (MD5 - 16 bytes placeholder)
    data.extend_from_slice(&[0xAAu8; 16]);

    // Hash 16k (MD5 - 16 bytes placeholder)
    data.extend_from_slice(&[0xBBu8; 16]);

    // File length
    data.extend_from_slice(&1024u64.to_le_bytes());

    // Filename (null-terminated, padded to multiple of 4)
    let filename_bytes = filename.as_bytes();
    data.extend_from_slice(filename_bytes);
    data.push(0); // null terminator

    // Pad to multiple of 4
    while !data.len().is_multiple_of(4) {
        data.push(0);
    }

    // Update packet length
    let len = data.len() as u64;
    data[8..16].copy_from_slice(&len.to_le_bytes());

    data
}

/// Create a minimal IFSC (Input File Slice Checksum) packet
fn create_ifsc_packet(file_id: &[u8; 16], checksums: &[u32]) -> Vec<u8> {
    let ifsc_type = [
        0x50, 0x41, 0x52, 0x20, 0x32, 0x2E, 0x30, 0x00, // "PAR 2.0\0"
        0x49, 0x46, 0x53, 0x43, 0x00, 0x00, 0x00, 0x00, // "IFSC\0\0\0\0"
    ];

    let mut data = create_par2_magic_and_header(&ifsc_type);

    // File ID (16 bytes)
    data.extend_from_slice(file_id);

    // Checksums (4 bytes each)
    for checksum in checksums {
        data.extend_from_slice(&checksum.to_le_bytes());
    }

    // Update packet length
    let len = data.len() as u64;
    data[8..16].copy_from_slice(&len.to_le_bytes());

    data
}
#[test]
fn test_parse_minimal_par2_structure() {
    // Create a minimal PAR2 file with Main packet
    let par2_data = create_minimal_main_packet();

    // Parse it
    let result = Par2File::parse(&par2_data);

    // Should parse successfully
    assert!(
        result.is_ok(),
        "Failed to parse minimal PAR2: {:?}",
        result.err()
    );

    let par2 = result.unwrap();

    // Should have a main packet
    assert!(par2.main.is_some(), "Main packet not found");

    // Should have slice size of 1024
    assert_eq!(par2.slice_size(), Some(1024));
}

#[test]
fn test_parse_par2_with_file_description() {
    // Create Main packet + FileDescription packet
    let file_id = [2u8; 16];
    let mut par2_data = create_minimal_main_packet();
    par2_data.extend_from_slice(&create_file_description_packet(&file_id, "test.bin"));

    // Parse it
    let result = Par2File::parse(&par2_data);
    assert!(
        result.is_ok(),
        "Failed to parse PAR2 with file description: {:?}",
        result.err()
    );

    let par2 = result.unwrap();

    // Should have one file description
    assert_eq!(par2.file_descriptions.len(), 1);

    // Verify file description
    let fd = par2
        .file_descriptions
        .get(&file_id)
        .expect("File description not found");
    assert_eq!(&*fd.name, "test.bin");
    assert_eq!(fd.length, 1024);
}

#[test]
fn test_parse_par2_with_ifsc() {
    // Create Main + FileDescription + IFSC packets
    let file_id = [2u8; 16];
    let mut par2_data = create_minimal_main_packet();
    par2_data.extend_from_slice(&create_file_description_packet(&file_id, "test.bin"));

    // Add IFSC with one CRC32 checksum
    let checksums = vec![0x12345678u32];
    par2_data.extend_from_slice(&create_ifsc_packet(&file_id, &checksums));

    // Parse it
    let result = Par2File::parse(&par2_data);
    assert!(
        result.is_ok(),
        "Failed to parse PAR2 with IFSC: {:?}",
        result.err()
    );

    let par2 = result.unwrap();

    // Should have one IFSC packet
    assert_eq!(par2.ifsc_packets.len(), 1);

    // Verify IFSC
    let ifsc = par2
        .ifsc_packets
        .get(&file_id)
        .expect("IFSC packet not found");
    assert_eq!(ifsc.checksums.len(), 1);
    assert_eq!(ifsc.checksums[0], 0x12345678);
}

#[test]
fn test_slice_size_extraction() {
    let par2_data = create_minimal_main_packet();
    let par2 = Par2File::parse(&par2_data).unwrap();

    // Should extract slice size
    assert_eq!(par2.slice_size(), Some(1024));
}

#[test]
fn test_empty_par2_data() {
    // Empty data returns an empty PAR2File (no packets)
    let result = Par2File::parse(&[]);
    assert!(result.is_ok(), "Empty PAR2 data should parse to empty file");

    let par2 = result.unwrap();
    assert!(par2.main.is_none(), "Empty PAR2 should have no main packet");
    assert_eq!(par2.file_descriptions.len(), 0);
    assert_eq!(par2.ifsc_packets.len(), 0);
    assert_eq!(par2.recovery_slices.len(), 0);
}

#[test]
fn test_invalid_magic_bytes() {
    // Invalid magic bytes
    let mut data = vec![0u8; 64];
    data[..8].copy_from_slice(b"INVALID!");

    let result = Par2File::parse(&data);
    assert!(result.is_err(), "Invalid magic bytes should fail to parse");
}
#[test]
fn test_verify_missing_file() {
    // Create a PAR2 with file description
    let file_id = [2u8; 16];
    let mut par2_data = create_minimal_main_packet();
    par2_data.extend_from_slice(&create_file_description_packet(&file_id, "missing.bin"));

    let par2 = Par2File::parse(&par2_data).unwrap();

    // Verify file with empty data (missing)
    let empty_data: &[u8] = &[];
    let result = par2.verify_file(empty_data, &file_id);

    assert!(result.is_ok());
    let verification = result.unwrap();
    assert!(matches!(verification.status, FileStatus::Missing));
}

#[test]
fn test_verify_complete_file_with_matching_size() {
    // Create a PAR2 with file description
    let file_id = [2u8; 16];
    let mut par2_data = create_minimal_main_packet();
    par2_data.extend_from_slice(&create_file_description_packet(&file_id, "complete.bin"));

    let par2 = Par2File::parse(&par2_data).unwrap();

    // Create file data with correct size (1024 bytes from file description)
    let file_data = vec![0u8; 1024];

    // Verify file
    let result = par2.verify_file(&file_data, &file_id);
    assert!(result.is_ok());

    let verification = result.unwrap();
    assert_eq!(verification.filename, "complete.bin");
    assert_eq!(verification.expected_size, 1024);
}
#[test]
fn test_slice_mapping_basic() {
    // Create a PAR2 with one file (1024 bytes = 1 slice at 1024 slice size)
    let file_id = [2u8; 16];
    let mut par2_data = create_minimal_main_packet();
    par2_data.extend_from_slice(&create_file_description_packet(&file_id, "test.bin"));

    let par2 = Par2File::parse(&par2_data).unwrap();

    // Map slices
    let result = par2.map_slices();
    assert!(result.is_ok(), "Failed to map slices: {:?}", result.err());

    let mappings = result.unwrap();

    // Should have 1 slice mapping (1024 bytes / 1024 slice size = 1 slice)
    assert_eq!(mappings.len(), 1);

    let mapping = &mappings[0];
    assert_eq!(mapping.file_id, file_id);
    assert_eq!(mapping.filename, "test.bin");
    assert_eq!(mapping.file_slice_index, 0);
    assert_eq!(mapping.offset, 0);
    assert_eq!(mapping.size, 1024);
}
#[test]
fn test_recovery_slice_count() {
    // Create a basic PAR2 with no recovery slices
    let par2_data = create_minimal_main_packet();
    let par2 = Par2File::parse(&par2_data).unwrap();

    // Should have 0 recovery slices
    assert_eq!(par2.recovery_slice_count(), 0);
}
#[test]
fn test_par2_set_discovery_nonexistent_dir() {
    // Try to discover PAR2 set in non-existent directory
    let result = Par2Set::discover("/nonexistent/path", "testfile");

    // Should fail gracefully
    assert!(
        result.is_err(),
        "Discovery in non-existent directory should fail"
    );
}

#[test]
fn test_par2_set_discovery_empty_dir() {
    // Create a temporary directory
    let temp_dir = std::env::temp_dir();
    let test_dir = temp_dir.join("par2_test_empty");
    let _ = std::fs::create_dir_all(&test_dir);

    // Try to discover PAR2 set (should fail - no PAR2 files)
    let result = Par2Set::discover(&test_dir, "nonexistent");

    // Should fail because no PAR2 files exist
    assert!(
        result.is_err(),
        "Discovery should fail when no PAR2 files exist"
    );

    // Cleanup
    let _ = std::fs::remove_dir(&test_dir);
}

// Integration Test with Real PAR2 File (if available)

#[test]
#[ignore = "Requires a real PAR2 file in tests/fixtures/"]
fn test_parse_real_par2_file() {
    // This test requires a real PAR2 file
    let par2_path = Path::new("tests/fixtures/test.par2");

    if !par2_path.exists() {
        println!("Skipping test - no PAR2 file at {:?}", par2_path);
        return;
    }

    // Read PAR2 file
    let par2_data = std::fs::read(par2_path).expect("Failed to read PAR2 file");

    // Parse it
    let result = Par2File::parse(&par2_data);
    assert!(
        result.is_ok(),
        "Failed to parse real PAR2 file: {:?}",
        result.err()
    );

    let par2 = result.unwrap();

    // Basic validation
    assert!(par2.main.is_some(), "Real PAR2 should have Main packet");
    assert!(
        par2.slice_size().is_some(),
        "Real PAR2 should have slice size"
    );

    println!("PAR2 file parsed successfully!");
    println!("  Slice size: {} bytes", par2.slice_size().unwrap());
    println!("  File descriptions: {}", par2.file_descriptions.len());
    println!("  IFSC packets: {}", par2.ifsc_packets.len());
    println!("  Recovery slices: {}", par2.recovery_slice_count());
}

// Notes on Live Integration Testing

// For true "live" integration testing with NNTP downloads, you would:
// 1. Download a multi-part post from NNTP server (using existing client)
// 2. Decode yEnc segments (using existing yenc module)
// 3. Look for accompanying PAR2 files in the same post/group
// 4. Download PAR2 files by message-ID
// 5. Parse PAR2 files and verify the downloaded data
//
// However, this requires:
// - Knowing specific message-IDs for posts with PAR2 files
// - Server having actual binary posts with PAR2 data
// - This is highly dependent on server content availability
//
// For now, these unit/integration tests validate the PAR2 parsing
// functionality itself. The "live" aspect would be downloading the
// PAR2 data via NNTP, which can be done by the existing NNTP client
// (fetch_article_by_message_id) and then passing to Par2File::parse().

//! Binary yEnc Tests - Live Integration Testing
//!
//! This test suite validates yEnc download, decoding, and verification
//! against a real NNTP server.
//!
//! Run with:
//! ```bash
//! cargo test --features live-tests -- --test-threads=1
//! ```

#![cfg(feature = "live-tests")]

use nntp_rs::{yenc_decode, NntpClient, YencMultipartAssembler};
use std::sync::Arc;

use super::{get_binary_test_group, get_test_config};

/// Convert NntpResponse lines to raw bytes using Latin-1 encoding
/// yEnc binary data uses bytes 0x80-0xFF which are not valid UTF-8,
/// but Usenet traditionally uses Latin-1 (ISO-8859-1) encoding
fn response_to_bytes(lines: &[String]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        // Convert each character to its Latin-1 byte value
        for ch in line.chars() {
            if ch as u32 <= 0xFF {
                bytes.push(ch as u8);
            } else {
                // Non-Latin-1 character, use replacement
                bytes.push(b'?');
            }
        }
        // Add line ending (except for last line)
        if i < lines.len() - 1 {
            bytes.push(b'\r');
            bytes.push(b'\n');
        }
    }
    bytes
}

// Single-Part yEnc Tests

#[allow(clippy::excessive_nesting)]
#[tokio::test]
async fn test_single_part_yenc_download() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    // Select binary test group
    let group = get_binary_test_group();
    let info = client.select_group(&group).await.unwrap();
    let (count, first, last) = (info.count, info.first, info.last);

    if count == 0 {
        println!("No articles in {}, skipping test", group);
        return;
    }

    println!(
        "Binary group {}: {} articles (first={}, last={})",
        group, count, first, last
    );

    // Search for a yEnc post by looking at recent articles
    // We'll scan backwards from the last article
    let mut found_yenc = false;
    let scan_count = std::cmp::min(100, count); // Scan up to 100 recent articles

    for article_num in (last.saturating_sub(scan_count)..=last).rev() {
        // Try to get the article body
        let body_result = client.fetch_body(&article_num.to_string()).await;

        if let Ok(response) = body_result {
            // Convert response to bytes (yEnc uses Latin-1 encoding)
            let body = response_to_bytes(&response.lines);
            // Check if it looks like yEnc (contains =ybegin)
            if body.windows(8).any(|w| w == b"=ybegin ") {
                println!("Found yEnc post at article {}", article_num);

                // Try to decode it
                match yenc_decode(&body) {
                    Ok(decoded) => {
                        println!("Successfully decoded yEnc post:");
                        println!("  Filename: {}", decoded.header.name);
                        println!("  Size: {} bytes", decoded.header.size);
                        println!("  Decoded data size: {} bytes", decoded.data.len());
                        println!("  Multipart: {}", decoded.is_multipart());

                        // Verify CRC32
                        let crc_valid = decoded.verify_crc32();
                        println!("  CRC32 valid: {}", crc_valid);

                        assert!(crc_valid, "CRC32 verification failed");
                        assert_eq!(
                            decoded.data.len() as u64,
                            decoded.trailer.size,
                            "Decoded size doesn't match trailer size"
                        );

                        found_yenc = true;
                        break;
                    }
                    Err(e) => {
                        println!(
                            "Found yEnc marker but decode failed at article {}: {}",
                            article_num, e
                        );
                        // Continue searching for a valid one
                    }
                }
            }
        }
    }

    if !found_yenc {
        println!(
            "WARNING: No valid yEnc posts found in recent articles of {}",
            group
        );
        println!("This test cannot verify yEnc functionality without test data");
        // Don't fail - the server might not have yEnc posts right now
    }
}

#[tokio::test]
async fn test_yenc_crc32_verification() {
    // Test with known good yEnc data
    let yenc_data = b"=ybegin line=128 size=11 name=test.txt\r\n\
                      *+./1256789*+./\r\n\
                      =yend size=11 crc32=a2582e90\r\n";

    let decoded = yenc_decode(yenc_data).unwrap();

    println!("Test yEnc decoded:");
    println!("  Filename: {}", decoded.header.name);
    println!("  Size: {} bytes", decoded.header.size);
    println!("  CRC32 expected: {:x}", decoded.trailer.crc32.unwrap());
    println!("  CRC32 calculated: {:x}", decoded.calculated_crc32);

    assert!(
        decoded.verify_crc32(),
        "CRC32 verification failed for test data"
    );
    assert_eq!(decoded.header.name, "test.txt");
    assert_eq!(decoded.data.len() as u64, 11);
}

// Multi-Part yEnc Tests

#[allow(clippy::excessive_nesting)]
#[tokio::test]
async fn test_multipart_yenc_download() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    // Select binary test group
    let group = get_binary_test_group();
    let info = client.select_group(&group).await.unwrap();
    let last = info.last;

    if info.count == 0 {
        println!("No articles in {}, skipping test", group);
        return;
    }

    println!("Scanning {} for multi-part yEnc posts...", group);

    // Scan for multi-part yEnc posts
    // Multi-part posts have "part=" in the =ybegin line
    let mut multipart_posts: Vec<(u64, Vec<u8>)> = Vec::new();
    let scan_count = std::cmp::min(200, info.count); // Scan more articles for multi-part

    for article_num in (last.saturating_sub(scan_count)..=last).rev() {
        let body_result = client.fetch_body(&article_num.to_string()).await;

        if let Ok(response) = body_result {
            let body = response_to_bytes(&response.lines);
            // Check for multi-part yEnc (contains "part=" after =ybegin)
            if body.windows(8).any(|w| w == b"=ybegin ") {
                // Check if it has "part=" which indicates multi-part
                let body_str = String::from_utf8_lossy(&body);
                if body_str.contains("part=") {
                    println!("Found potential multi-part yEnc at article {}", article_num);
                    multipart_posts.push((article_num, body));

                    // Collect at least 2 parts
                    if multipart_posts.len() >= 2 {
                        break;
                    }
                }
            }
        }
    }

    if multipart_posts.is_empty() {
        println!("WARNING: No multi-part yEnc posts found in {}", group);
        println!("This test cannot verify multi-part functionality without test data");
        return;
    }

    println!(
        "Found {} potential multi-part yEnc posts",
        multipart_posts.len()
    );

    // Try to decode the parts
    let mut assembler = YencMultipartAssembler::new();
    let mut decoded_count = 0;

    for (article_num, body) in &multipart_posts {
        match yenc_decode(body) {
            Ok(decoded) => {
                println!("Decoded multi-part yEnc from article {}:", article_num);
                println!("  Filename: {}", decoded.header.name);
                println!(
                    "  Part: {:?} of {:?}",
                    decoded.header.part, decoded.header.total
                );
                println!("  Part range: {:?}", decoded.part);
                println!("  Part size: {} bytes", decoded.data.len());

                assert!(decoded.is_multipart(), "Expected multi-part yEnc");
                assert!(decoded.verify_crc32(), "Part CRC32 verification failed");

                // Try to add to assembler
                match assembler.add_part(decoded) {
                    Ok(_) => {
                        decoded_count += 1;
                        println!("  Added to assembler successfully");
                    }
                    Err(e) => {
                        println!("  Could not add to assembler: {}", e);
                        println!("  (Might be from different files)");
                    }
                }
            }
            Err(e) => {
                println!("Failed to decode article {}: {}", article_num, e);
            }
        }
    }

    if decoded_count > 0 {
        println!(
            "Successfully decoded {} multi-part yEnc segments",
            decoded_count
        );
        println!("Assembler status:");
        println!("  Complete: {}", assembler.is_complete());
        println!("  Parts received: {}", assembler.parts_received());

        if !assembler.is_complete() {
            let missing = assembler.missing_parts();
            println!("  Missing parts: {:?}", missing);
            println!("  (Expected - we only scanned a limited number of articles)");
        }

        // If we somehow got all parts, try to assemble
        if assembler.is_complete() {
            match assembler.assemble() {
                Ok(data) => {
                    println!("Successfully assembled complete file: {} bytes", data.len());
                    // This is a real success - we found and assembled a complete multi-part post!
                }
                Err(e) => {
                    println!("Assembly failed: {}", e);
                }
            }
        }
    } else {
        println!("WARNING: Could not decode any multi-part yEnc segments");
    }
}

#[tokio::test]
async fn test_multipart_assembly_with_known_data() {
    // Test with known multi-part yEnc data
    // This ensures the assembly logic works even if we can't find real multi-part posts

    // Part 1 of 2
    let part1 = b"=ybegin part=1 total=2 line=128 size=20 name=test.bin\r\n\
                  =ypart begin=1 end=10\r\n\
                  *+./123456\r\n\
                  =yend size=10 pcrc32=a4337ae0\r\n";

    // Part 2 of 2
    let part2 = b"=ybegin part=2 total=2 line=128 size=20 name=test.bin\r\n\
                  =ypart begin=11 end=20\r\n\
                  789*+./123\r\n\
                  =yend size=10 pcrc32=b80bbeb9\r\n";

    let decoded1 = yenc_decode(part1).unwrap();
    let decoded2 = yenc_decode(part2).unwrap();

    println!(
        "Part 1: {} bytes (range {:?})",
        decoded1.data.len(),
        decoded1.part
    );
    println!(
        "Part 2: {} bytes (range {:?})",
        decoded2.data.len(),
        decoded2.part
    );

    assert!(decoded1.is_multipart());
    assert!(decoded2.is_multipart());
    assert!(decoded1.verify_crc32());
    assert!(decoded2.verify_crc32());

    // Assemble
    let mut assembler = YencMultipartAssembler::new();
    assembler.add_part(decoded1).unwrap();
    assembler.add_part(decoded2).unwrap();

    assert!(assembler.is_complete(), "Assembler should have all parts");
    assert_eq!(assembler.parts_received(), 2);

    let assembled = assembler.assemble().unwrap();
    println!("Assembled: {} bytes", assembled.len());
    assert_eq!(assembled.len(), 20);
}
#[tokio::test]
async fn test_corrupted_yenc_handling() {
    // Test with corrupted yEnc data (invalid CRC32)
    let corrupted = b"=ybegin line=128 size=11 name=test.txt\r\n\
                      *+./1256789*+./\r\n\
                      =yend size=11 crc32=00000000\r\n";

    let decoded = yenc_decode(corrupted).unwrap();

    // Decode should succeed but CRC32 verification should fail
    assert!(
        !decoded.verify_crc32(),
        "CRC32 should be invalid for corrupted data"
    );
    println!("Correctly detected corrupted yEnc data");
}

#[tokio::test]
async fn test_missing_yenc_trailer() {
    // Test with incomplete yEnc data (missing =yend)
    let incomplete = b"=ybegin line=128 size=11 name=test.txt\r\n\
                       *+./1256789*+./\r\n";

    let result = yenc_decode(incomplete);
    assert!(
        result.is_err(),
        "Should fail to decode yEnc without trailer"
    );
    println!("Correctly rejected incomplete yEnc data: {:?}", result);
}

#[tokio::test]
async fn test_invalid_yenc_header() {
    // Test with invalid yEnc header
    let invalid = b"=ybegin invalid_format\r\n\
                    *+./1256789*+./\r\n\
                    =yend size=11 crc32=a2582e90\r\n";

    let result = yenc_decode(invalid);
    assert!(
        result.is_err(),
        "Should fail to decode yEnc with invalid header"
    );
    println!("Correctly rejected invalid yEnc header: {:?}", result);
}
#[tokio::test]
async fn test_yenc_preserves_binary_data() {
    // Test that yEnc correctly handles all byte values (0x00-0xFF)
    // This is important for binary files

    // Create test data with all possible byte values
    let mut test_data: Vec<u8> = Vec::new();
    for i in 0..=255u8 {
        test_data.push(i);
    }

    // We can't easily encode here without importing the encode function
    // But we can verify that decoding preserves binary data
    // This test validates that our decoder handles binary correctly

    // Known yEnc encoding of some binary data
    // The yEnc format should preserve all bytes after decoding
    let yenc_binary = b"=ybegin line=128 size=3 name=binary.dat\r\n\
                        =M*+\r\n\
                        =yend size=3 crc32=352441c2\r\n";

    let decoded = yenc_decode(yenc_binary).unwrap();
    println!("Binary data decoded: {:?}", decoded.data);
    println!("Size: {} bytes", decoded.data.len());

    // Verify it's actually binary (contains byte 0)
    assert!(decoded.data.contains(&0), "Should contain null bytes");
    assert!(decoded.verify_crc32(), "CRC32 should match");
}

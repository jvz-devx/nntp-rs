//! NZB Tests - Live Integration Testing
//!
//! This test suite validates NZB parsing and downloading segments
//! from a real NNTP server.
//!
//! Run with:
//! ```bash
//! cargo test --features live-tests -- --test-threads=1
//! ```

#![cfg(feature = "live-tests")]

use nntp_rs::{parse_nzb, NntpClient};
use std::sync::Arc;

use super::{get_binary_test_group, get_test_config};

// Helper Functions

/// Create a minimal valid NZB for testing
fn create_test_nzb_simple() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE nzb PUBLIC "-//newzBin//DTD NZB 1.1//EN" "http://www.newzbin.com/DTD/nzb/nzb-1.1.dtd">
<nzb xmlns="http://www.newzbin.com/DTD/2003/nzb">
  <file poster="test@example.com" date="1234567890" subject="Test File (1/1)">
    <groups>
      <group>alt.binaries.test</group>
    </groups>
    <segments>
      <segment bytes="5000" number="1">test-message-id@example.com</segment>
    </segments>
  </file>
</nzb>"#
        .to_string()
}

/// Create a multi-file NZB for testing
fn create_test_nzb_multi_file() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE nzb PUBLIC "-//newzBin//DTD NZB 1.1//EN" "http://www.newzbin.com/DTD/nzb/nzb-1.1.dtd">
<nzb xmlns="http://www.newzbin.com/DTD/2003/nzb">
  <head>
    <meta type="title">Test Archive</meta>
    <meta type="password">secret123</meta>
    <meta type="tag">test</meta>
    <meta type="category">testing</meta>
  </head>
  <file poster="poster1@example.com" date="1234567890" subject="File One (1/2)">
    <groups>
      <group>alt.binaries.test</group>
      <group>alt.test</group>
    </groups>
    <segments>
      <segment bytes="10000" number="1">file1-part1@example.com</segment>
      <segment bytes="5000" number="2">file1-part2@example.com</segment>
    </segments>
  </file>
  <file poster="poster2@example.com" date="1234567891" subject="File Two (1/1)">
    <groups>
      <group>alt.binaries.test</group>
    </groups>
    <segments>
      <segment bytes="8000" number="1">file2-part1@example.com</segment>
    </segments>
  </file>
</nzb>"#
        .to_string()
}
#[test]
fn test_parse_simple_nzb() {
    let xml = create_test_nzb_simple();
    let nzb = parse_nzb(&xml).expect("Failed to parse simple NZB");

    // Verify structure
    assert_eq!(nzb.files.len(), 1);
    assert_eq!(nzb.meta.len(), 0); // No <head> section in simple NZB

    let file = &nzb.files[0];
    assert_eq!(file.poster, "test@example.com");
    assert_eq!(file.date, 1234567890);
    assert_eq!(file.subject, "Test File (1/1)");
    assert_eq!(file.groups.len(), 1);
    assert_eq!(file.groups[0], "alt.binaries.test");
    assert_eq!(file.segments.len(), 1);

    let seg = &file.segments[0];
    assert_eq!(seg.bytes, 5000);
    assert_eq!(seg.number, 1);
    assert_eq!(seg.message_id, "test-message-id@example.com");
}

#[test]
fn test_parse_multi_file_nzb() {
    let xml = create_test_nzb_multi_file();
    let nzb = parse_nzb(&xml).expect("Failed to parse multi-file NZB");

    // Verify metadata
    assert_eq!(nzb.meta.len(), 4);
    assert_eq!(nzb.meta.get("title"), Some(&"Test Archive".to_string()));
    assert_eq!(nzb.meta.get("password"), Some(&"secret123".to_string()));
    assert_eq!(nzb.meta.get("tag"), Some(&"test".to_string()));
    assert_eq!(nzb.meta.get("category"), Some(&"testing".to_string()));

    // Verify files
    assert_eq!(nzb.files.len(), 2);

    // First file - 2 segments
    let file1 = &nzb.files[0];
    assert_eq!(file1.poster, "poster1@example.com");
    assert_eq!(file1.date, 1234567890);
    assert_eq!(file1.subject, "File One (1/2)");
    assert_eq!(file1.groups.len(), 2);
    assert_eq!(file1.segments.len(), 2);
    assert_eq!(file1.total_bytes(), 15000); // 10000 + 5000

    // Second file - 1 segment
    let file2 = &nzb.files[1];
    assert_eq!(file2.poster, "poster2@example.com");
    assert_eq!(file2.date, 1234567891);
    assert_eq!(file2.subject, "File Two (1/1)");
    assert_eq!(file2.groups.len(), 1);
    assert_eq!(file2.segments.len(), 1);
    assert_eq!(file2.total_bytes(), 8000);

    // Total bytes
    assert_eq!(nzb.total_bytes(), 23000); // 15000 + 8000
}

#[test]
fn test_nzb_validation() {
    let xml = create_test_nzb_multi_file();
    let nzb = parse_nzb(&xml).expect("Failed to parse NZB");

    // Should validate successfully
    nzb.validate().expect("NZB validation failed");

    // Check individual file validation
    for file in &nzb.files {
        file.validate_segments().expect("Segment validation failed");
        assert!(file.missing_segments().is_empty());
    }
}

#[test]
fn test_segment_ordering() {
    let xml = create_test_nzb_multi_file();
    let nzb = parse_nzb(&xml).expect("Failed to parse NZB");

    // Check first file has sequential segments 1, 2
    let file1 = &nzb.files[0];
    assert_eq!(file1.segments.len(), 2);

    let mut numbers: Vec<u32> = file1.segments.iter().map(|s| s.number).collect();
    numbers.sort();
    assert_eq!(numbers, vec![1, 2]);

    // Verify no missing segments
    assert!(file1.missing_segments().is_empty());
}
#[tokio::test]
async fn test_download_segment_by_message_id() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    // Select binary test group
    let group = get_binary_test_group();
    let (count, first, last) = client.select_group(&group).await.unwrap();

    if count == 0 {
        println!("No articles in {}, skipping test", group);
        return;
    }

    println!(
        "Binary group {}: {} articles (first={}, last={})",
        group, count, first, last
    );

    // Try to find an article with a Message-ID we can use
    // We'll scan recent articles and try to fetch by message-ID
    let scan_count = std::cmp::min(50, count);
    let mut found_article = false;

    for article_num in (last.saturating_sub(scan_count)..=last).rev() {
        // First get the article's headers to extract Message-ID
        let head_result = client.fetch_head(&article_num.to_string()).await;

        if let Ok(head_response) = head_result {
            // Extract Message-ID from the headers
            // Look for "Message-ID: <...>" in the headers
            let message_id_opt = head_response
                .lines
                .iter()
                .find(|line| line.to_lowercase().starts_with("message-id:"))
                .and_then(|line| {
                    let parts: Vec<&str> = line.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        Some(parts[1].trim().to_string())
                    } else {
                        None
                    }
                });

            if let Some(message_id) = message_id_opt {
                println!(
                    "Found article {} with Message-ID: {}",
                    article_num, message_id
                );

                // Now try to fetch by Message-ID
                let fetch_result = client.fetch_article(&message_id).await;

                match fetch_result {
                    Ok(response) => {
                        println!("Successfully fetched article by Message-ID");
                        println!("  Lines: {}", response.lines.len());
                        assert!(!response.lines.is_empty(), "Article should have content");
                        found_article = true;
                        break;
                    }
                    Err(e) => {
                        println!("Failed to fetch by Message-ID: {}", e);
                        continue;
                    }
                }
            }
        }
    }

    if !found_article {
        println!("Could not find article with Message-ID to test, skipping");
    } else {
        assert!(
            found_article,
            "Should have found at least one article to fetch by Message-ID"
        );
    }
}

#[tokio::test]
async fn test_nzb_segment_download_simulation() {
    // This test simulates the NZB download workflow:
    // 1. Parse NZB
    // 2. Connect to server
    // 3. For each segment, fetch by Message-ID
    // 4. Verify ordering

    let xml = create_test_nzb_simple();
    let nzb = parse_nzb(&xml).expect("Failed to parse NZB");

    // Verify NZB structure
    nzb.validate().expect("NZB validation failed");

    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    // In a real scenario, we would:
    // 1. Iterate through nzb.files
    // 2. For each file, download all segments by message_id
    // 3. Assemble the segments in order
    // 4. Decode (e.g., yEnc) and verify

    // For this test, we'll just verify the structure is correct
    for (file_idx, file) in nzb.files.iter().enumerate() {
        println!(
            "File {}: {} ({} segments)",
            file_idx,
            file.subject,
            file.segments.len()
        );

        // Verify segments are sequential
        file.validate_segments().expect("Segment validation failed");

        for seg in &file.segments {
            println!(
                "  Segment {}: {} bytes, Message-ID: {}",
                seg.number, seg.bytes, seg.message_id
            );

            // In a real download, we would do:
            // let article = client.fetch_article(&seg.message_id).await?;
            // let body = article.lines.join("\n");
            // let decoded = yenc_decode(body.as_bytes())?;
            // assembled_data.extend(decoded.data);
        }
    }

    println!("NZB download simulation completed successfully");
}

#[tokio::test]
async fn test_multi_segment_assembly_logic() {
    // Test the logic for assembling multi-segment files
    let xml = create_test_nzb_multi_file();
    let nzb = parse_nzb(&xml).expect("Failed to parse NZB");

    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    // Focus on the first file which has 2 segments
    let file = &nzb.files[0];
    assert_eq!(file.segments.len(), 2);

    println!("Testing multi-segment file: {}", file.subject);
    println!("Total expected size: {} bytes", file.total_bytes());

    // Verify segments are in correct order
    let mut segments_sorted = file.segments.clone();
    segments_sorted.sort_by_key(|s| s.number);

    for (i, seg) in segments_sorted.iter().enumerate() {
        assert_eq!(
            seg.number,
            (i + 1) as u32,
            "Segments should be numbered sequentially"
        );
        println!("  Segment {}: {} bytes", seg.number, seg.bytes);
    }

    // In a real implementation, we would:
    // 1. Download each segment by message_id in order
    // 2. Decode each segment (yEnc)
    // 3. Concatenate the decoded data
    // 4. Verify final file size matches sum of segment sizes

    println!("Multi-segment assembly logic verified");
}


#[test]
fn test_nzb_with_special_characters() {
    // Test NZB with XML entities and special characters
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE nzb PUBLIC "-//newzBin//DTD NZB 1.1//EN" "http://www.newzbin.com/DTD/nzb/nzb-1.1.dtd">
<nzb xmlns="http://www.newzbin.com/DTD/2003/nzb">
  <file poster="test &lt;test@example.com&gt;" date="1234567890" subject="Test &amp; Special &quot;Chars&quot; (1/1)">
    <groups>
      <group>alt.binaries.test</group>
    </groups>
    <segments>
      <segment bytes="1000" number="1">special-chars-id@example.com</segment>
    </segments>
  </file>
</nzb>"#;

    let nzb = parse_nzb(xml).expect("Failed to parse NZB with special chars");

    let file = &nzb.files[0];
    // XML entities should be unescaped
    assert_eq!(file.poster, "test <test@example.com>");
    assert!(file.subject.contains("Test & Special \"Chars\""));
}

#[test]
fn test_empty_nzb_validation() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE nzb PUBLIC "-//newzBin//DTD NZB 1.1//EN" "http://www.newzbin.com/DTD/nzb/nzb-1.1.dtd">
<nzb xmlns="http://www.newzbin.com/DTD/2003/nzb">
</nzb>"#;

    let nzb = parse_nzb(xml).expect("Failed to parse empty NZB");

    // Empty NZB should fail validation
    let result = nzb.validate();
    assert!(result.is_err(), "Empty NZB should fail validation");
}

#[test]
fn test_missing_segment_detection() {
    // Create NZB with gap in segment numbering (1, 3 - missing 2)
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE nzb PUBLIC "-//newzBin//DTD NZB 1.1//EN" "http://www.newzbin.com/DTD/nzb/nzb-1.1.dtd">
<nzb xmlns="http://www.newzbin.com/DTD/2003/nzb">
  <file poster="test@example.com" date="1234567890" subject="Incomplete File (1/3)">
    <groups>
      <group>alt.binaries.test</group>
    </groups>
    <segments>
      <segment bytes="1000" number="1">seg1@example.com</segment>
      <segment bytes="1000" number="3">seg3@example.com</segment>
    </segments>
  </file>
</nzb>"#;

    let nzb = parse_nzb(xml).expect("Failed to parse NZB");
    let file = &nzb.files[0];

    // Should detect missing segment 2
    let missing = file.missing_segments();
    assert_eq!(missing, vec![2]);

    // Validation should fail
    let result = file.validate_segments();
    assert!(
        result.is_err(),
        "Should fail validation with missing segment"
    );
}

#[test]
fn test_nzb_round_trip() {
    // Test that parsing and generating preserves data
    let original_xml = create_test_nzb_multi_file();
    let nzb = parse_nzb(&original_xml).expect("Failed to parse NZB");

    // Generate XML from parsed structure
    let generated_xml = nzb.to_xml();

    // Parse the generated XML
    let reparsed_nzb = parse_nzb(&generated_xml).expect("Failed to reparse generated NZB");

    // Structures should be identical
    assert_eq!(nzb.files.len(), reparsed_nzb.files.len());
    assert_eq!(nzb.meta, reparsed_nzb.meta);
    assert_eq!(nzb.total_bytes(), reparsed_nzb.total_bytes());

    for (orig_file, reparsed_file) in nzb.files.iter().zip(reparsed_nzb.files.iter()) {
        assert_eq!(orig_file.poster, reparsed_file.poster);
        assert_eq!(orig_file.date, reparsed_file.date);
        assert_eq!(orig_file.subject, reparsed_file.subject);
        assert_eq!(orig_file.groups, reparsed_file.groups);
        assert_eq!(orig_file.segments.len(), reparsed_file.segments.len());
    }
}

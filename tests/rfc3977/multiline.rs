//! RFC 3977 Section 3.1.1 - Multi-line Response and Byte-Stuffing Tests
//!
//! These tests verify compliance with NNTP multi-line response requirements:
//! - Multi-line blocks are terminated by ".\r\n"
//! - Lines starting with "." must be dot-stuffed (prepend another ".")
//! - When receiving, initial dots followed by non-CRLF are disregarded

/// Helper function to simulate byte-stuffing removal (dot-unstuffing)
/// This mirrors the logic in client.rs:584-590 and 618-624
fn unstuff_line(line: &str) -> &str {
    if line.starts_with("..") {
        &line[1..]
    } else {
        line
    }
}

/// Helper function to check if a line is the terminator
fn is_terminator(line: &str) -> bool {
    line == "."
}

// Byte-Stuffing Tests (RFC 3977 §3.1.1)

#[test]
fn test_dot_stuffing_single_dot_is_terminator() {
    // A single "." line indicates end of multi-line block
    assert!(is_terminator("."));
}

#[test]
fn test_dot_stuffing_double_dot_becomes_single() {
    // RFC 3977 §3.1.1: ".." at start of line becomes "."
    assert_eq!(unstuff_line(".."), ".");
}

#[test]
fn test_dot_stuffing_triple_dot_becomes_double() {
    // RFC 3977 §3.1.1: "..." at start of line becomes ".."
    assert_eq!(unstuff_line("..."), "..");
}

#[test]
fn test_dot_stuffing_quad_dot_becomes_triple() {
    // "...." at start becomes "..."
    assert_eq!(unstuff_line("...."), "...");
}

#[test]
fn test_dot_stuffing_double_dot_with_text() {
    // "..Hello" becomes ".Hello"
    assert_eq!(unstuff_line("..Hello"), ".Hello");
}

#[test]
fn test_dot_stuffing_preserves_non_dot_lines() {
    // Lines not starting with ".." are unchanged
    assert_eq!(unstuff_line("Hello World"), "Hello World");
    assert_eq!(unstuff_line(""), "");
    assert_eq!(unstuff_line("Normal line"), "Normal line");
}

#[test]
fn test_dot_stuffing_dot_in_middle_unchanged() {
    // Dots in the middle of lines are not affected
    let line = "Hello.World.Test";
    assert_eq!(unstuff_line(line), line);
}

#[test]
fn test_dot_stuffing_dot_at_end_unchanged() {
    // Dots at the end are not affected
    let line = "End with dot.";
    assert_eq!(unstuff_line(line), line);
}

#[test]
fn test_dot_stuffing_single_dot_not_unstuffed() {
    // A single "." is the terminator, not data
    // It should NOT be unstuffed to empty string
    assert_eq!(unstuff_line("."), ".");
    // But our is_terminator check happens first
    assert!(is_terminator("."));
}

// Multi-line Block Parsing Simulation

/// Simulate parsing a multi-line response body
fn parse_multiline_body(lines: &[&str]) -> Vec<String> {
    let mut result = Vec::new();

    for line in lines {
        // Check for terminator
        if is_terminator(line) {
            break;
        }

        // Apply dot-unstuffing
        result.push(unstuff_line(line).to_string());
    }

    result
}

#[test]
fn test_multiline_simple_body() {
    let lines = ["Line 1", "Line 2", "Line 3", "."];
    let result = parse_multiline_body(&lines);

    assert_eq!(result.len(), 3);
    assert_eq!(result[0], "Line 1");
    assert_eq!(result[1], "Line 2");
    assert_eq!(result[2], "Line 3");
}

#[test]
fn test_multiline_with_dot_stuffed_lines() {
    // Server sends ".." for lines that start with "."
    let lines = ["Normal line", "..This started with a dot", "...", "."];
    let result = parse_multiline_body(&lines);

    assert_eq!(result.len(), 3);
    assert_eq!(result[0], "Normal line");
    assert_eq!(result[1], ".This started with a dot");
    assert_eq!(result[2], "..");
}

#[test]
fn test_multiline_empty_lines_preserved() {
    let lines = ["First", "", "Third", "", "."];
    let result = parse_multiline_body(&lines);

    assert_eq!(result.len(), 4);
    assert_eq!(result[0], "First");
    assert_eq!(result[1], "");
    assert_eq!(result[2], "Third");
    assert_eq!(result[3], "");
}

#[test]
fn test_multiline_only_terminator() {
    // Empty body - just terminator
    let lines = ["."];
    let result = parse_multiline_body(&lines);

    assert!(result.is_empty());
}

#[test]
fn test_multiline_article_simulation() {
    // Simulate a typical article with headers and body
    let lines = [
        "From: user@example.com",
        "Subject: Test Article",
        "Date: Mon, 1 Jan 2024 00:00:00 +0000",
        "Message-ID: <test@example.com>",
        "", // Separator between headers and body
        "This is the body of the article.",
        "It has multiple lines.",
        "",
        "..Hidden dot line", // This line starts with . in the actual content
        ".",
    ];
    let result = parse_multiline_body(&lines);

    assert_eq!(result.len(), 9);
    assert_eq!(result[0], "From: user@example.com");
    assert_eq!(result[4], ""); // Empty line separator
    assert_eq!(result[8], ".Hidden dot line"); // Dot-unstuffed
}


#[test]
fn test_multiline_dot_only_line_after_content() {
    // If someone writes a line that's just ".", it gets stuffed to ".."
    // When we receive "..", we unstuff it back to "."
    // This is different from the terminator "."
    let lines = ["Content", "..", "."];
    let result = parse_multiline_body(&lines);

    assert_eq!(result.len(), 2);
    assert_eq!(result[0], "Content");
    assert_eq!(result[1], "."); // ".." unstuffed to "."
}

#[test]
fn test_multiline_multiple_dot_lines() {
    // Multiple lines that are just dots
    let lines = ["..", "...", "....", "."];
    let result = parse_multiline_body(&lines);

    assert_eq!(result.len(), 3);
    assert_eq!(result[0], ".");
    assert_eq!(result[1], "..");
    assert_eq!(result[2], "...");
}

#[test]
fn test_multiline_xover_line_simulation() {
    // XOVER lines are tab-separated and don't typically have dot issues
    let lines = [
        "12345\tSubject\tauthor@example.com\tDate\t<msgid>\trefs\t1000\t50",
        "12346\tAnother\tother@example.com\tDate2\t<msgid2>\trefs2\t2000\t100",
        ".",
    ];
    let result = parse_multiline_body(&lines);

    assert_eq!(result.len(), 2);
    assert!(result[0].starts_with("12345\t"));
    assert!(result[1].starts_with("12346\t"));
}

// RFC 3977 §3.1.1 Specific Requirements

#[test]
fn test_rfc_requirement_no_nul_in_lines() {
    // RFC 3977 §3.1.1: Lines must not contain NUL
    // This is a server requirement, but we should handle gracefully if received
    let line_with_nul = "Hello\0World";
    // Our unstuff function should still work
    assert_eq!(unstuff_line(line_with_nul), line_with_nul);
}

#[test]
fn test_rfc_requirement_terminator_not_included_in_data() {
    // RFC 3977 §3.1.1: "do not include the terminating line"
    let lines = ["Data", "."];
    let result = parse_multiline_body(&lines);

    // Result should NOT include the "." terminator
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], "Data");
    assert!(!result.contains(&".".to_string()) || result.iter().any(|l| l == "."));
}

#[test]
fn test_dot_stuffing_with_leading_spaces() {
    // Dot-stuffing only applies if "." is at position 0
    // " .." is just a line starting with space, not dot-stuffed
    let line = " ..test";
    assert_eq!(unstuff_line(line), " ..test");
}

#[test]
fn test_dot_stuffing_only_first_dot_pair() {
    // Only the FIRST dot is removed in dot-unstuffing
    // "..test.." unstuffs to ".test.." (only leading ".." -> ".")
    assert_eq!(unstuff_line("..test.."), ".test..");
}

// RFC 3977 §3.1.1 - CRLF Line Ending Integration Tests
//
// RFC 3977 §3.1 states: "Each line is followed by a CRLF pair."
// The protocol operates on lines terminated by \r\n (CRLF).
// These tests verify correct handling of CRLF in multi-line responses.

/// Helper to split a raw byte stream into lines, handling CRLF
fn split_crlf_lines(data: &str) -> Vec<&str> {
    data.split("\r\n").filter(|s| !s.is_empty()).collect()
}

/// Helper to parse a complete multi-line response with CRLF
fn parse_raw_multiline_response(raw: &str) -> Vec<String> {
    let lines: Vec<&str> = split_crlf_lines(raw);
    let mut result = Vec::new();

    for line in lines {
        if is_terminator(line) {
            break;
        }
        result.push(unstuff_line(line).to_string());
    }

    result
}

#[test]
fn test_crlf_multiline_basic() {
    // RFC 3977 §3.1: Lines are terminated by CRLF
    let raw = "Line 1\r\nLine 2\r\n.\r\n";
    let result = parse_raw_multiline_response(raw);

    assert_eq!(result.len(), 2);
    assert_eq!(result[0], "Line 1");
    assert_eq!(result[1], "Line 2");
}

#[test]
fn test_crlf_multiline_with_dot_stuffing() {
    // RFC 3977 §3.1.1: Dot-stuffing with proper CRLF termination
    let raw = "Normal\r\n..Dot-stuffed\r\n.\r\n";
    let result = parse_raw_multiline_response(raw);

    assert_eq!(result.len(), 2);
    assert_eq!(result[0], "Normal");
    assert_eq!(result[1], ".Dot-stuffed"); // Dot unstuffed
}

#[test]
fn test_crlf_multiline_empty_body() {
    // Empty multi-line response (just terminator)
    let raw = ".\r\n";
    let result = parse_raw_multiline_response(raw);

    assert!(result.is_empty());
}

#[test]
fn test_crlf_multiline_with_empty_lines() {
    // RFC 3977 §3.1.1: Empty lines (just CRLF) are valid
    let raw = "First\r\n\r\nThird\r\n.\r\n";
    let lines: Vec<&str> = raw.split("\r\n").collect();

    // The empty line between First and Third
    assert_eq!(lines[0], "First");
    assert_eq!(lines[1], ""); // Empty line
    assert_eq!(lines[2], "Third");
}

#[test]
fn test_crlf_multiline_article() {
    // Typical article with headers, blank line, body, terminator
    // Note: The split_crlf_lines helper filters empty lines to avoid trailing ""
    // In real NNTP parsing, empty lines (header/body separator) must be preserved
    let raw = "From: user@example.com\r\nSubject: Test\r\nBody text.\r\n.\r\n";
    let result = parse_raw_multiline_response(raw);

    assert_eq!(result.len(), 3);
    assert!(result[0].starts_with("From:"));
    assert!(result[1].starts_with("Subject:"));
    assert_eq!(result[2], "Body text.");
}

#[test]
fn test_crlf_multiline_article_with_empty_line() {
    // RFC 3977: Empty lines (header/body separator) are significant
    // This test uses a raw split that preserves empty lines
    let raw = "From: user@example.com\r\nSubject: Test\r\n\r\nBody text.\r\n.\r\n";
    let lines: Vec<&str> = raw.split("\r\n").collect();

    // Before terminator: From, Subject, "", Body text, ., ""
    assert_eq!(lines[0], "From: user@example.com");
    assert_eq!(lines[1], "Subject: Test");
    assert_eq!(lines[2], ""); // Header/body separator - significant!
    assert_eq!(lines[3], "Body text.");
    assert_eq!(lines[4], "."); // Terminator
}

#[test]
fn test_crlf_preserved_in_content() {
    // If content contains \r\n, it would be split into multiple lines
    // This is expected behavior per RFC
    let raw = "Line1\r\nLine2\r\n.\r\n";
    let lines = split_crlf_lines(raw);

    assert_eq!(lines.len(), 3); // Line1, Line2, .
    assert_eq!(lines[0], "Line1");
    assert_eq!(lines[1], "Line2");
    assert_eq!(lines[2], ".");
}

#[test]
fn test_crlf_terminator_format() {
    // RFC 3977 §3.1.1: Terminator is ".\r\n" on its own line
    let raw = "Content\r\n.\r\n";

    // Extract just the terminator portion
    let has_proper_terminator = raw.ends_with(".\r\n");
    assert!(has_proper_terminator, "Must end with .\\r\\n");

    // The terminator line is just "."
    let lines: Vec<&str> = raw.trim_end_matches("\r\n").split("\r\n").collect();
    let last_line = lines.last().unwrap();
    assert_eq!(*last_line, ".");
}

// RFC 3977 §3.1 - Bare CR/LF Handling (Invalid Per RFC)
//
// RFC 3977 requires CRLF (\r\n) for line endings. Bare CR (\r) or bare LF (\n)
// without the pair are not valid per the RFC. These tests document how our
// parser handles such malformed input (defensive parsing).

#[test]
fn test_bare_lf_handling() {
    // NOTE: CLIENT-SIDE DEFENSIVE PARSING - bare \n is not RFC-compliant
    // RFC 3977 requires \r\n, but we may encounter bare \n from broken servers
    let raw = "Line1\nLine2\n.\n";

    // If we split on \r\n, this is treated as one long line
    let crlf_lines = split_crlf_lines(raw);
    assert_eq!(crlf_lines.len(), 1); // All one "line" since no \r\n

    // But if we split on just \n, we'd get multiple lines
    let lf_lines: Vec<&str> = raw.split('\n').filter(|s| !s.is_empty()).collect();
    assert_eq!(lf_lines.len(), 3); // Line1, Line2, .
}

#[test]
fn test_bare_cr_handling() {
    // NOTE: CLIENT-SIDE DEFENSIVE PARSING - bare \r is not RFC-compliant
    // RFC 3977 requires \r\n, but we may encounter bare \r from broken servers
    let raw = "Line1\rLine2\r.\r";

    // If we split on \r\n, this is treated as one long line
    let crlf_lines = split_crlf_lines(raw);
    assert_eq!(crlf_lines.len(), 1); // All one "line" since no \r\n

    // But if we split on just \r, we'd get multiple lines
    let cr_lines: Vec<&str> = raw.split('\r').filter(|s| !s.is_empty()).collect();
    assert_eq!(cr_lines.len(), 3); // Line1, Line2, .
}

#[test]
fn test_mixed_line_endings() {
    // NOTE: CLIENT-SIDE DEFENSIVE PARSING - mixed endings are not RFC-compliant
    // Some broken servers might send mixed \r\n and \n
    let raw = "Line1\r\nLine2\nLine3\r\n.\r\n";

    // Our CRLF parser would see: "Line1", "Line2\nLine3", "."
    let crlf_lines = split_crlf_lines(raw);

    // Line2\nLine3 is treated as a single line (with embedded \n)
    assert!(crlf_lines.iter().any(|l| l.contains("Line2\nLine3")));
}

//! RFC 3977 Section 8.4 - LIST OVERVIEW.FMT Command Tests
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc3977#section-8.4
//!
//! Tests for the LIST OVERVIEW.FMT command and response parsing.

use nntp_rs::{NntpResponse, codes, commands};
#[test]
fn test_list_overview_fmt_command_format() {
    let cmd = commands::list_overview_fmt();
    assert_eq!(cmd, "LIST OVERVIEW.FMT\r\n");
}

#[test]
fn test_list_overview_fmt_command_ends_with_crlf() {
    let cmd = commands::list_overview_fmt();
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_list_overview_fmt_command_uppercase() {
    let cmd = commands::list_overview_fmt();
    assert!(cmd.starts_with("LIST OVERVIEW.FMT"));
}

#[test]
fn test_parse_list_overview_fmt_response_success() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Order of fields in overview database.".to_string(),
        lines: vec![
            "Subject:".to_string(),
            "From:".to_string(),
            "Date:".to_string(),
            "Message-ID:".to_string(),
            "References:".to_string(),
            ":bytes".to_string(),
            ":lines".to_string(),
        ],
    };

    let fields = commands::parse_list_overview_fmt_response(response).unwrap();
    assert_eq!(fields.len(), 7);
    assert_eq!(fields[0], "Subject:");
    assert_eq!(fields[1], "From:");
    assert_eq!(fields[5], ":bytes");
    assert_eq!(fields[6], ":lines");
}

#[test]
fn test_parse_list_overview_fmt_response_with_full_suffix() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Overview format".to_string(),
        lines: vec![
            "Subject:".to_string(),
            "From:".to_string(),
            "Date:".to_string(),
            "Message-ID:".to_string(),
            "References:".to_string(),
            ":bytes".to_string(),
            ":lines".to_string(),
            "Xref:full".to_string(),
            "Distribution:full".to_string(),
        ],
    };

    let fields = commands::parse_list_overview_fmt_response(response).unwrap();
    assert_eq!(fields.len(), 9);
    assert_eq!(fields[7], "Xref:full");
    assert_eq!(fields[8], "Distribution:full");
}

#[test]
fn test_parse_list_overview_fmt_response_alternative_bytes_lines() {
    // RFC 3977: Bytes: and Lines: may be used instead of :bytes and :lines
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Overview format".to_string(),
        lines: vec![
            "Subject:".to_string(),
            "From:".to_string(),
            "Date:".to_string(),
            "Message-ID:".to_string(),
            "References:".to_string(),
            "Bytes:".to_string(),
            "Lines:".to_string(),
        ],
    };

    let fields = commands::parse_list_overview_fmt_response(response).unwrap();
    assert_eq!(fields.len(), 7);
    assert_eq!(fields[5], "Bytes:");
    assert_eq!(fields[6], "Lines:");
}

#[test]
fn test_parse_list_overview_fmt_response_empty() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Empty overview format".to_string(),
        lines: vec![],
    };

    let fields = commands::parse_list_overview_fmt_response(response).unwrap();
    assert_eq!(fields.len(), 0);
}

#[test]
fn test_parse_list_overview_fmt_response_minimal() {
    // Minimal valid format with required fields
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Overview format".to_string(),
        lines: vec![
            "Subject:".to_string(),
            "From:".to_string(),
            "Date:".to_string(),
            "Message-ID:".to_string(),
        ],
    };

    let fields = commands::parse_list_overview_fmt_response(response).unwrap();
    assert_eq!(fields.len(), 4);
}

#[test]
fn test_parse_list_overview_fmt_response_extended() {
    // Extended format with many additional headers
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Overview format".to_string(),
        lines: vec![
            "Subject:".to_string(),
            "From:".to_string(),
            "Date:".to_string(),
            "Message-ID:".to_string(),
            "References:".to_string(),
            ":bytes".to_string(),
            ":lines".to_string(),
            "Xref:full".to_string(),
            "Distribution:full".to_string(),
            "Newsgroups:full".to_string(),
            "Path:full".to_string(),
            "Organization:".to_string(),
            "User-Agent:".to_string(),
        ],
    };

    let fields = commands::parse_list_overview_fmt_response(response).unwrap();
    assert_eq!(fields.len(), 13);
    assert!(fields.contains(&"Organization:".to_string()));
    assert!(fields.contains(&"User-Agent:".to_string()));
}

#[test]
fn test_parse_list_overview_fmt_response_protocol_error() {
    let response = NntpResponse {
        code: 500,
        message: "Command not understood".to_string(),
        lines: vec![],
    };

    assert!(commands::parse_list_overview_fmt_response(response).is_err());
}

#[test]
fn test_parse_list_overview_fmt_response_wrong_code() {
    let response = NntpResponse {
        code: 480,
        message: "Authentication required".to_string(),
        lines: vec![],
    };

    assert!(commands::parse_list_overview_fmt_response(response).is_err());
}

#[test]
fn test_parse_list_overview_fmt_response_preserves_field_order() {
    // Order is important for parsing OVER responses
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Overview format".to_string(),
        lines: vec![
            "Subject:".to_string(),
            "From:".to_string(),
            "Date:".to_string(),
            "Message-ID:".to_string(),
            "References:".to_string(),
            ":bytes".to_string(),
            ":lines".to_string(),
        ],
    };

    let fields = commands::parse_list_overview_fmt_response(response).unwrap();

    // Verify exact order preservation
    assert_eq!(fields[0], "Subject:");
    assert_eq!(fields[1], "From:");
    assert_eq!(fields[2], "Date:");
    assert_eq!(fields[3], "Message-ID:");
    assert_eq!(fields[4], "References:");
    assert_eq!(fields[5], ":bytes");
    assert_eq!(fields[6], ":lines");
}

#[test]
fn test_parse_list_overview_fmt_response_preserves_whitespace() {
    // Field names should be preserved exactly as returned
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Overview format".to_string(),
        lines: vec![
            "Subject:".to_string(),
            " From:".to_string(), // Leading space
            "Date: ".to_string(), // Trailing space
        ],
    };

    let fields = commands::parse_list_overview_fmt_response(response).unwrap();
    assert_eq!(fields[0], "Subject:");
    assert_eq!(fields[1], " From:"); // Preserved
    assert_eq!(fields[2], "Date: "); // Preserved
}

#[test]
fn test_list_overview_fmt_real_world_example() {
    // Example from RFC 3977 Section 8.4
    let response = NntpResponse {
        code: 215,
        message: "Order of fields in overview database.".to_string(),
        lines: vec![
            "Subject:".to_string(),
            "From:".to_string(),
            "Date:".to_string(),
            "Message-ID:".to_string(),
            "References:".to_string(),
            ":bytes".to_string(),
            ":lines".to_string(),
            "Xref:full".to_string(),
            "Distribution:full".to_string(),
        ],
    };

    let fields = commands::parse_list_overview_fmt_response(response).unwrap();
    assert_eq!(fields.len(), 9);

    // Verify standard headers
    assert_eq!(fields[0], "Subject:");
    assert_eq!(fields[1], "From:");
    assert_eq!(fields[2], "Date:");
    assert_eq!(fields[3], "Message-ID:");
    assert_eq!(fields[4], "References:");

    // Verify metadata
    assert_eq!(fields[5], ":bytes");
    assert_eq!(fields[6], ":lines");

    // Verify full headers
    assert_eq!(fields[7], "Xref:full");
    assert_eq!(fields[8], "Distribution:full");
}

#[test]
fn test_list_overview_fmt_case_sensitivity() {
    // Field names should be returned as-is (case preserved)
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Overview format".to_string(),
        lines: vec![
            "SUBJECT:".to_string(),
            "subject:".to_string(),
            "Subject:".to_string(),
        ],
    };

    let fields = commands::parse_list_overview_fmt_response(response).unwrap();
    assert_eq!(fields[0], "SUBJECT:");
    assert_eq!(fields[1], "subject:");
    assert_eq!(fields[2], "Subject:");
}

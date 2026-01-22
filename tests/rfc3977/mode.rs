//! RFC 3977 Section 5.3 - MODE READER Command Tests
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc3977#section-5.3
//!
//! The MODE READER command instructs the server to switch to reader mode,
//! indicating this is a news reading client (as opposed to a news transfer agent).

use nntp_rs::{codes, commands, NntpResponse};
#[test]
fn test_mode_reader_command_format() {
    let cmd = commands::mode_reader();
    assert_eq!(cmd, "MODE READER\r\n");
}

#[test]
fn test_mode_reader_command_ends_with_crlf() {
    let cmd = commands::mode_reader();
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_mode_reader_command_uppercase() {
    let cmd = commands::mode_reader();
    assert!(cmd.starts_with("MODE READER"));
}
#[test]
fn test_mode_reader_response_posting_allowed() {
    let response = NntpResponse {
        code: codes::READY_POSTING_ALLOWED,
        message: "Posting allowed".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 200);
    assert!(response.is_success());
}

#[test]
fn test_mode_reader_response_no_posting() {
    let response = NntpResponse {
        code: codes::READY_NO_POSTING,
        message: "Posting prohibited".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 201);
    assert!(response.is_success());
}

#[test]
fn test_mode_reader_response_codes_are_success() {
    let response_200 = NntpResponse {
        code: codes::READY_POSTING_ALLOWED,
        message: "Ready".to_string(),
        lines: vec![],
    };

    let response_201 = NntpResponse {
        code: codes::READY_NO_POSTING,
        message: "Ready".to_string(),
        lines: vec![],
    };

    assert!(response_200.is_success());
    assert!(response_201.is_success());
    assert!(!response_200.is_error());
    assert!(!response_201.is_error());
}
#[test]
fn test_mode_reader_typical_posting_allowed_message() {
    let response = NntpResponse {
        code: codes::READY_POSTING_ALLOWED,
        message: "Posting allowed".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 200);
    assert!(response.message.contains("Posting"));
}

#[test]
fn test_mode_reader_typical_no_posting_message() {
    let response = NntpResponse {
        code: codes::READY_NO_POSTING,
        message: "Posting prohibited".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 201);
    assert!(response.message.contains("prohibited") || response.message.contains("not"));
}

#[test]
fn test_mode_reader_response_has_no_multiline_data() {
    // MODE READER always returns single-line response
    let response = NntpResponse {
        code: codes::READY_POSTING_ALLOWED,
        message: "Ready".to_string(),
        lines: vec![],
    };

    assert!(response.lines.is_empty());
}

// Real-World Examples

#[test]
fn test_mode_reader_posting_server_response() {
    // Typical response from a server that allows posting
    let response = NntpResponse {
        code: codes::READY_POSTING_ALLOWED,
        message: "Server ready - posting allowed".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 200);
    assert!(response.is_success());
}

#[test]
fn test_mode_reader_read_only_server_response() {
    // Typical response from a read-only server
    let response = NntpResponse {
        code: codes::READY_NO_POSTING,
        message: "Server ready - no posting allowed".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 201);
    assert!(response.is_success());
}

#[test]
fn test_mode_reader_usenet_provider_posting_allowed() {
    // Common Usenet provider response
    let response = NntpResponse {
        code: codes::READY_POSTING_ALLOWED,
        message: "News server ready (posting ok)".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 200);
}

#[test]
fn test_mode_reader_usenet_provider_no_posting() {
    // Common Usenet provider response for read-only access
    let response = NntpResponse {
        code: codes::READY_NO_POSTING,
        message: "News server ready (no posting)".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 201);
}
#[test]
fn test_mode_reader_error_response_is_error() {
    // Some servers may reject MODE READER
    let response = NntpResponse {
        code: 502,
        message: "Command unavailable".to_string(),
        lines: vec![],
    };

    assert!(response.is_error());
    assert!(!response.is_success());
}

#[test]
fn test_mode_reader_authentication_required_error() {
    // Server may require authentication before MODE READER
    let response = NntpResponse {
        code: 480,
        message: "Authentication required".to_string(),
        lines: vec![],
    };

    assert!(response.is_error());
    assert_eq!(response.code, 480);
}


#[test]
fn test_mode_reader_minimal_message() {
    let response = NntpResponse {
        code: codes::READY_POSTING_ALLOWED,
        message: "OK".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 200);
    assert!(response.is_success());
}

#[test]
fn test_mode_reader_verbose_message() {
    let response = NntpResponse {
        code: codes::READY_NO_POSTING,
        message: "Reader mode activated successfully. This server is read-only and does not accept article postings from clients.".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 201);
    assert!(response.is_success());
}

#[test]
fn test_mode_reader_response_codes_distinct() {
    // Ensure 200 and 201 are different
    assert_ne!(codes::READY_POSTING_ALLOWED, codes::READY_NO_POSTING);
    assert_eq!(codes::READY_POSTING_ALLOWED, 200);
    assert_eq!(codes::READY_NO_POSTING, 201);
}

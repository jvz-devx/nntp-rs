//! RFC 4644 Section 2.3 - MODE STREAM Command Tests
//!
//! These tests verify the MODE STREAM command implementation:
//! - Command format
//! - Response code handling (203 STREAMING_OK)
//! - Error handling
//! - Integration with CAPABILITIES
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc4644#section-2.3

use nntp_rs::{codes, commands, NntpResponse};
#[test]
fn test_mode_stream_format() {
    let cmd = commands::mode_stream();
    assert_eq!(cmd, "MODE STREAM\r\n");
}

#[test]
fn test_mode_stream_ends_with_crlf() {
    let cmd = commands::mode_stream();
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_mode_stream_uppercase() {
    let cmd = commands::mode_stream();
    assert!(cmd.starts_with("MODE STREAM"));
}

#[test]
fn test_mode_stream_no_arguments() {
    let cmd = commands::mode_stream();
    // MODE STREAM takes no arguments
    assert_eq!(cmd, "MODE STREAM\r\n");
}
#[test]
fn test_streaming_ok_response() {
    // 203 = Streaming OK (RFC 4644 Section 2.3)
    let response = NntpResponse {
        code: codes::STREAMING_OK,
        message: "Streaming permitted".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 203);
    assert!(response.is_success());
}

#[test]
fn test_mode_stream_success_response() {
    // Valid success response to MODE STREAM
    let response = NntpResponse {
        code: codes::STREAMING_OK,
        message: "Stream commands allowed".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 203);
    assert!(response.is_success());
    assert!(!response.is_error());
}

#[test]
fn test_mode_stream_error_503() {
    // 503 = Program error, function not performed
    let response = NntpResponse {
        code: 503,
        message: "Streaming not supported".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 503);
    assert!(response.is_error());
}

#[test]
fn test_mode_stream_error_502() {
    // 502 = Command unavailable
    let response = NntpResponse {
        code: 502,
        message: "Command not implemented".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 502);
    assert!(response.is_error());
}

// RFC 4644 Section 2.3 Example

#[test]
fn test_rfc4644_section_2_3_example() {
    // RFC 4644 Section 2.3 specifies MODE STREAM command
    let cmd = commands::mode_stream();
    assert_eq!(cmd, "MODE STREAM\r\n");

    // Expected response: 203 Streaming permitted
    let response = NntpResponse {
        code: codes::STREAMING_OK,
        message: "Streaming permitted".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 203);
    assert!(response.is_success());
}

#[test]
fn test_mode_stream_response_variations() {
    // Different valid 203 response messages
    let messages = vec![
        "Streaming permitted",
        "Stream commands allowed",
        "Streaming OK",
        "Stream mode",
    ];

    for message in messages {
        let response = NntpResponse {
            code: codes::STREAMING_OK,
            message: message.to_string(),
            lines: vec![],
        };
        assert_eq!(response.code, 203);
        assert!(response.is_success());
    }
}

#[test]
fn test_mode_stream_wrong_success_code() {
    // MODE STREAM should return 203, not other 2xx codes
    let response = NntpResponse {
        code: 200, // Wrong success code
        message: "OK".to_string(),
        lines: vec![],
    };

    assert_ne!(response.code, codes::STREAMING_OK);
    assert!(response.is_success()); // Still a success code
}

// Real-World Scenarios

#[test]
fn test_mode_stream_capability_check_workflow() {
    // Typical workflow: check CAPABILITIES, then MODE STREAM
    // This test documents the expected usage pattern

    // Step 1: Server advertises STREAMING in capabilities
    let caps_response = NntpResponse {
        code: 101,
        message: "Capability list follows".to_string(),
        lines: vec![
            "VERSION 2".to_string(),
            "READER".to_string(),
            "STREAMING".to_string(),
            "POST".to_string(),
            ".".to_string(),
        ],
    };

    // Check for STREAMING capability
    let has_streaming = caps_response
        .lines
        .iter()
        .any(|line| line.starts_with("STREAMING"));
    assert!(has_streaming);

    // Step 2: Send MODE STREAM
    let cmd = commands::mode_stream();
    assert_eq!(cmd, "MODE STREAM\r\n");

    // Step 3: Receive 203 response
    let stream_response = NntpResponse {
        code: codes::STREAMING_OK,
        message: "Streaming permitted".to_string(),
        lines: vec![],
    };
    assert_eq!(stream_response.code, 203);
}

#[test]
fn test_mode_stream_not_supported() {
    // Server doesn't support streaming
    let response = NntpResponse {
        code: 503,
        message: "Streaming extension not available".to_string(),
        lines: vec![],
    };

    assert!(response.is_error());
    assert_eq!(response.code, 503);
}

#[test]
fn test_mode_stream_server_to_server() {
    // MODE STREAM is typically used for server-to-server article transfer
    // This test documents the expected use case

    let cmd = commands::mode_stream();
    assert_eq!(cmd, "MODE STREAM\r\n");

    // After successful MODE STREAM, server accepts CHECK/TAKETHIS
    let response = NntpResponse {
        code: codes::STREAMING_OK,
        message: "Streaming permitted".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 203);
    assert!(response.is_success());
}

// Error Handling

#[test]
fn test_mode_stream_protocol_error() {
    // Protocol error response
    let response = NntpResponse {
        code: 500,
        message: "Command not understood".to_string(),
        lines: vec![],
    };

    assert!(response.is_error());
    assert!(!response.is_success());
}

#[test]
fn test_mode_stream_already_in_mode() {
    // Some servers might return error if already in streaming mode
    // Though spec doesn't explicitly define this behavior
    let response = NntpResponse {
        code: 502,
        message: "Already in streaming mode".to_string(),
        lines: vec![],
    };

    assert!(response.is_error());
}

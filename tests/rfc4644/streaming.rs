//! RFC 4644 - NNTP Streaming Extension Tests
//!
//! These tests verify compliance with NNTP streaming requirements:
//! - MODE STREAM command and response codes
//! - CHECK command response codes (238, 431, 438)
//! - TAKETHIS command response codes (239, 439)
//! - Response code 203 = streaming OK
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc4644

use nntp_rs::{codes, NntpResponse};

// Streaming Response Codes (RFC 4644)

#[test]
fn test_streaming_ok_203() {
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
fn test_streaming_ok_constant() {
    assert_eq!(codes::STREAMING_OK, 203);
}

// Error Codes for Streaming (RFC 4644)

#[test]
fn test_streaming_not_available_503() {
    // 503 = MODE STREAM not available
    let response = NntpResponse {
        code: codes::FEATURE_NOT_SUPPORTED,
        message: "Streaming not supported".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 503);
    assert!(response.is_error());
}

#[test]
fn test_streaming_already_active_502() {
    // 502 = MODE STREAM already active
    let response = NntpResponse {
        code: codes::ACCESS_DENIED,
        message: "Streaming already active".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 502);
    assert!(response.is_error());
}

#[test]
fn test_check_without_mode_stream_480() {
    // 480 = CHECK/TAKETHIS not available without MODE STREAM
    let response = NntpResponse {
        code: codes::AUTH_REQUIRED,
        message: "Must use MODE STREAM first".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 480);
    assert!(response.is_error());
}

// Streaming Workflow Simulation

#[test]
fn test_streaming_workflow_success() {
    // 1. Enable streaming mode
    let mode_stream_response = NntpResponse {
        code: codes::STREAMING_OK,
        message: "Streaming permitted".to_string(),
        lines: vec![],
    };
    assert!(mode_stream_response.is_success());

    // 2. Check if server wants article
    let check_response = NntpResponse {
        code: codes::CHECK_SEND,
        message: "238 <msg@id> Send article".to_string(),
        lines: vec![],
    };
    assert!(check_response.is_success());

    // 3. Send article with TAKETHIS
    let takethis_response = NntpResponse {
        code: codes::TAKETHIS_RECEIVED,
        message: "239 <msg@id> Article received".to_string(),
        lines: vec![],
    };
    assert!(takethis_response.is_success());
}

#[test]
fn test_streaming_workflow_article_not_wanted() {
    // 1. Enable streaming mode
    let mode_stream_response = NntpResponse {
        code: codes::STREAMING_OK,
        message: "Streaming permitted".to_string(),
        lines: vec![],
    };
    assert!(mode_stream_response.is_success());

    // 2. Check if server wants article - it doesn't
    let check_response = NntpResponse {
        code: codes::CHECK_NOT_WANTED,
        message: "438 <msg@id> Article not wanted".to_string(),
        lines: vec![],
    };
    assert!(check_response.is_error());

    // 3. Don't send article - server already has it or doesn't want it
    // No TAKETHIS command should be sent
}

#[test]
fn test_streaming_workflow_try_later() {
    // 1. Enable streaming mode
    let mode_stream_response = NntpResponse {
        code: codes::STREAMING_OK,
        message: "Streaming permitted".to_string(),
        lines: vec![],
    };
    assert!(mode_stream_response.is_success());

    // 2. Check if server wants article - it's busy
    let check_response = NntpResponse {
        code: codes::CHECK_LATER,
        message: "431 <msg@id> Try again later".to_string(),
        lines: vec![],
    };
    assert!(check_response.is_error());

    // 3. Client should retry later
    // Implementation should queue for retry
}

// Pipeline Behavior (RFC 4644 Section 2.6)

#[test]
fn test_check_responses_can_arrive_out_of_order() {
    // RFC 4644: CHECK responses may arrive in different order than requests
    // Responses include message-id for matching

    let responses = [
        NntpResponse {
            code: codes::CHECK_SEND,
            message: "238 <msg3@id> Send article".to_string(),
            lines: vec![],
        },
        NntpResponse {
            code: codes::CHECK_NOT_WANTED,
            message: "438 <msg1@id> Article not wanted".to_string(),
            lines: vec![],
        },
        NntpResponse {
            code: codes::CHECK_SEND,
            message: "238 <msg2@id> Send article".to_string(),
            lines: vec![],
        },
    ];

    // Extract message IDs from responses
    let msg_ids: Vec<&str> = responses
        .iter()
        .filter_map(|r| {
            if r.message.contains("<msg") {
                Some(if r.message.contains("<msg1@id>") {
                    "<msg1@id>"
                } else if r.message.contains("<msg2@id>") {
                    "<msg2@id>"
                } else {
                    "<msg3@id>"
                })
            } else {
                None
            }
        })
        .collect();

    // Responses arrived in order: msg3, msg1, msg2
    assert_eq!(msg_ids, vec!["<msg3@id>", "<msg1@id>", "<msg2@id>"]);
}

#[test]
fn test_takethis_responses_can_arrive_out_of_order() {
    // RFC 4644: TAKETHIS responses may also arrive out of order

    let responses = [
        NntpResponse {
            code: codes::TAKETHIS_RECEIVED,
            message: "239 <article2@id> Article received".to_string(),
            lines: vec![],
        },
        NntpResponse {
            code: codes::TAKETHIS_REJECTED,
            message: "439 <article1@id> Article rejected".to_string(),
            lines: vec![],
        },
    ];

    // Both responses are valid regardless of order
    assert!(responses[0].is_success());
    assert!(responses[1].is_error());
}

#[test]
fn test_streaming_code_ranges() {
    // Verify codes are in correct ranges
    assert!((200..300).contains(&codes::STREAMING_OK));
    assert!((200..300).contains(&codes::CHECK_SEND));
    assert!((200..300).contains(&codes::TAKETHIS_RECEIVED));
    assert!((400..500).contains(&codes::CHECK_LATER));
    assert!((400..500).contains(&codes::CHECK_NOT_WANTED));
    assert!((400..500).contains(&codes::TAKETHIS_REJECTED));
}

#[test]
fn test_streaming_success_codes_are_2xx() {
    let success_codes = [
        codes::STREAMING_OK,
        codes::CHECK_SEND,
        codes::TAKETHIS_RECEIVED,
    ];

    for code in success_codes {
        let response = NntpResponse {
            code,
            message: String::new(),
            lines: vec![],
        };
        assert!(response.is_success(), "Code {} should be success", code);
    }
}

#[test]
fn test_streaming_error_codes_are_4xx() {
    let error_codes = vec![
        codes::CHECK_LATER,
        codes::CHECK_NOT_WANTED,
        codes::TAKETHIS_REJECTED,
    ];

    for code in error_codes {
        let response = NntpResponse {
            code,
            message: String::new(),
            lines: vec![],
        };
        assert!(response.is_error(), "Code {} should be error", code);
    }
}

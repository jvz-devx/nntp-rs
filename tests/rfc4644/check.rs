//! RFC 4644 Section 2.4 - CHECK Command Tests
//!
//! These tests verify the CHECK command implementation:
//! - Command format
//! - Response code handling (238, 431, 438)
//! - Message-ID format validation
//! - Response parsing
//! - Error handling
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc4644#section-2.4

use nntp_rs::{codes, commands, NntpResponse};
#[test]
fn test_check_format() {
    let cmd = commands::check("<article123@example.com>");
    assert_eq!(cmd, "CHECK <article123@example.com>\r\n");
}

#[test]
fn test_check_ends_with_crlf() {
    let cmd = commands::check("<test@host.com>");
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_check_uppercase() {
    let cmd = commands::check("<msg@id>");
    assert!(cmd.starts_with("CHECK "));
}

#[test]
fn test_check_message_id_format() {
    let message_id = "<abc123@news.example.com>";
    let cmd = commands::check(message_id);
    assert!(cmd.contains(message_id));
    assert_eq!(cmd, format!("CHECK {}\r\n", message_id));
}

#[test]
fn test_check_various_message_ids() {
    // Simple message-id
    let cmd1 = commands::check("<simple@host>");
    assert_eq!(cmd1, "CHECK <simple@host>\r\n");

    // Complex message-id with numbers
    let cmd2 = commands::check("<1234567890.abcdef@news.server.com>");
    assert_eq!(cmd2, "CHECK <1234567890.abcdef@news.server.com>\r\n");

    // Message-id with special characters
    let cmd3 = commands::check("<part1$part2@domain.org>");
    assert_eq!(cmd3, "CHECK <part1$part2@domain.org>\r\n");
}

#[test]
fn test_check_long_message_id() {
    // Test with a very long message-id
    let long_id = format!("<{}.{}@{}>", "a".repeat(50), "b".repeat(50), "example.com");
    let cmd = commands::check(&long_id);
    assert!(cmd.starts_with("CHECK <"));
    assert!(cmd.ends_with("\r\n"));
    assert!(cmd.contains(&long_id));
}
#[test]
fn test_check_send_238() {
    // 238 = Send article (RFC 4644 Section 2.4)
    // Server wants the article - use TAKETHIS to send it
    let response = NntpResponse {
        code: codes::CHECK_SEND,
        message: "<article123@example.com> Send article".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 238);
    assert!(response.is_success());
    assert!(!response.is_error());
}

#[test]
fn test_check_later_431() {
    // 431 = Try again later (RFC 4644 Section 2.4)
    // Server suggests retrying later
    let response = NntpResponse {
        code: codes::CHECK_LATER,
        message: "<article123@example.com> Try again later".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 431);
    assert!(response.is_error());
    assert!(!response.is_success());
}

#[test]
fn test_check_not_wanted_438() {
    // 438 = Article not wanted (RFC 4644 Section 2.4)
    // Server does not want this article
    let response = NntpResponse {
        code: codes::CHECK_NOT_WANTED,
        message: "<article123@example.com> Article not wanted".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 438);
    assert!(response.is_error());
    assert!(!response.is_success());
}

#[test]
fn test_check_response_includes_message_id() {
    // RFC 4644 Section 2.4: Response MUST include the message-id
    let message_id = "<test@example.com>";

    // 238 response with message-id
    let response1 = NntpResponse {
        code: codes::CHECK_SEND,
        message: format!("{} Send article", message_id),
        lines: vec![],
    };
    assert!(response1.message.contains(message_id));

    // 431 response with message-id
    let response2 = NntpResponse {
        code: codes::CHECK_LATER,
        message: format!("{} Try later", message_id),
        lines: vec![],
    };
    assert!(response2.message.contains(message_id));

    // 438 response with message-id
    let response3 = NntpResponse {
        code: codes::CHECK_NOT_WANTED,
        message: format!("{} Not wanted", message_id),
        lines: vec![],
    };
    assert!(response3.message.contains(message_id));
}

// RFC 4644 Section 2.4 Example Tests

#[test]
fn test_rfc_example_check_send() {
    // RFC 4644 Section 2.6.1 Example:
    // C: CHECK <i.am.an.article.you.will.want@example.com>
    // S: 238 <i.am.an.article.you.will.want@example.com>

    let cmd = commands::check("<i.am.an.article.you.will.want@example.com>");
    assert_eq!(cmd, "CHECK <i.am.an.article.you.will.want@example.com>\r\n");

    let response = NntpResponse {
        code: 238,
        message: "<i.am.an.article.you.will.want@example.com>".to_string(),
        lines: vec![],
    };
    assert_eq!(response.code, codes::CHECK_SEND);
    assert!(response.is_success());
}

#[test]
fn test_rfc_example_check_not_wanted() {
    // RFC 4644 Section 2.6.1 Example:
    // C: CHECK <i.am.an.article.you.have@example.com>
    // S: 438 <i.am.an.article.you.have@example.com>

    let cmd = commands::check("<i.am.an.article.you.have@example.com>");
    assert_eq!(cmd, "CHECK <i.am.an.article.you.have@example.com>\r\n");

    let response = NntpResponse {
        code: 438,
        message: "<i.am.an.article.you.have@example.com>".to_string(),
        lines: vec![],
    };
    assert_eq!(response.code, codes::CHECK_NOT_WANTED);
    assert!(response.is_error());
}


#[test]
fn test_check_empty_message_id() {
    // Edge case: empty message-id (invalid but test handling)
    let cmd = commands::check("");
    assert_eq!(cmd, "CHECK \r\n");
}

#[test]
fn test_check_message_id_without_brackets() {
    // Message-ID without angle brackets (technically invalid but should work)
    let cmd = commands::check("article123@example.com");
    assert_eq!(cmd, "CHECK article123@example.com\r\n");
}

#[test]
fn test_check_message_id_with_whitespace() {
    // Message-ID with internal whitespace (unusual but valid)
    let cmd = commands::check("<article 123@example.com>");
    assert!(cmd.contains("<article 123@example.com>"));
}

#[test]
fn test_check_response_variations() {
    // Various message formats in responses

    // Minimal response (just message-id)
    let r1 = NntpResponse {
        code: codes::CHECK_SEND,
        message: "<msg@id>".to_string(),
        lines: vec![],
    };
    assert!(r1.is_success());

    // Response with additional text
    let r2 = NntpResponse {
        code: codes::CHECK_LATER,
        message: "<msg@id> Server busy, try later".to_string(),
        lines: vec![],
    };
    assert!(r2.is_error());

    // Response with descriptive message
    let r3 = NntpResponse {
        code: codes::CHECK_NOT_WANTED,
        message: "<msg@id> Article already received from another peer".to_string(),
        lines: vec![],
    };
    assert!(r3.is_error());
}

// Real-World Scenarios

#[test]
fn test_streaming_workflow() {
    // Simulate a typical streaming workflow

    // 1. Check if server wants article
    let message_id = "<binary-post-part1@uploader.com>";
    let cmd = commands::check(message_id);
    assert!(cmd.contains(message_id));

    // 2. Server wants it (238)
    let response = NntpResponse {
        code: codes::CHECK_SEND,
        message: format!("{} Send it", message_id),
        lines: vec![],
    };
    assert_eq!(response.code, 238);

    // 3. Would now use TAKETHIS to send the article
    // (TAKETHIS implementation is separate)
}

#[test]
fn test_pipelined_check_responses() {
    // RFC 4644 allows pipelining - multiple CHECK commands can be sent
    // without waiting for responses. Response includes message-id for matching.

    let ids = vec![
        "<article1@example.com>",
        "<article2@example.com>",
        "<article3@example.com>",
    ];

    // Send all CHECK commands
    let commands: Vec<String> = ids.iter().map(|id| commands::check(id)).collect();
    assert_eq!(commands.len(), 3);

    // Simulate receiving responses (possibly out of order)
    let responses = vec![
        NntpResponse {
            code: codes::CHECK_SEND,
            message: format!("{} Send", ids[0]),
            lines: vec![],
        },
        NntpResponse {
            code: codes::CHECK_NOT_WANTED,
            message: format!("{} Not wanted", ids[1]),
            lines: vec![],
        },
        NntpResponse {
            code: codes::CHECK_SEND,
            message: format!("{} Send", ids[2]),
            lines: vec![],
        },
    ];

    // Verify each response can be matched to its message-id
    assert!(responses[0].message.contains(ids[0]));
    assert!(responses[1].message.contains(ids[1]));
    assert!(responses[2].message.contains(ids[2]));
}

#[test]
fn test_binary_newsgroup_message_ids() {
    // Test with typical binary newsgroup message-ids

    // yEnc multi-part naming convention
    let cmd1 = commands::check("<20240115123456.abcdef@uploader.com>");
    assert!(cmd1.contains("20240115123456"));

    // UUID-style message-id
    let cmd2 = commands::check("<550e8400-e29b-41d4-a716-446655440000@server.com>");
    assert!(cmd2.contains("550e8400-e29b-41d4-a716-446655440000"));

    // Part numbering in message-id
    let cmd3 = commands::check("<file.part001.rar@usenet.com>");
    assert!(cmd3.contains("part001"));
}

#[test]
fn test_check_after_mode_stream() {
    // CHECK requires streaming mode to be enabled first
    // This test documents the workflow

    // 1. MODE STREAM must succeed first (203)
    // (already tested in stream.rs)

    // 2. Then CHECK can be used
    let cmd = commands::check("<article@example.com>");
    assert_eq!(cmd, "CHECK <article@example.com>\r\n");

    // 3. Server responds with 238/431/438
    let response = NntpResponse {
        code: codes::CHECK_SEND,
        message: "<article@example.com> Send".to_string(),
        lines: vec![],
    };
    assert!(response.is_success());
}

// Error Handling

#[test]
fn test_check_invalid_response_codes() {
    // Test that invalid response codes are handled properly

    // 500-series errors
    let response1 = NntpResponse {
        code: 500,
        message: "Command not recognized".to_string(),
        lines: vec![],
    };
    assert!(response1.is_error());

    let response2 = NntpResponse {
        code: 503,
        message: "Program error".to_string(),
        lines: vec![],
    };
    assert!(response2.is_error());
}

#[test]
fn test_check_response_without_message_id() {
    // Server MUST include message-id in response, but test handling if missing
    let response = NntpResponse {
        code: codes::CHECK_SEND,
        message: "Send article".to_string(), // Missing message-id
        lines: vec![],
    };

    // Should still be a valid response object
    assert_eq!(response.code, 238);
    assert!(response.is_success());
}

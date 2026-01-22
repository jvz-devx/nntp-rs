//! RFC 4644 Section 2.5 - TAKETHIS Command Tests
//!
//! These tests verify the TAKETHIS command implementation:
//! - Command format with article data
//! - Response code handling (239, 439)
//! - Message-ID format validation
//! - Article data inclusion
//! - Error handling
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc4644#section-2.5

use nntp_rs::{codes, commands, NntpResponse};
#[test]
fn test_takethis_format() {
    let message_id = "<article123@example.com>";
    let article_data = "From: test@example.com\r\nSubject: Test\r\n\r\nBody text\r\n.\r\n";
    let cmd = commands::takethis(message_id, article_data);

    assert!(cmd.starts_with("TAKETHIS <article123@example.com>\r\n"));
    assert!(cmd.contains(article_data));
}

#[test]
fn test_takethis_ends_with_article() {
    let message_id = "<test@host.com>";
    let article_data = "Headers\r\n\r\nBody\r\n.\r\n";
    let cmd = commands::takethis(message_id, article_data);

    // Command should be: TAKETHIS <msg-id>\r\n<article-data>
    assert!(cmd.starts_with("TAKETHIS "));
    assert!(cmd.contains(message_id));
    assert!(cmd.contains(article_data));
}

#[test]
fn test_takethis_uppercase() {
    let cmd = commands::takethis("<msg@id>", "article\r\n.\r\n");
    assert!(cmd.starts_with("TAKETHIS "));
}

#[test]
fn test_takethis_message_id_format() {
    let message_id = "<abc123@news.example.com>";
    let article_data = "Data\r\n.\r\n";
    let cmd = commands::takethis(message_id, article_data);

    assert!(cmd.contains(message_id));
    assert!(cmd.starts_with(format!("TAKETHIS {}\r\n", message_id).as_str()));
}

#[test]
fn test_takethis_various_message_ids() {
    let article_data = "From: test@example.com\r\n\r\nBody\r\n.\r\n";

    // Simple message-id
    let cmd1 = commands::takethis("<simple@host>", article_data);
    assert!(cmd1.starts_with("TAKETHIS <simple@host>\r\n"));

    // Complex message-id with numbers
    let cmd2 = commands::takethis("<1234567890.abcdef@news.server.com>", article_data);
    assert!(cmd2.starts_with("TAKETHIS <1234567890.abcdef@news.server.com>\r\n"));

    // Message-id with special characters
    let cmd3 = commands::takethis("<part1$part2@domain.org>", article_data);
    assert!(cmd3.starts_with("TAKETHIS <part1$part2@domain.org>\r\n"));
}

#[test]
fn test_takethis_long_message_id() {
    // Test with a very long message-id
    let long_id = format!("<{}.{}@{}>", "a".repeat(50), "b".repeat(50), "example.com");
    let article_data = "Data\r\n.\r\n";
    let cmd = commands::takethis(&long_id, article_data);

    assert!(cmd.starts_with("TAKETHIS <"));
    assert!(cmd.contains(&long_id));
}

#[test]
fn test_takethis_with_complete_article() {
    let message_id = "<full@example.com>";
    let article = "From: sender@example.com\r\n\
                   Subject: Test Article\r\n\
                   Newsgroups: test.group\r\n\
                   Date: Mon, 01 Jan 2024 12:00:00 +0000\r\n\
                   Message-ID: <full@example.com>\r\n\
                   \r\n\
                   This is the body of the article.\r\n\
                   It can have multiple lines.\r\n\
                   .\r\n";

    let cmd = commands::takethis(message_id, article);

    // Verify command structure
    assert!(cmd.starts_with("TAKETHIS <full@example.com>\r\n"));
    assert!(cmd.contains("From: sender@example.com"));
    assert!(cmd.contains("Subject: Test Article"));
    assert!(cmd.contains("This is the body"));
}
#[test]
fn test_takethis_received_239() {
    // 239 = Article received (RFC 4644 Section 2.5)
    // Server successfully received and processed the article
    let response = NntpResponse {
        code: codes::TAKETHIS_RECEIVED,
        message: "<article123@example.com> Article received".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 239);
    assert!(response.is_success());
    assert!(!response.is_error());
}

#[test]
fn test_takethis_rejected_439() {
    // 439 = Article rejected (RFC 4644 Section 2.5)
    // Server rejected the article, do not retry
    let response = NntpResponse {
        code: codes::TAKETHIS_REJECTED,
        message: "<article123@example.com> Article rejected".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 439);
    assert!(response.is_error());
    assert!(!response.is_success());
}

#[test]
fn test_takethis_response_includes_message_id() {
    // Response should include the message-id for matching in pipelined scenarios
    let response1 = NntpResponse {
        code: codes::TAKETHIS_RECEIVED,
        message: "<msg1@example.com> Article received".to_string(),
        lines: vec![],
    };

    let response2 = NntpResponse {
        code: codes::TAKETHIS_REJECTED,
        message: "<msg2@example.com> Article rejected".to_string(),
        lines: vec![],
    };

    assert!(response1.message.contains("<msg1@example.com>"));
    assert!(response2.message.contains("<msg2@example.com>"));
}

// RFC 4644 Section 2.5 Examples

#[test]
fn test_takethis_rfc_example_received() {
    // From RFC 4644 Section 2.5.2: Article received
    // C: TAKETHIS <123456@example.com>
    // C: [article data]
    // S: 239 <123456@example.com> Article received ok

    let message_id = "<123456@example.com>";
    let article_data = "From: test@example.com\r\n\r\nBody\r\n.\r\n";
    let cmd = commands::takethis(message_id, article_data);

    assert!(cmd.starts_with("TAKETHIS <123456@example.com>\r\n"));

    let response = NntpResponse {
        code: 239,
        message: "<123456@example.com> Article received ok".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, codes::TAKETHIS_RECEIVED);
    assert!(response.is_success());
}

#[test]
fn test_takethis_rfc_example_rejected() {
    // From RFC 4644 Section 2.5.2: Article rejected
    // C: TAKETHIS <789012@example.com>
    // C: [article data]
    // S: 439 <789012@example.com> Article rejected

    let message_id = "<789012@example.com>";
    let article_data = "From: test@example.com\r\n\r\nBody\r\n.\r\n";
    let cmd = commands::takethis(message_id, article_data);

    assert!(cmd.starts_with("TAKETHIS <789012@example.com>\r\n"));

    let response = NntpResponse {
        code: 439,
        message: "<789012@example.com> Article rejected".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, codes::TAKETHIS_REJECTED);
    assert!(response.is_error());
}


#[test]
fn test_takethis_empty_article() {
    // Even with empty article data, command should be properly formatted
    let cmd = commands::takethis("<test@example.com>", "");
    assert!(cmd.starts_with("TAKETHIS <test@example.com>\r\n"));
}

#[test]
fn test_takethis_message_id_no_brackets() {
    // Message-id without angle brackets (non-standard but should work)
    let cmd = commands::takethis("test@example.com", "Data\r\n.\r\n");
    assert!(cmd.contains("test@example.com"));
}

#[test]
fn test_takethis_message_id_with_whitespace() {
    // Message-id with surrounding whitespace
    let cmd = commands::takethis(" <test@example.com> ", "Data\r\n.\r\n");
    assert!(cmd.contains(" <test@example.com> "));
}

#[test]
fn test_takethis_various_response_formats() {
    // Different response message formats servers might use
    let responses = vec![
        NntpResponse {
            code: 239,
            message: "<abc@example.com> Received".to_string(),
            lines: vec![],
        },
        NntpResponse {
            code: 239,
            message: "<abc@example.com>".to_string(),
            lines: vec![],
        },
        NntpResponse {
            code: 439,
            message: "<xyz@example.com> Duplicate".to_string(),
            lines: vec![],
        },
    ];

    assert!(responses[0].is_success());
    assert!(responses[1].is_success());
    assert!(responses[2].is_error());
}

// Real-World Scenarios

#[test]
fn test_takethis_streaming_workflow() {
    // Typical streaming workflow: MODE STREAM, then TAKETHIS
    let message_id = "<article@example.com>";
    let article_data = "From: user@example.com\r\n\
                       Subject: Binary post\r\n\
                       Newsgroups: alt.binaries.test\r\n\
                       \r\n\
                       Binary data here\r\n\
                       .\r\n";

    let cmd = commands::takethis(message_id, article_data);

    // Verify command is ready to send
    assert!(cmd.starts_with("TAKETHIS "));
    assert!(cmd.contains(message_id));
    assert!(cmd.contains("Binary data here"));
}

#[test]
fn test_takethis_pipelined_responses() {
    // In pipelined mode, responses may arrive out of order
    // Each response must include the message-id for matching
    let responses = vec![
        NntpResponse {
            code: 239,
            message: "<msg2@example.com> Article received".to_string(),
            lines: vec![],
        },
        NntpResponse {
            code: 239,
            message: "<msg1@example.com> Article received".to_string(),
            lines: vec![],
        },
        NntpResponse {
            code: 439,
            message: "<msg3@example.com> Article rejected".to_string(),
            lines: vec![],
        },
    ];

    // Verify all responses have message-ids
    assert!(responses[0].message.contains("<msg2@example.com>"));
    assert!(responses[1].message.contains("<msg1@example.com>"));
    assert!(responses[2].message.contains("<msg3@example.com>"));
}

#[test]
fn test_takethis_binary_newsgroup() {
    // Binary newsgroups are common use case for streaming
    let message_id = "<part1.yenc@news.example.com>";
    let article_data = "From: poster@example.com\r\n\
                       Subject: [1/10] - \"file.bin\" yEnc (1/100)\r\n\
                       Newsgroups: alt.binaries.test\r\n\
                       \r\n\
                       =ybegin line=128 size=1000 name=file.bin\r\n\
                       Binary yEnc data here\r\n\
                       =yend size=1000\r\n\
                       .\r\n";

    let cmd = commands::takethis(message_id, article_data);

    assert!(cmd.contains("yEnc"));
    assert!(cmd.contains("alt.binaries.test"));
}

#[test]
fn test_takethis_without_prior_check() {
    // Client can send TAKETHIS without preceding CHECK
    // This is allowed by RFC 4644 Section 2.5
    let message_id = "<direct@example.com>";
    let article_data = "Headers\r\n\r\nBody\r\n.\r\n";
    let cmd = commands::takethis(message_id, article_data);

    // Should work fine - no dependency on CHECK
    assert!(cmd.starts_with("TAKETHIS "));
    assert!(cmd.contains(message_id));
}

#[test]
fn test_takethis_mode_stream_requirement() {
    // TAKETHIS requires MODE STREAM first
    // Server should return 480 if not in streaming mode
    let response = NntpResponse {
        code: codes::AUTH_REQUIRED,
        message: "Streaming mode not enabled".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 480);
    assert!(response.is_error());
}

// Error Handling

#[test]
fn test_takethis_invalid_response_code() {
    // Test handling of unexpected response codes
    let response = NntpResponse {
        code: 500,
        message: "Internal error".to_string(),
        lines: vec![],
    };

    assert!(response.is_error());
    assert_ne!(response.code, codes::TAKETHIS_RECEIVED);
    assert_ne!(response.code, codes::TAKETHIS_REJECTED);
}

#[test]
fn test_takethis_response_without_message_id() {
    // Non-compliant server might not include message-id in response
    // Client should still handle it (even if not ideal)
    let response = NntpResponse {
        code: 239,
        message: "Article received".to_string(),
        lines: vec![],
    };

    assert!(response.is_success());
    // Note: In real pipelined scenario, this would be problematic
    // but the response is still valid NNTP
}

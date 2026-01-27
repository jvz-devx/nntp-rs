//! RFC 3977 Section 6.3.2 - IHAVE Command Tests
//!
//! Tests for the IHAVE command which is used for server-to-server article transfer.
//! IHAVE uses a two-phase protocol where the server first indicates whether it wants
//! the article, then accepts or rejects the transfer.
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc3977#section-6.3.2

use nntp_rs::{NntpResponse, codes, commands};
#[test]
fn test_ihave_command_format() {
    let cmd = commands::ihave("<test@example.com>");
    assert_eq!(cmd, "IHAVE <test@example.com>\r\n");
}

#[test]
fn test_ihave_command_ends_with_crlf() {
    let cmd = commands::ihave("<abc@server.com>");
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_ihave_command_uppercase() {
    let cmd = commands::ihave("<msg@host.org>");
    assert!(cmd.starts_with("IHAVE "));
}

#[test]
fn test_ihave_with_complex_message_id() {
    let cmd = commands::ihave("<part1of50.abc123def@news.example.com>");
    assert_eq!(cmd, "IHAVE <part1of50.abc123def@news.example.com>\r\n");
}

#[test]
fn test_ihave_with_binary_message_id() {
    // Binary posts often have yEnc-style message-ids
    let cmd = commands::ihave("<file.rar.001.yenc@usenet.server>");
    assert_eq!(cmd, "IHAVE <file.rar.001.yenc@usenet.server>\r\n");
}
#[test]
fn test_ihave_response_335_send_article() {
    let response = NntpResponse {
        code: codes::SEND_ARTICLE_TRANSFER,
        message: "Send article to be transferred".to_string(),
        lines: vec![],
    };
    assert_eq!(response.code, 335);
    assert!(response.is_continuation());
}

#[test]
fn test_ihave_response_435_not_wanted() {
    let response = NntpResponse {
        code: codes::ARTICLE_NOT_WANTED,
        message: "Article not wanted".to_string(),
        lines: vec![],
    };
    assert_eq!(response.code, 435);
    assert!(response.is_error());
}

#[test]
fn test_ihave_response_436_not_possible() {
    let response = NntpResponse {
        code: codes::TRANSFER_NOT_POSSIBLE,
        message: "Transfer not possible; try again later".to_string(),
        lines: vec![],
    };
    assert_eq!(response.code, 436);
    assert!(response.is_error());
}
#[test]
fn test_ihave_response_235_transferred() {
    let response = NntpResponse {
        code: codes::ARTICLE_TRANSFERRED,
        message: "Article transferred OK".to_string(),
        lines: vec![],
    };
    assert_eq!(response.code, 235);
    assert!(response.is_success());
}

#[test]
fn test_ihave_response_437_rejected() {
    let response = NntpResponse {
        code: codes::TRANSFER_REJECTED,
        message: "Transfer rejected; do not retry".to_string(),
        lines: vec![],
    };
    assert_eq!(response.code, 437);
    assert!(response.is_error());
}

#[test]
fn test_ihave_response_436_failed() {
    // Code 436 can appear in both phases
    let response = NntpResponse {
        code: codes::TRANSFER_NOT_POSSIBLE,
        message: "Transfer failed; try again later".to_string(),
        lines: vec![],
    };
    assert_eq!(response.code, 436);
    assert!(response.is_error());
}
#[test]
fn test_ihave_protocol_error() {
    let response = NntpResponse {
        code: 500,
        message: "Command not recognized".to_string(),
        lines: vec![],
    };
    assert!(response.is_error());
    assert_eq!(response.code, 500);
}

#[test]
fn test_ihave_command_unavailable() {
    let response = NntpResponse {
        code: codes::ACCESS_DENIED,
        message: "IHAVE not available".to_string(),
        lines: vec![],
    };
    assert_eq!(response.code, 502);
    assert!(response.is_error());
}

#[test]
fn test_ihave_wrong_continuation_code() {
    // Test that we properly distinguish 335 from 340
    let response = NntpResponse {
        code: codes::SEND_ARTICLE, // 340 is for POST, not IHAVE
        message: "Send article".to_string(),
        lines: vec![],
    };
    assert_eq!(response.code, 340);
    assert_ne!(response.code, codes::SEND_ARTICLE_TRANSFER); // Should be 335
}

// RFC 3977 Section 6.3.2 Examples

#[test]
fn test_ihave_rfc_example_success() {
    // Example from RFC 3977 Section 6.3.2
    // [C] IHAVE <i.am.a.message.id@example.com>
    // [S] 335 Send article to be transferred
    // [C] (article text follows)
    // [S] 235 Article transferred OK

    let cmd = commands::ihave("<i.am.a.message.id@example.com>");
    assert_eq!(cmd, "IHAVE <i.am.a.message.id@example.com>\r\n");

    let response1 = NntpResponse {
        code: 335,
        message: "Send article to be transferred".to_string(),
        lines: vec![],
    };
    assert!(response1.is_continuation());

    let response2 = NntpResponse {
        code: 235,
        message: "Article transferred OK".to_string(),
        lines: vec![],
    };
    assert!(response2.is_success());
}

#[test]
fn test_ihave_rfc_example_not_wanted() {
    // Example: server already has the article
    // [C] IHAVE <12345@example.com>
    // [S] 435 Article not wanted

    let cmd = commands::ihave("<12345@example.com>");
    assert_eq!(cmd, "IHAVE <12345@example.com>\r\n");

    let response = NntpResponse {
        code: 435,
        message: "Article not wanted".to_string(),
        lines: vec![],
    };
    assert_eq!(response.code, codes::ARTICLE_NOT_WANTED);
}

#[test]
fn test_ihave_rfc_example_transfer_failed() {
    // Example: transfer starts but fails
    // [C] IHAVE <article@server.com>
    // [S] 335 Send article
    // [C] (article text)
    // [S] 436 Transfer failed; try again later

    let response1 = NntpResponse {
        code: 335,
        message: "Send article".to_string(),
        lines: vec![],
    };
    assert!(response1.is_continuation());

    let response2 = NntpResponse {
        code: 436,
        message: "Transfer failed; try again later".to_string(),
        lines: vec![],
    };
    assert_eq!(response2.code, codes::TRANSFER_NOT_POSSIBLE);
}

#[test]
fn test_ihave_empty_message_id() {
    // While invalid, the command builder should still work
    let cmd = commands::ihave("");
    assert_eq!(cmd, "IHAVE \r\n");
}

#[test]
fn test_ihave_message_id_without_brackets() {
    // Some systems might use message-ids without angle brackets
    let cmd = commands::ihave("msgid@example.com");
    assert_eq!(cmd, "IHAVE msgid@example.com\r\n");
}

#[test]
fn test_ihave_message_id_with_spaces() {
    // Invalid message-id but test the command builder
    let cmd = commands::ihave("<msg id@example.com>");
    assert_eq!(cmd, "IHAVE <msg id@example.com>\r\n");
}

#[test]
fn test_ihave_very_long_message_id() {
    let long_id = format!("<{:0<100}@example.com>", "x");
    let cmd = commands::ihave(&long_id);
    assert!(cmd.starts_with("IHAVE <"));
    assert!(cmd.ends_with("@example.com>\r\n"));
}

// Real-World Scenarios

#[test]
fn test_ihave_binary_post_scenario() {
    // Binary posts use IHAVE for server-to-server propagation
    let message_id = "<part01.rar.yenc.12345@news.server.com>";
    let cmd = commands::ihave(message_id);
    assert_eq!(cmd, "IHAVE <part01.rar.yenc.12345@news.server.com>\r\n");
}

#[test]
fn test_ihave_retry_logic() {
    // When server returns 436, client should be able to retry
    let response_436 = NntpResponse {
        code: codes::TRANSFER_NOT_POSSIBLE,
        message: "Server busy; try again".to_string(),
        lines: vec![],
    };
    assert_eq!(response_436.code, 436);
    // This code means: temporary failure, retry is appropriate
}

#[test]
fn test_ihave_no_retry_logic() {
    // When server returns 437, client should NOT retry
    let response_437 = NntpResponse {
        code: codes::TRANSFER_REJECTED,
        message: "Article violates policy".to_string(),
        lines: vec![],
    };
    assert_eq!(response_437.code, 437);
    // This code means: permanent rejection, do not retry
}

#[test]
fn test_ihave_duplicate_detection() {
    // Server uses 435 to indicate it already has the article
    let response = NntpResponse {
        code: codes::ARTICLE_NOT_WANTED,
        message: "Duplicate message-id".to_string(),
        lines: vec![],
    };
    assert_eq!(response.code, 435);
    // This prevents duplicate article propagation
}

#[test]
fn test_ihave_server_to_server_workflow() {
    // Complete workflow: offer -> accept -> transfer -> success
    let cmd = commands::ihave("<news.article@origin.com>");
    assert!(cmd.starts_with("IHAVE "));

    // Phase 1: Server accepts
    let phase1 = NntpResponse {
        code: 335,
        message: "Send it".to_string(),
        lines: vec![],
    };
    assert_eq!(phase1.code, codes::SEND_ARTICLE_TRANSFER);

    // Phase 2: Transfer succeeds
    let phase2 = NntpResponse {
        code: 235,
        message: "Got it".to_string(),
        lines: vec![],
    };
    assert_eq!(phase2.code, codes::ARTICLE_TRANSFERRED);
}

#[test]
fn test_ihave_vs_post_distinction() {
    // IHAVE uses 335, POST uses 340
    assert_eq!(codes::SEND_ARTICLE_TRANSFER, 335);
    assert_eq!(codes::SEND_ARTICLE, 340);
    assert_ne!(codes::SEND_ARTICLE_TRANSFER, codes::SEND_ARTICLE);

    // IHAVE uses 235, POST uses 240
    assert_eq!(codes::ARTICLE_TRANSFERRED, 235);
    assert_eq!(codes::ARTICLE_POSTED, 240);
    assert_ne!(codes::ARTICLE_TRANSFERRED, codes::ARTICLE_POSTED);
}

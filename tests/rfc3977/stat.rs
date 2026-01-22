//! RFC 3977 Section 6.2.4 - STAT Command Tests
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc3977#section-6.2.4
//!
//! These tests verify the STAT command which checks article existence and
//! retrieves metadata without downloading the article content.

use nntp_rs::{codes, commands, NntpResponse};
#[test]
fn test_stat_command_format_with_number() {
    let cmd = commands::stat("12345");
    assert_eq!(cmd, "STAT 12345\r\n");
}

#[test]
fn test_stat_command_format_with_message_id() {
    let cmd = commands::stat("<abc123@example.com>");
    assert_eq!(cmd, "STAT <abc123@example.com>\r\n");
}

#[test]
fn test_stat_command_ends_with_crlf() {
    let cmd = commands::stat("123");
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_stat_command_uppercase() {
    let cmd = commands::stat("42");
    assert!(cmd.starts_with("STAT"));
}

#[test]
fn test_stat_command_with_large_article_number() {
    let cmd = commands::stat("999999999");
    assert_eq!(cmd, "STAT 999999999\r\n");
}
#[test]
fn test_parse_stat_response_with_article_number() {
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "12345 <abc@example.com>".to_string(),
        lines: vec![],
    };

    let result = commands::parse_stat_response(&response).unwrap();
    assert_eq!(result.0, 12345);
    assert_eq!(result.1, "<abc@example.com>");
}

#[test]
fn test_parse_stat_response_with_zero_article_number() {
    // When queried by message-id, article number may be 0
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "0 <xyz@test.com>".to_string(),
        lines: vec![],
    };

    let result = commands::parse_stat_response(&response).unwrap();
    assert_eq!(result.0, 0);
    assert_eq!(result.1, "<xyz@test.com>");
}

#[test]
fn test_parse_stat_response_rfc_example() {
    // RFC 3977 Section 6.2.4 example: "223 1000234 <message@example.com>"
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "1000234 <message@example.com>".to_string(),
        lines: vec![],
    };

    let result = commands::parse_stat_response(&response).unwrap();
    assert_eq!(result.0, 1000234);
    assert_eq!(result.1, "<message@example.com>");
}

#[test]
fn test_parse_stat_response_complex_message_id() {
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "42 <part1of10.yEnc.abc123@usenet.example.com>".to_string(),
        lines: vec![],
    };

    let result = commands::parse_stat_response(&response).unwrap();
    assert_eq!(result.0, 42);
    assert_eq!(result.1, "<part1of10.yEnc.abc123@usenet.example.com>");
}

#[test]
fn test_parse_stat_response_very_large_article_number() {
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "18446744073709551615 <test@example.com>".to_string(),
        lines: vec![],
    };

    let result = commands::parse_stat_response(&response).unwrap();
    assert_eq!(result.0, u64::MAX);
    assert_eq!(result.1, "<test@example.com>");
}
#[test]
fn test_parse_stat_response_protocol_error() {
    let response = NntpResponse {
        code: 500,
        message: "Command not recognized".to_string(),
        lines: vec![],
    };

    let result = commands::parse_stat_response(&response);
    assert!(result.is_err());
}

#[test]
fn test_parse_stat_response_no_such_article() {
    let response = NntpResponse {
        code: codes::NO_SUCH_ARTICLE_ID,
        message: "No such article".to_string(),
        lines: vec![],
    };

    let result = commands::parse_stat_response(&response);
    assert!(result.is_err());
}

#[test]
fn test_parse_stat_response_no_group_selected() {
    let response = NntpResponse {
        code: codes::NO_GROUP_SELECTED,
        message: "No newsgroup selected".to_string(),
        lines: vec![],
    };

    let result = commands::parse_stat_response(&response);
    assert!(result.is_err());
}

#[test]
fn test_parse_stat_response_invalid_article_number() {
    let response = NntpResponse {
        code: codes::NO_SUCH_ARTICLE_NUMBER,
        message: "No article with that number".to_string(),
        lines: vec![],
    };

    let result = commands::parse_stat_response(&response);
    assert!(result.is_err());
}

#[test]
fn test_parse_stat_response_wrong_success_code() {
    let response = NntpResponse {
        code: 200,
        message: "12345 <test@example.com>".to_string(),
        lines: vec![],
    };

    // parse_stat_response checks is_success(), which should pass for 2xx codes
    // But the message should still parse correctly
    let result = commands::parse_stat_response(&response).unwrap();
    assert_eq!(result.0, 12345);
    assert_eq!(result.1, "<test@example.com>");
}


#[test]
fn test_parse_stat_response_malformed_missing_message_id() {
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "12345".to_string(),
        lines: vec![],
    };

    // Should error - need at least article number AND message-id
    let result = commands::parse_stat_response(&response);
    assert!(result.is_err());
}

#[test]
fn test_parse_stat_response_malformed_invalid_article_number() {
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "notanumber <test@example.com>".to_string(),
        lines: vec![],
    };

    let result = commands::parse_stat_response(&response);
    assert!(result.is_err());
}

#[test]
fn test_parse_stat_response_extra_whitespace() {
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "  12345   <test@example.com>  ".to_string(),
        lines: vec![],
    };

    let result = commands::parse_stat_response(&response).unwrap();
    assert_eq!(result.0, 12345);
    assert_eq!(result.1, "<test@example.com>");
}

#[test]
fn test_parse_stat_response_message_id_with_spaces() {
    // Though not RFC-compliant, handle gracefully
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "100 <test message@example.com>".to_string(),
        lines: vec![],
    };

    let result = commands::parse_stat_response(&response).unwrap();
    assert_eq!(result.0, 100);
    // Message-ID parts are joined back together
    assert_eq!(result.1, "<test message@example.com>");
}

// Real-world Scenarios

#[test]
fn test_stat_command_typical_binary_post() {
    let cmd = commands::stat("<yEnc-part001of100-abc123@usenet.example>");
    assert_eq!(cmd, "STAT <yEnc-part001of100-abc123@usenet.example>\r\n");
}

#[test]
fn test_parse_stat_response_typical_binary_server() {
    // Typical response from a binary Usenet server
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "3456789 <20250121.part01.yEnc@provider.example.com>".to_string(),
        lines: vec![],
    };

    let result = commands::parse_stat_response(&response).unwrap();
    assert_eq!(result.0, 3456789);
    assert_eq!(result.1, "<20250121.part01.yEnc@provider.example.com>");
}

#[test]
fn test_stat_command_check_sequence() {
    // Testing multiple article numbers in sequence
    let cmd1 = commands::stat("100");
    let cmd2 = commands::stat("101");
    let cmd3 = commands::stat("102");

    assert_eq!(cmd1, "STAT 100\r\n");
    assert_eq!(cmd2, "STAT 101\r\n");
    assert_eq!(cmd3, "STAT 102\r\n");
}

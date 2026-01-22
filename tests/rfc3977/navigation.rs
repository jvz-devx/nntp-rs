//! RFC 3977 Section 6.1.3-6.1.4 - Navigation Command Tests (LAST/NEXT)
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc3977#section-6.1
//!
//! These tests verify the NEXT and LAST commands which navigate between articles
//! in the currently selected newsgroup.
//!
//! - NEXT (§6.1.4): Move to the next article in the selected newsgroup
//! - LAST (§6.1.3): Move to the previous article in the selected newsgroup
//!
//! Both commands return the same response format: "223 n message-id"

use nntp_rs::{codes, commands, NntpResponse};
#[test]
fn test_next_command_format() {
    let cmd = commands::next();
    assert_eq!(cmd, "NEXT\r\n");
}

#[test]
fn test_next_command_ends_with_crlf() {
    let cmd = commands::next();
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_next_command_uppercase() {
    let cmd = commands::next();
    assert!(cmd.starts_with("NEXT"));
}

#[test]
fn test_next_command_no_arguments() {
    // NEXT takes no arguments - verify it's just "NEXT\r\n"
    let cmd = commands::next();
    assert_eq!(cmd, "NEXT\r\n");
    assert_eq!(cmd.len(), 6); // "NEXT\r\n" is 6 bytes
}
#[test]
fn test_parse_next_response_basic() {
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "12346 <next@example.com>".to_string(),
        lines: vec![],
    };

    let result = commands::parse_next_response(&response).unwrap();
    assert_eq!(result.0, 12346);
    assert_eq!(result.1, "<next@example.com>");
}

#[test]
fn test_parse_next_response_rfc_example() {
    // RFC 3977 Section 6.1.4: "223 n message-id"
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "1000235 <article.next@example.org>".to_string(),
        lines: vec![],
    };

    let result = commands::parse_next_response(&response).unwrap();
    assert_eq!(result.0, 1000235);
    assert_eq!(result.1, "<article.next@example.org>");
}

#[test]
fn test_parse_next_response_complex_message_id() {
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "500 <part2of10.yEnc.xyz789@binary.example.net>".to_string(),
        lines: vec![],
    };

    let result = commands::parse_next_response(&response).unwrap();
    assert_eq!(result.0, 500);
    assert_eq!(result.1, "<part2of10.yEnc.xyz789@binary.example.net>");
}

#[test]
fn test_parse_next_response_very_large_article_number() {
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "18446744073709551614 <huge@example.com>".to_string(),
        lines: vec![],
    };

    let result = commands::parse_next_response(&response).unwrap();
    assert_eq!(result.0, 18446744073709551614u64);
    assert_eq!(result.1, "<huge@example.com>");
}

#[test]
fn test_parse_next_response_sequential_article() {
    // Moving from article 100 to 101
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "101 <sequential@example.com>".to_string(),
        lines: vec![],
    };

    let result = commands::parse_next_response(&response).unwrap();
    assert_eq!(result.0, 101);
    assert_eq!(result.1, "<sequential@example.com>");
}

#[test]
fn test_parse_next_response_sparse_articles() {
    // In sparse groups, next article might be far ahead (e.g., 1000 -> 1500)
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "1500 <sparse@example.com>".to_string(),
        lines: vec![],
    };

    let result = commands::parse_next_response(&response).unwrap();
    assert_eq!(result.0, 1500);
    assert_eq!(result.1, "<sparse@example.com>");
}
#[test]
fn test_parse_next_response_protocol_error() {
    let response = NntpResponse {
        code: 500,
        message: "Internal server error".to_string(),
        lines: vec![],
    };

    assert!(commands::parse_next_response(&response).is_err());
}

#[test]
fn test_parse_next_response_no_group_selected() {
    let response = NntpResponse {
        code: codes::NO_GROUP_SELECTED,
        message: "No newsgroup selected".to_string(),
        lines: vec![],
    };

    assert!(commands::parse_next_response(&response).is_err());
}

#[test]
fn test_parse_next_response_no_current_article() {
    let response = NntpResponse {
        code: codes::NO_CURRENT_ARTICLE,
        message: "No current article selected".to_string(),
        lines: vec![],
    };

    assert!(commands::parse_next_response(&response).is_err());
}

#[test]
fn test_parse_next_response_no_next_article() {
    let response = NntpResponse {
        code: codes::NO_NEXT_ARTICLE,
        message: "No next article in this group".to_string(),
        lines: vec![],
    };

    assert!(commands::parse_next_response(&response).is_err());
}

#[test]
fn test_parse_next_response_wrong_success_code() {
    // Success code but not 223 (ARTICLE_STAT)
    let response = NntpResponse {
        code: 200,
        message: "12345 <test@example.com>".to_string(),
        lines: vec![],
    };

    let result = commands::parse_next_response(&response);
    assert!(result.is_ok()); // Will still parse if it's a success code
}
#[test]
fn test_parse_next_response_malformed_missing_message_id() {
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "12345".to_string(),
        lines: vec![],
    };

    assert!(commands::parse_next_response(&response).is_err());
}

#[test]
fn test_parse_next_response_malformed_invalid_article_number() {
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "abc <test@example.com>".to_string(),
        lines: vec![],
    };

    assert!(commands::parse_next_response(&response).is_err());
}

#[test]
fn test_parse_next_response_extra_whitespace() {
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "  12345   <test@example.com>  ".to_string(),
        lines: vec![],
    };

    let result = commands::parse_next_response(&response).unwrap();
    assert_eq!(result.0, 12345);
    // Whitespace should be trimmed in split_whitespace()
}

#[test]
fn test_parse_next_response_message_id_with_spaces() {
    // Technically invalid per RFC, but handle gracefully
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "12345 <test message@example.com>".to_string(),
        lines: vec![],
    };

    let result = commands::parse_next_response(&response).unwrap();
    assert_eq!(result.0, 12345);
    // Should join the parts after the article number
    assert!(result.1.contains("test"));
    assert!(result.1.contains("message@example.com"));
}

// Real-World Scenarios

#[test]
fn test_next_response_binary_newsgroup_navigation() {
    // Binary newsgroups often have large article numbers
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "9876543210 <binary.part.001of100@usenet.provider.net>".to_string(),
        lines: vec![],
    };

    let result = commands::parse_next_response(&response).unwrap();
    assert_eq!(result.0, 9876543210);
    assert_eq!(result.1, "<binary.part.001of100@usenet.provider.net>");
}

#[test]
fn test_next_response_typical_server_response() {
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "42001 <20250121120000.12345@news.example.com>".to_string(),
        lines: vec![],
    };

    let result = commands::parse_next_response(&response).unwrap();
    assert_eq!(result.0, 42001);
    assert_eq!(result.1, "<20250121120000.12345@news.example.com>");
}

#[test]
fn test_next_sequential_navigation() {
    // Simulating multiple NEXT commands in sequence
    let responses = vec![
        ("100 <msg100@example.com>", 100),
        ("101 <msg101@example.com>", 101),
        ("102 <msg102@example.com>", 102),
    ];

    for (message, expected_num) in responses {
        let response = NntpResponse {
            code: codes::ARTICLE_STAT,
            message: message.to_string(),
            lines: vec![],
        };

        let result = commands::parse_next_response(&response).unwrap();
        assert_eq!(result.0, expected_num);
    }
}
#[test]
fn test_last_command_format() {
    let cmd = commands::last();
    assert_eq!(cmd, "LAST\r\n");
}

#[test]
fn test_last_command_ends_with_crlf() {
    let cmd = commands::last();
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_last_command_uppercase() {
    let cmd = commands::last();
    assert!(cmd.starts_with("LAST"));
}

#[test]
fn test_last_command_no_arguments() {
    // LAST takes no arguments - verify it's just "LAST\r\n"
    let cmd = commands::last();
    assert_eq!(cmd, "LAST\r\n");
    assert_eq!(cmd.len(), 6); // "LAST\r\n" is 6 bytes
}
#[test]
fn test_parse_last_response_basic() {
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "12344 <prev@example.com>".to_string(),
        lines: vec![],
    };

    let result = commands::parse_last_response(&response).unwrap();
    assert_eq!(result.0, 12344);
    assert_eq!(result.1, "<prev@example.com>");
}

#[test]
fn test_parse_last_response_rfc_example() {
    // RFC 3977 Section 6.1.3: "223 n message-id"
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "1000234 <article.prev@example.org>".to_string(),
        lines: vec![],
    };

    let result = commands::parse_last_response(&response).unwrap();
    assert_eq!(result.0, 1000234);
    assert_eq!(result.1, "<article.prev@example.org>");
}

#[test]
fn test_parse_last_response_complex_message_id() {
    // Binary posts often have complex message IDs
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "999998 <part1of10.yEnc.6f3a2b1c@news.server.com>".to_string(),
        lines: vec![],
    };

    let result = commands::parse_last_response(&response).unwrap();
    assert_eq!(result.0, 999998);
    assert_eq!(result.1, "<part1of10.yEnc.6f3a2b1c@news.server.com>");
}

#[test]
fn test_parse_last_response_very_large_article_number() {
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: format!("{} <test@example.com>", u64::MAX - 1),
        lines: vec![],
    };

    let result = commands::parse_last_response(&response).unwrap();
    assert_eq!(result.0, u64::MAX - 1);
}

#[test]
fn test_parse_last_response_sequential_navigation() {
    // Going backwards: 101 -> 100
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "100 <msg100@example.com>".to_string(),
        lines: vec![],
    };

    let result = commands::parse_last_response(&response).unwrap();
    assert_eq!(result.0, 100);
    assert_eq!(result.1, "<msg100@example.com>");
}

#[test]
fn test_parse_last_response_sparse_navigation() {
    // Sparse article numbers: 1500 -> 1000 (previous available article)
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "1000 <sparse@example.com>".to_string(),
        lines: vec![],
    };

    let result = commands::parse_last_response(&response).unwrap();
    assert_eq!(result.0, 1000);
}
#[test]
fn test_parse_last_response_protocol_error() {
    let response = NntpResponse {
        code: 500,
        message: "Internal server error".to_string(),
        lines: vec![],
    };

    assert!(commands::parse_last_response(&response).is_err());
}

#[test]
fn test_parse_last_response_no_group_selected() {
    let response = NntpResponse {
        code: codes::NO_GROUP_SELECTED,
        message: "No newsgroup selected".to_string(),
        lines: vec![],
    };

    assert!(commands::parse_last_response(&response).is_err());
}

#[test]
fn test_parse_last_response_no_current_article() {
    let response = NntpResponse {
        code: codes::NO_CURRENT_ARTICLE,
        message: "No current article selected".to_string(),
        lines: vec![],
    };

    assert!(commands::parse_last_response(&response).is_err());
}

#[test]
fn test_parse_last_response_no_previous_article() {
    let response = NntpResponse {
        code: codes::NO_PREV_ARTICLE,
        message: "No previous article in this group".to_string(),
        lines: vec![],
    };

    assert!(commands::parse_last_response(&response).is_err());
}

#[test]
fn test_parse_last_response_wrong_success_code() {
    let response = NntpResponse {
        code: 220, // Wrong success code
        message: "12345 <test@example.com>".to_string(),
        lines: vec![],
    };

    // Should still work - is_success() checks 2xx range
    let result = commands::parse_last_response(&response).unwrap();
    assert_eq!(result.0, 12345);
}
#[test]
fn test_parse_last_response_malformed_missing_message_id() {
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "12345".to_string(),
        lines: vec![],
    };

    assert!(commands::parse_last_response(&response).is_err());
}

#[test]
fn test_parse_last_response_malformed_invalid_article_number() {
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "abc <test@example.com>".to_string(),
        lines: vec![],
    };

    assert!(commands::parse_last_response(&response).is_err());
}

#[test]
fn test_parse_last_response_extra_whitespace() {
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "  12343   <test@example.com>  ".to_string(),
        lines: vec![],
    };

    let result = commands::parse_last_response(&response).unwrap();
    assert_eq!(result.0, 12343);
    // Whitespace should be trimmed in split_whitespace()
}

#[test]
fn test_parse_last_response_message_id_with_spaces() {
    // Technically invalid per RFC, but handle gracefully
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "12343 <test message@example.com>".to_string(),
        lines: vec![],
    };

    let result = commands::parse_last_response(&response).unwrap();
    assert_eq!(result.0, 12343);
    // Should join the parts after the article number
    assert!(result.1.contains("test"));
    assert!(result.1.contains("message@example.com"));
}

// Real-World Scenarios

#[test]
fn test_last_response_binary_newsgroup_navigation() {
    // Binary newsgroups often have large article numbers
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "4567899 <yEnc-Part42of50.8a3f2e1d@binaries.example.com>".to_string(),
        lines: vec![],
    };

    let result = commands::parse_last_response(&response).unwrap();
    assert_eq!(result.0, 4567899);
    assert!(result.1.contains("yEnc"));
}

#[test]
fn test_last_response_typical_server_response() {
    let response = NntpResponse {
        code: codes::ARTICLE_STAT,
        message: "41999 <20250121115959.12344@news.example.com>".to_string(),
        lines: vec![],
    };

    let result = commands::parse_last_response(&response).unwrap();
    assert_eq!(result.0, 41999);
    assert_eq!(result.1, "<20250121115959.12344@news.example.com>");
}

#[test]
fn test_last_sequential_navigation() {
    // Simulating multiple LAST commands in sequence (going backwards)
    let responses = vec![
        ("102 <msg102@example.com>", 102),
        ("101 <msg101@example.com>", 101),
        ("100 <msg100@example.com>", 100),
    ];

    for (message, expected_num) in responses {
        let response = NntpResponse {
            code: codes::ARTICLE_STAT,
            message: message.to_string(),
            lines: vec![],
        };

        let result = commands::parse_last_response(&response).unwrap();
        assert_eq!(result.0, expected_num);
    }
}

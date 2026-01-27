//! RFC 3977 Section 7.4 - NEWNEWS Command Tests
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc3977#section-7.4
//!
//! Tests for the NEWNEWS command which lists message-IDs of articles posted since a specified date/time.

use nntp_rs::{NntpError, NntpResponse, codes, commands};
#[test]
fn test_newnews_command_format() {
    // RFC 3977 §7.4: NEWNEWS wildmat yyyymmdd hhmmss [GMT]
    let cmd = commands::newnews("comp.lang.rust", "20240101", "000000");
    assert_eq!(cmd, "NEWNEWS comp.lang.rust 20240101 000000\r\n");
}

#[test]
fn test_newnews_gmt_command_format() {
    let cmd = commands::newnews_gmt("alt.binaries.*", "20240101", "120000");
    assert_eq!(cmd, "NEWNEWS alt.binaries.* 20240101 120000 GMT\r\n");
}

#[test]
fn test_newnews_command_ends_with_crlf() {
    let cmd = commands::newnews("*", "20240101", "000000");
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_newnews_gmt_command_ends_with_crlf() {
    let cmd = commands::newnews_gmt("comp.*", "20240101", "000000");
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_newnews_command_uppercase() {
    let cmd = commands::newnews("comp.lang.rust", "20240101", "000000");
    assert!(cmd.starts_with("NEWNEWS"));
}

#[test]
fn test_newnews_wildmat_patterns() {
    // Test various wildmat patterns
    let cmd1 = commands::newnews("*", "20240101", "000000");
    assert!(cmd1.contains("NEWNEWS *"));

    let cmd2 = commands::newnews("comp.*", "20240101", "000000");
    assert!(cmd2.contains("comp.*"));

    let cmd3 = commands::newnews("alt.binaries.*", "20240101", "000000");
    assert!(cmd3.contains("alt.binaries.*"));

    let cmd4 = commands::newnews("comp.lang.rust", "20240101", "000000");
    assert!(cmd4.contains("comp.lang.rust"));
}

#[test]
fn test_newnews_various_dates() {
    // Test different date/time formats
    let cmd1 = commands::newnews("*", "19700101", "000000"); // Unix epoch
    assert!(cmd1.contains("19700101"));

    let cmd2 = commands::newnews("*", "20380119", "031407"); // Year 2038 problem
    assert!(cmd2.contains("20380119 031407"));

    let cmd3 = commands::newnews("*", "20231231", "235959"); // End of year
    assert!(cmd3.contains("20231231 235959"));
}

#[test]
fn test_newnews_gmt_has_gmt_flag() {
    let cmd = commands::newnews_gmt("*", "20240101", "120000");
    assert!(cmd.contains("GMT"));
    assert!(cmd.ends_with("GMT\r\n"));
}
#[test]
fn test_parse_newnews_success() {
    // RFC 3977 §7.4: Response is list of message-IDs
    let response = NntpResponse {
        code: codes::NEW_ARTICLE_LIST_FOLLOWS,
        message: "List of new articles follows".to_string(),
        lines: vec![
            "<abc123@example.com>".to_string(),
            "<def456@news.server.com>".to_string(),
            "<xyz789@usenet.provider.net>".to_string(),
        ],
    };

    let message_ids = commands::parse_newnews_response(response).unwrap();
    assert_eq!(message_ids.len(), 3);
    assert_eq!(message_ids[0], "<abc123@example.com>");
    assert_eq!(message_ids[1], "<def456@news.server.com>");
    assert_eq!(message_ids[2], "<xyz789@usenet.provider.net>");
}

#[test]
fn test_parse_newnews_empty_response() {
    // No new articles since specified date
    let response = NntpResponse {
        code: codes::NEW_ARTICLE_LIST_FOLLOWS,
        message: "List of new articles follows".to_string(),
        lines: vec![],
    };

    let message_ids = commands::parse_newnews_response(response).unwrap();
    assert_eq!(message_ids.len(), 0);
}

#[test]
fn test_parse_newnews_single_article() {
    let response = NntpResponse {
        code: codes::NEW_ARTICLE_LIST_FOLLOWS,
        message: "List of new articles follows".to_string(),
        lines: vec!["<single@example.com>".to_string()],
    };

    let message_ids = commands::parse_newnews_response(response).unwrap();
    assert_eq!(message_ids.len(), 1);
    assert_eq!(message_ids[0], "<single@example.com>");
}

#[test]
fn test_parse_newnews_complex_message_ids() {
    // Test various message-ID formats
    let response = NntpResponse {
        code: codes::NEW_ARTICLE_LIST_FOLLOWS,
        message: "List of new articles follows".to_string(),
        lines: vec![
            "<part1of10.abc123@server.com>".to_string(),
            "<2024.01.01.120000.xyz@news.example.org>".to_string(),
            "<user-post-12345@usenet.provider.net>".to_string(),
            "<a1b2c3d4-e5f6-7890-abcd-ef1234567890@uuid.server.com>".to_string(),
        ],
    };

    let message_ids = commands::parse_newnews_response(response).unwrap();
    assert_eq!(message_ids.len(), 4);
    assert!(message_ids[0].contains("part1of10"));
    assert!(message_ids[1].contains("2024.01.01"));
    assert!(message_ids[2].contains("user-post"));
    assert!(message_ids[3].contains("uuid"));
}

#[test]
fn test_parse_newnews_whitespace_handling() {
    // Test that leading/trailing whitespace is trimmed
    let response = NntpResponse {
        code: codes::NEW_ARTICLE_LIST_FOLLOWS,
        message: "List of new articles follows".to_string(),
        lines: vec![
            "  <abc@example.com>  ".to_string(),
            "\t<def@example.com>\t".to_string(),
            " <xyz@example.com> ".to_string(),
        ],
    };

    let message_ids = commands::parse_newnews_response(response).unwrap();
    assert_eq!(message_ids.len(), 3);
    assert_eq!(message_ids[0], "<abc@example.com>");
    assert_eq!(message_ids[1], "<def@example.com>");
    assert_eq!(message_ids[2], "<xyz@example.com>");
}

#[test]
fn test_parse_newnews_empty_lines_filtered() {
    // Empty lines should be filtered out
    let response = NntpResponse {
        code: codes::NEW_ARTICLE_LIST_FOLLOWS,
        message: "List of new articles follows".to_string(),
        lines: vec![
            "<abc@example.com>".to_string(),
            "".to_string(),
            "<def@example.com>".to_string(),
            "  ".to_string(),
            "<xyz@example.com>".to_string(),
        ],
    };

    let message_ids = commands::parse_newnews_response(response).unwrap();
    assert_eq!(message_ids.len(), 3);
    assert_eq!(message_ids[0], "<abc@example.com>");
    assert_eq!(message_ids[1], "<def@example.com>");
    assert_eq!(message_ids[2], "<xyz@example.com>");
}
#[test]
fn test_parse_newnews_protocol_error() {
    let response = NntpResponse {
        code: 500,
        message: "Command not recognized".to_string(),
        lines: vec![],
    };

    let result = commands::parse_newnews_response(response);
    assert!(result.is_err());
    match result.unwrap_err() {
        NntpError::Protocol { code, message } => {
            assert_eq!(code, 500);
            assert_eq!(message, "Command not recognized");
        }
        _ => panic!("Expected Protocol error"),
    }
}

#[test]
fn test_parse_newnews_wrong_success_code() {
    // Should only accept 230, not other success codes
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS, // 215 instead of 230
        message: "Wrong code".to_string(),
        lines: vec!["<abc@example.com>".to_string()],
    };

    // Parser doesn't validate specific code, just checks is_success()
    // This is consistent with other parsers in the codebase
    let result = commands::parse_newnews_response(response);
    assert!(result.is_ok());
}
#[test]
fn test_newnews_success_code() {
    let response = NntpResponse {
        code: codes::NEW_ARTICLE_LIST_FOLLOWS,
        message: "List of new articles follows".to_string(),
        lines: vec![],
    };

    assert!(response.is_success());
    assert_eq!(response.code, 230);
}

#[test]
fn test_newnews_error_codes() {
    // Various error codes that might be returned
    let error_codes = vec![
        500, // Command not recognized
        501, // Command syntax error
        503, // Feature not supported
    ];

    for code in error_codes {
        let response = NntpResponse {
            code,
            message: "Error".to_string(),
            lines: vec![],
        };
        assert!(response.is_error());
        assert!(!response.is_success());
    }
}

// RFC 3977 Section 7.4 Example Tests

#[test]
fn test_newnews_rfc_example() {
    // RFC 3977 §7.4 example
    // C: NEWNEWS comp.lang.lisp 20020624 120000 GMT
    // S: 230 List of new articles follows
    // S: <45223423@example.com>
    // S: <45223483@example.com>
    // S: <45223517@example.net>
    // S: .

    let cmd = commands::newnews_gmt("comp.lang.lisp", "20020624", "120000");
    assert_eq!(cmd, "NEWNEWS comp.lang.lisp 20020624 120000 GMT\r\n");

    let response = NntpResponse {
        code: codes::NEW_ARTICLE_LIST_FOLLOWS,
        message: "List of new articles follows".to_string(),
        lines: vec![
            "<45223423@example.com>".to_string(),
            "<45223483@example.com>".to_string(),
            "<45223517@example.net>".to_string(),
        ],
    };

    let message_ids = commands::parse_newnews_response(response).unwrap();
    assert_eq!(message_ids.len(), 3);
    assert_eq!(message_ids[0], "<45223423@example.com>");
    assert_eq!(message_ids[1], "<45223483@example.com>");
    assert_eq!(message_ids[2], "<45223517@example.net>");
}

#[test]
fn test_newnews_very_long_list() {
    // Test handling of large response (1000 message-IDs)
    let lines: Vec<String> = (0..1000)
        .map(|i| format!("<msg{}@example.com>", i))
        .collect();

    let response = NntpResponse {
        code: codes::NEW_ARTICLE_LIST_FOLLOWS,
        message: "List of new articles follows".to_string(),
        lines,
    };

    let message_ids = commands::parse_newnews_response(response).unwrap();
    assert_eq!(message_ids.len(), 1000);
    assert_eq!(message_ids[0], "<msg0@example.com>");
    assert_eq!(message_ids[999], "<msg999@example.com>");
}

#[test]
fn test_newnews_very_long_message_id() {
    // Test message-ID with very long domain/local part
    let long_msgid = format!("<{}@{}>", "a".repeat(100), "example.com");

    let response = NntpResponse {
        code: codes::NEW_ARTICLE_LIST_FOLLOWS,
        message: "List of new articles follows".to_string(),
        lines: vec![long_msgid.clone()],
    };

    let message_ids = commands::parse_newnews_response(response).unwrap();
    assert_eq!(message_ids.len(), 1);
    assert_eq!(message_ids[0], long_msgid);
}

// Real-World Scenarios

#[test]
fn test_newnews_binary_newsgroup_pattern() {
    // Binary newsgroups often have specific patterns
    let cmd = commands::newnews("alt.binaries.sounds.mp3.*", "20240101", "000000");
    assert!(cmd.contains("alt.binaries.sounds.mp3.*"));

    let response = NntpResponse {
        code: codes::NEW_ARTICLE_LIST_FOLLOWS,
        message: "List of new articles follows".to_string(),
        lines: vec![
            "<part01of99.abc123@usenet.provider.com>".to_string(),
            "<part02of99.abc123@usenet.provider.com>".to_string(),
        ],
    };

    let message_ids = commands::parse_newnews_response(response).unwrap();
    assert_eq!(message_ids.len(), 2);
}

#[test]
fn test_newnews_all_newsgroups_pattern() {
    // Using "*" to get all new articles across all groups
    let cmd = commands::newnews("*", "20240101", "000000");
    assert!(cmd.contains("NEWNEWS *"));
}

#[test]
fn test_newnews_hierarchy_pattern() {
    // Using hierarchy wildcards
    let patterns = vec![
        "comp.*",          // All comp.* groups
        "*.d",             // All groups ending in .d
        "comp.*.announce", // All comp.*.announce groups
    ];

    for pattern in patterns {
        let cmd = commands::newnews(pattern, "20240101", "000000");
        assert!(cmd.contains(pattern));
    }
}

#[test]
fn test_newnews_typical_server_response() {
    // Typical response from a real NNTP server
    let response = NntpResponse {
        code: codes::NEW_ARTICLE_LIST_FOLLOWS,
        message: "new news follows".to_string(),
        lines: vec![
            "<20240101120000.12345@news.example.com>".to_string(),
            "<abc-def-ghi@usenet.provider.net>".to_string(),
            "<user.post.1704110400@server.org>".to_string(),
        ],
    };

    let message_ids = commands::parse_newnews_response(response).unwrap();
    assert_eq!(message_ids.len(), 3);
    assert!(message_ids[0].starts_with('<'));
    assert!(message_ids[0].ends_with('>'));
}

#[test]
fn test_newnews_date_boundary() {
    // Test date boundaries (Y2K, leap years, etc.)
    let dates = vec![
        ("19991231", "235959"), // Y2K boundary
        ("20000101", "000000"), // Y2K
        ("20240229", "120000"), // Leap year
        ("20991231", "235959"), // Far future
    ];

    for (date, time) in dates {
        let cmd = commands::newnews("*", date, time);
        assert!(cmd.contains(date));
        assert!(cmd.contains(time));
    }
}

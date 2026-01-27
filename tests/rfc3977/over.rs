//! RFC 3977 Section 8.3 - OVER Command Tests
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc3977#section-8.3
//!
//! OVER is the RFC 3977 standard name for the XOVER command. It retrieves
//! article overview data (metadata) for a range of articles or a specific
//! message-id. This is more efficient than fetching full articles when you
//! only need to browse or search.
//!
//! Note: Response parsing is tested in xover.rs since OVER and XOVER use
//! the same response format. These tests focus on command format and
//! integration-level behavior.

use nntp_rs::{codes, commands, NntpResponse};
#[test]
fn test_over_command_format_with_range() {
    let cmd = commands::over("1-100");
    assert_eq!(cmd, "OVER 1-100\r\n");
}

#[test]
fn test_over_command_format_single_article() {
    let cmd = commands::over("12345");
    assert_eq!(cmd, "OVER 12345\r\n");
}

#[test]
fn test_over_command_format_current_article() {
    let cmd = commands::over_current();
    assert_eq!(cmd, "OVER\r\n");
}

#[test]
fn test_over_command_format_with_message_id() {
    let cmd = commands::over("<abc@example.com>");
    assert_eq!(cmd, "OVER <abc@example.com>\r\n");
}

#[test]
fn test_over_command_ends_with_crlf() {
    let cmd = commands::over("100-200");
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_over_current_ends_with_crlf() {
    let cmd = commands::over_current();
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_over_command_uppercase() {
    let cmd = commands::over("1-100");
    assert!(cmd.starts_with("OVER"));
}

#[test]
fn test_over_range_open_ended() {
    // RFC 3977 allows "100-" for "from 100 to end"
    let cmd = commands::over("100-");
    assert_eq!(cmd, "OVER 100-\r\n");
}

#[test]
fn test_over_range_up_to() {
    // RFC 3977 allows "-100" for "up to 100"
    let cmd = commands::over("-100");
    assert_eq!(cmd, "OVER -100\r\n");
}

#[test]
fn test_over_binary_newsgroup_article() {
    // Large article numbers common in binary newsgroups
    let cmd = commands::over("999999999");
    assert_eq!(cmd, "OVER 999999999\r\n");
}

// Response Code Handling (RFC 3977 §8.3)

#[test]
fn test_over_success_response_224() {
    let response = NntpResponse {
        code: codes::OVERVIEW_INFO_FOLLOWS,
        message: "Overview information follows".to_string(),
        lines: vec!["12345\tSubject\tFrom\tDate\t<msgid>\t\t1000\t100".to_string()],
    };

    assert!(response.is_success());
    assert_eq!(response.code, 224);
}

#[test]
fn test_over_empty_response() {
    // No articles in range is valid - returns empty list
    let response = NntpResponse {
        code: codes::OVERVIEW_INFO_FOLLOWS,
        message: "Overview information follows".to_string(),
        lines: vec![],
    };

    assert!(response.is_success());
    assert!(response.lines.is_empty());
}

#[test]
fn test_over_multiple_articles() {
    let response = NntpResponse {
        code: codes::OVERVIEW_INFO_FOLLOWS,
        message: "Overview information follows".to_string(),
        lines: vec![
            "100\tSubject 1\tFrom 1\tDate 1\t<msg1>\t\t1000\t50".to_string(),
            "101\tSubject 2\tFrom 2\tDate 2\t<msg2>\t<msg1>\t2000\t75".to_string(),
            "102\tSubject 3\tFrom 3\tDate 3\t<msg3>\t<msg2>\t1500\t60".to_string(),
        ],
    };

    assert!(response.is_success());
    assert_eq!(response.lines.len(), 3);
}

// Error Code Handling (RFC 3977 §8.3)

#[test]
fn test_over_no_group_selected_412() {
    let response = NntpResponse {
        code: codes::NO_GROUP_SELECTED,
        message: "No newsgroup selected".to_string(),
        lines: vec![],
    };

    assert!(response.is_error());
    assert_eq!(response.code, 412);
}

#[test]
fn test_over_no_current_article_420() {
    // When using OVER with no argument and no current article
    let response = NntpResponse {
        code: codes::NO_CURRENT_ARTICLE,
        message: "Current article number is invalid".to_string(),
        lines: vec![],
    };

    assert!(response.is_error());
    assert_eq!(response.code, 420);
}

#[test]
fn test_over_no_such_article_423() {
    // Article number not in group
    let response = NntpResponse {
        code: codes::NO_SUCH_ARTICLE_NUMBER,
        message: "No article with that number".to_string(),
        lines: vec![],
    };

    assert!(response.is_error());
    assert_eq!(response.code, 423);
}

#[test]
fn test_over_no_such_article_id_430() {
    // Message-ID not found
    let response = NntpResponse {
        code: codes::NO_SUCH_ARTICLE_ID,
        message: "No article with that message-id".to_string(),
        lines: vec![],
    };

    assert!(response.is_error());
    assert_eq!(response.code, 430);
}

#[test]
fn test_over_protocol_error_500() {
    let response = NntpResponse {
        code: 500,
        message: "Command not recognized".to_string(),
        lines: vec![],
    };

    assert!(response.is_error());
}

#[test]
fn test_over_wrong_success_code() {
    // Server returned wrong success code (should be 224)
    let response = NntpResponse {
        code: 220, // ARTICLE_FOLLOWS instead of OVERVIEW_INFO_FOLLOWS
        message: "Article follows".to_string(),
        lines: vec![],
    };

    assert!(response.is_success());
    assert_ne!(response.code, codes::OVERVIEW_INFO_FOLLOWS);
}

#[test]
fn test_over_very_large_range() {
    // 10,000 articles in response
    let mut lines = Vec::new();
    for i in 1..=10000 {
        lines.push(format!(
            "{}\tSubject {}\tFrom\tDate\t<msg{}>\t\t1000\t100",
            i, i, i
        ));
    }

    let response = NntpResponse {
        code: codes::OVERVIEW_INFO_FOLLOWS,
        message: "Overview information follows".to_string(),
        lines,
    };

    assert_eq!(response.lines.len(), 10000);
}

#[test]
fn test_over_sparse_article_numbers() {
    // Non-sequential article numbers (gaps are normal)
    let response = NntpResponse {
        code: codes::OVERVIEW_INFO_FOLLOWS,
        message: "Overview information follows".to_string(),
        lines: vec![
            "100\tSubject\tFrom\tDate\t<msg1>\t\t1000\t50".to_string(),
            "150\tSubject\tFrom\tDate\t<msg2>\t\t1000\t50".to_string(),
            "500\tSubject\tFrom\tDate\t<msg3>\t\t1000\t50".to_string(),
            "999\tSubject\tFrom\tDate\t<msg4>\t\t1000\t50".to_string(),
        ],
    };

    assert_eq!(response.lines.len(), 4);
}

#[test]
fn test_over_message_id_zero_article_number() {
    // RFC 3977 §8.3: When queried by message-id, article number is 0
    let response = NntpResponse {
        code: codes::OVERVIEW_INFO_FOLLOWS,
        message: "Overview information follows".to_string(),
        lines: vec!["0\tSubject\tFrom\tDate\t<msgid@example.com>\t\t1000\t100".to_string()],
    };

    assert!(response.is_success());
    assert!(response.lines[0].starts_with("0\t"));
}

#[test]
fn test_over_mixed_line_formats() {
    // Some lines may have extra fields (XREF, etc.)
    let response = NntpResponse {
        code: codes::OVERVIEW_INFO_FOLLOWS,
        message: "Overview information follows".to_string(),
        lines: vec![
            "100\tSubject\tFrom\tDate\t<msg1>\t\t1000\t50".to_string(),
            "101\tSubject\tFrom\tDate\t<msg2>\t\t1000\t50\txref:server group:101".to_string(),
            "102\tSubject\tFrom\tDate\t<msg3>\t\t1000\t50\txref:server group:102\textra"
                .to_string(),
        ],
    };

    assert_eq!(response.lines.len(), 3);
}

// Real-World Scenarios

#[test]
fn test_over_binary_newsgroup_overview() {
    // Binary newsgroups often have large article numbers and yEnc subjects
    let response = NntpResponse {
        code: codes::OVERVIEW_INFO_FOLLOWS,
        message: "Overview information follows".to_string(),
        lines: vec![
            "987654321\t[1/50] - \"file.rar\" yEnc (1/100)\tposter@example.com\tDate\t<part1@server>\t\t52428800\t100000".to_string(),
            "987654322\t[2/50] - \"file.rar\" yEnc (2/100)\tposter@example.com\tDate\t<part2@server>\t\t52428800\t100000".to_string(),
        ],
    };

    assert!(response.is_success());
    assert_eq!(response.lines.len(), 2);
    assert!(response.lines[0].contains("yEnc"));
}

#[test]
fn test_over_threaded_discussion() {
    // Typical threaded discussion with references
    let response = NntpResponse {
        code: codes::OVERVIEW_INFO_FOLLOWS,
        message: "Overview information follows".to_string(),
        lines: vec![
            "100\tOriginal Post\tauthor1@example.com\tDate\t<orig@example>\t\t1000\t50".to_string(),
            "101\tRe: Original Post\tauthor2@example.com\tDate\t<reply1@example>\t<orig@example>\t1200\t60".to_string(),
            "102\tRe: Original Post\tauthor3@example.com\tDate\t<reply2@example>\t<orig@example> <reply1@example>\t1500\t70".to_string(),
        ],
    };

    assert_eq!(response.lines.len(), 3);
    // First message has no references
    assert!(response.lines[0].ends_with("\t\t1000\t50"));
    // Replies have references
    assert!(response.lines[1].contains("<orig@example>"));
    assert!(response.lines[2].contains("<orig@example> <reply1@example>"));
}

#[test]
fn test_over_rfc_example() {
    // RFC 3977 Section 8.3 example response format
    let response = NntpResponse {
        code: 224,
        message: "Overview information follows".to_string(),
        lines: vec![
            "3000234\tI am just a test article\t\"Demo User\" <nobody@example.com>\t6 Oct 1998 04:38:40 -0500\t<45223423@example.com>\t<45454@example.net>\t1234\t17".to_string(),
        ],
    };

    assert!(response.is_success());
    assert_eq!(response.code, codes::OVERVIEW_INFO_FOLLOWS);
}

#[test]
fn test_over_current_article_usage() {
    // OVER with no argument requires current article to be set
    // This would be after STAT, ARTICLE, HEAD, BODY, NEXT, or LAST
    let cmd = commands::over_current();
    assert_eq!(cmd, "OVER\r\n");
}

#[test]
fn test_over_range_formats() {
    // Test all valid range formats
    assert_eq!(commands::over("100"), "OVER 100\r\n"); // Single article
    assert_eq!(commands::over("100-200"), "OVER 100-200\r\n"); // Closed range
    assert_eq!(commands::over("100-"), "OVER 100-\r\n"); // Open-ended
    assert_eq!(commands::over("-200"), "OVER -200\r\n"); // Up to
}

#[test]
fn test_over_whitespace_in_fields() {
    // Ensure internal whitespace in fields is preserved
    let response = NntpResponse {
        code: codes::OVERVIEW_INFO_FOLLOWS,
        message: "Overview information follows".to_string(),
        lines: vec![
            "100\tHello World Test\tJohn Doe <john@example.com>\tMon, 1 Jan 2024\t<msgid>\t<ref1> <ref2>\t1000\t50".to_string(),
        ],
    };

    assert!(response.lines[0].contains("Hello World Test"));
    assert!(response.lines[0].contains("John Doe <john@example.com>"));
}

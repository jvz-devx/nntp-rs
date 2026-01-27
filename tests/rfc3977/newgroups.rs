//! RFC 3977 Section 7.3 - NEWGROUPS Command Tests
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc3977#section-7.3
//!
//! Tests for the NEWGROUPS command which lists newsgroups created since a specified date/time.

use nntp_rs::{NntpError, NntpResponse, codes, commands};
#[test]
fn test_newgroups_command_format() {
    // RFC 3977 §7.3: NEWGROUPS yyyymmdd hhmmss [GMT]
    let cmd = commands::newgroups("20240101", "000000");
    assert_eq!(cmd, "NEWGROUPS 20240101 000000\r\n");
}

#[test]
fn test_newgroups_gmt_command_format() {
    let cmd = commands::newgroups_gmt("20240101", "120000");
    assert_eq!(cmd, "NEWGROUPS 20240101 120000 GMT\r\n");
}

#[test]
fn test_newgroups_command_ends_with_crlf() {
    let cmd = commands::newgroups("20240101", "000000");
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_newgroups_gmt_command_ends_with_crlf() {
    let cmd = commands::newgroups_gmt("20240101", "000000");
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_newgroups_command_uppercase() {
    let cmd = commands::newgroups("20240101", "000000");
    assert!(cmd.starts_with("NEWGROUPS"));
}

#[test]
fn test_newgroups_various_dates() {
    // Test different date formats
    let cmd1 = commands::newgroups("19700101", "000000"); // Unix epoch
    assert!(cmd1.contains("19700101"));

    let cmd2 = commands::newgroups("20380119", "031407"); // Year 2038 problem
    assert!(cmd2.contains("20380119 031407"));

    let cmd3 = commands::newgroups("20231231", "235959"); // End of year
    assert!(cmd3.contains("20231231 235959"));
}

#[test]
fn test_newgroups_gmt_has_gmt_flag() {
    let cmd = commands::newgroups_gmt("20240101", "120000");
    assert!(cmd.contains("GMT"));
    assert!(cmd.ends_with("GMT\r\n"));
}
#[test]
fn test_parse_newgroups_success() {
    // RFC 3977 §7.3: Response format is same as LIST ACTIVE
    let response = NntpResponse {
        code: codes::NEW_NEWSGROUPS_FOLLOW,
        message: "List of new newsgroups follows".to_string(),
        lines: vec![
            "comp.lang.rust 12345 1000 y".to_string(),
            "alt.test 5000 1 n".to_string(),
        ],
    };

    let groups = commands::parse_newgroups_response(response).unwrap();
    assert_eq!(groups.len(), 2);

    assert_eq!(groups[0].name, "comp.lang.rust");
    assert_eq!(groups[0].high, 12345);
    assert_eq!(groups[0].low, 1000);
    assert_eq!(groups[0].status, "y");

    assert_eq!(groups[1].name, "alt.test");
    assert_eq!(groups[1].high, 5000);
    assert_eq!(groups[1].low, 1);
    assert_eq!(groups[1].status, "n");
}

#[test]
fn test_parse_newgroups_empty_response() {
    // No new groups since specified date
    let response = NntpResponse {
        code: codes::NEW_NEWSGROUPS_FOLLOW,
        message: "No new newsgroups".to_string(),
        lines: vec![],
    };

    let groups = commands::parse_newgroups_response(response).unwrap();
    assert_eq!(groups.len(), 0);
}

#[test]
fn test_parse_newgroups_single_group() {
    let response = NntpResponse {
        code: codes::NEW_NEWSGROUPS_FOLLOW,
        message: "List of new newsgroups follows".to_string(),
        lines: vec!["alt.binaries.test 999999 100000 y".to_string()],
    };

    let groups = commands::parse_newgroups_response(response).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].name, "alt.binaries.test");
    assert_eq!(groups[0].high, 999999);
    assert_eq!(groups[0].low, 100000);
}

#[test]
fn test_parse_newgroups_moderated_groups() {
    let response = NntpResponse {
        code: codes::NEW_NEWSGROUPS_FOLLOW,
        message: "List of new newsgroups follows".to_string(),
        lines: vec![
            "news.announce.important 1000 1 m".to_string(),
            "comp.lang.python.announce 5000 1 m".to_string(),
        ],
    };

    let groups = commands::parse_newgroups_response(response).unwrap();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].status, "m");
    assert_eq!(groups[1].status, "m");
}

#[test]
fn test_parse_newgroups_various_statuses() {
    let response = NntpResponse {
        code: codes::NEW_NEWSGROUPS_FOLLOW,
        message: "List follows".to_string(),
        lines: vec![
            "group.allowed 100 1 y".to_string(),
            "group.notallowed 200 1 n".to_string(),
            "group.moderated 300 1 m".to_string(),
        ],
    };

    let groups = commands::parse_newgroups_response(response).unwrap();
    assert_eq!(groups.len(), 3);
    assert_eq!(groups[0].status, "y");
    assert_eq!(groups[1].status, "n");
    assert_eq!(groups[2].status, "m");
}

#[test]
fn test_parse_newgroups_large_article_numbers() {
    let response = NntpResponse {
        code: codes::NEW_NEWSGROUPS_FOLLOW,
        message: "List follows".to_string(),
        lines: vec![
            "alt.binaries.huge 18446744073709551615 1 y".to_string(), // u64::MAX
        ],
    };

    let groups = commands::parse_newgroups_response(response).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].high, u64::MAX);
}

#[test]
fn test_parse_newgroups_rfc_example() {
    // RFC 3977 Section 7.3 example format
    let response = NntpResponse {
        code: 231,
        message: "list of new newsgroups follows".to_string(),
        lines: vec![
            "misc.test 3000234 3000000 y".to_string(),
            "alt.rfc-writers.recovery 4999 4099 y".to_string(),
            "tx.natives.recovery 89 56 y".to_string(),
        ],
    };

    let groups = commands::parse_newgroups_response(response).unwrap();
    assert_eq!(groups.len(), 3);
    assert_eq!(groups[0].name, "misc.test");
    assert_eq!(groups[1].name, "alt.rfc-writers.recovery");
    assert_eq!(groups[2].name, "tx.natives.recovery");
}
#[test]
fn test_parse_newgroups_protocol_error() {
    let response = NntpResponse {
        code: 500,
        message: "Command not recognized".to_string(),
        lines: vec![],
    };

    let result = commands::parse_newgroups_response(response);
    assert!(result.is_err());
    match result {
        Err(NntpError::Protocol { code, .. }) => assert_eq!(code, 500),
        _ => panic!("Expected Protocol error"),
    }
}

#[test]
fn test_parse_newgroups_malformed_line_skipped() {
    let response = NntpResponse {
        code: codes::NEW_NEWSGROUPS_FOLLOW,
        message: "List follows".to_string(),
        lines: vec![
            "comp.test 100 1 y".to_string(),
            "malformed".to_string(), // Only 1 field - should be skipped
            "alt.test 200 1 n".to_string(),
        ],
    };

    let groups = commands::parse_newgroups_response(response).unwrap();
    // Should have 2 groups, malformed line skipped
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].name, "comp.test");
    assert_eq!(groups[1].name, "alt.test");
}

#[test]
fn test_parse_newgroups_invalid_numbers_default_to_zero() {
    let response = NntpResponse {
        code: codes::NEW_NEWSGROUPS_FOLLOW,
        message: "List follows".to_string(),
        lines: vec!["test.group invalid invalid y".to_string()],
    };

    let groups = commands::parse_newgroups_response(response).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].high, 0);
    assert_eq!(groups[0].low, 0);
}

#[test]
fn test_parse_newgroups_wrong_success_code() {
    // Wrong success code (should be 231, not 215)
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS, // 215
        message: "Wrong code".to_string(),
        lines: vec!["test 100 1 y".to_string()],
    };

    let groups = commands::parse_newgroups_response(response).unwrap();
    // Parser doesn't validate specific success code, just checks is_success()
    assert_eq!(groups.len(), 1);
}
#[test]
fn test_newgroups_success_code_231() {
    let response = NntpResponse {
        code: codes::NEW_NEWSGROUPS_FOLLOW,
        message: "List follows".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 231);
    assert!(response.is_success());
}

#[test]
fn test_newgroups_error_code_handling() {
    // Test various error codes that might be returned
    let error_codes = vec![
        (500, "Command not recognized"),
        (501, "Command syntax error"),
        (503, "Feature not supported"),
    ];

    for (code, msg) in error_codes {
        let response = NntpResponse {
            code,
            message: msg.to_string(),
            lines: vec![],
        };

        assert!(response.is_error());
        let result = commands::parse_newgroups_response(response);
        assert!(result.is_err());
    }
}

#[test]
fn test_parse_newgroups_very_long_list() {
    // Simulate server returning many new groups
    let mut lines = Vec::new();
    for i in 0..1000 {
        lines.push(format!("test.group.{} {} 1 y", i, i * 100));
    }

    let response = NntpResponse {
        code: codes::NEW_NEWSGROUPS_FOLLOW,
        message: "Many groups follow".to_string(),
        lines,
    };

    let groups = commands::parse_newgroups_response(response).unwrap();
    assert_eq!(groups.len(), 1000);
    assert_eq!(groups[0].name, "test.group.0");
    assert_eq!(groups[999].name, "test.group.999");
}

#[test]
fn test_parse_newgroups_extra_whitespace() {
    let response = NntpResponse {
        code: codes::NEW_NEWSGROUPS_FOLLOW,
        message: "List follows".to_string(),
        lines: vec![
            "  comp.test   100   1   y  ".to_string(), // Extra spaces
        ],
    };

    let groups = commands::parse_newgroups_response(response).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].name, "comp.test");
    assert_eq!(groups[0].high, 100);
}

#[test]
fn test_parse_newgroups_zero_article_numbers() {
    // Groups with no articles yet
    let response = NntpResponse {
        code: codes::NEW_NEWSGROUPS_FOLLOW,
        message: "List follows".to_string(),
        lines: vec!["new.empty.group 0 0 y".to_string()],
    };

    let groups = commands::parse_newgroups_response(response).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].high, 0);
    assert_eq!(groups[0].low, 0);
}

#[test]
fn test_parse_newgroups_all_malformed() {
    let response = NntpResponse {
        code: codes::NEW_NEWSGROUPS_FOLLOW,
        message: "List follows".to_string(),
        lines: vec![
            "invalid".to_string(),
            "also invalid".to_string(),
            "still wrong".to_string(),
        ],
    };

    let groups = commands::parse_newgroups_response(response).unwrap();
    // All lines malformed, should return empty vec
    assert_eq!(groups.len(), 0);
}

// Real-World Scenarios

#[test]
fn test_newgroups_binary_newsgroups() {
    // Binary newsgroups often have large article numbers
    let response = NntpResponse {
        code: codes::NEW_NEWSGROUPS_FOLLOW,
        message: "New binary groups".to_string(),
        lines: vec![
            "alt.binaries.test 5000000 4000000 y".to_string(),
            "alt.binaries.pictures 10000000 9000000 y".to_string(),
        ],
    };

    let groups = commands::parse_newgroups_response(response).unwrap();
    assert_eq!(groups.len(), 2);
    assert!(groups[0].name.starts_with("alt.binaries"));
    assert!(groups[0].high > 1_000_000);
}

#[test]
fn test_newgroups_hierarchical_names() {
    // Test various newsgroup hierarchies
    let response = NntpResponse {
        code: codes::NEW_NEWSGROUPS_FOLLOW,
        message: "List follows".to_string(),
        lines: vec![
            "comp.lang.rust.announce 100 1 m".to_string(),
            "alt.test.test.test.test 200 1 y".to_string(),
            "misc.misc 300 1 n".to_string(),
        ],
    };

    let groups = commands::parse_newgroups_response(response).unwrap();
    assert_eq!(groups.len(), 3);
    assert!(groups[0].name.contains("."));
    assert!(groups[1].name.matches('.').count() >= 3);
}

#[test]
fn test_newgroups_typical_server_response() {
    // Simulate a typical server response with a few new groups
    let response = NntpResponse {
        code: 231,
        message: "List of new newsgroups follows (multi-line)".to_string(),
        lines: vec![
            "comp.programming.rust 15234 12001 y".to_string(),
            "alt.test.2024 500 1 y".to_string(),
        ],
    };

    assert!(response.is_success());
    let groups = commands::parse_newgroups_response(response).unwrap();
    assert_eq!(groups.len(), 2);
}

#[test]
fn test_newgroups_date_boundaries() {
    // Test date format validation (via command builders)
    let cmd1 = commands::newgroups("20000101", "000000"); // Y2K
    assert!(cmd1.contains("20000101"));

    let cmd2 = commands::newgroups_gmt("20991231", "235959"); // End of century
    assert!(cmd2.contains("20991231 235959 GMT"));
}

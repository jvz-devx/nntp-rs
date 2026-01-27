//! RFC 3977 Section 7.6.1 - LIST ACTIVE Command Tests
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc3977#section-7.6.1
//!
//! Tests for the LIST ACTIVE command and response parsing.

use nntp_rs::{codes, commands, NntpResponse};

#[test]
fn test_list_active_command_format() {
    let cmd = commands::list_active("*");
    assert_eq!(cmd, "LIST ACTIVE *\r\n");
}

#[test]
fn test_list_active_command_ends_with_crlf() {
    let cmd = commands::list_active("comp.*");
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_list_active_command_uppercase() {
    let cmd = commands::list_active("*");
    assert!(cmd.starts_with("LIST ACTIVE"));
}

#[test]
fn test_list_active_with_wildmat() {
    let cmd = commands::list_active("comp.lang.*");
    assert_eq!(cmd, "LIST ACTIVE comp.lang.*\r\n");
}

#[test]
fn test_list_active_all_groups() {
    let cmd = commands::list_active("*");
    assert!(cmd.contains("*"));
}
#[test]
fn test_list_active_response_code_215() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List of newsgroups follows".to_string(),
        lines: vec!["comp.lang.rust 12345 1000 y".to_string()],
    };

    assert_eq!(response.code, 215);
    assert!(response.is_success());
    assert!(!response.is_error());
}

#[test]
fn test_list_active_response_is_success() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Newsgroups follow".to_string(),
        lines: vec![],
    };

    assert!(response.is_success());
    assert!(!response.is_error());
    assert!(!response.is_continuation());
}

#[test]
fn test_list_active_response_has_multiline_data() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![
            "comp.lang.rust 12345 1000 y".to_string(),
            "comp.lang.python 54321 5000 y".to_string(),
        ],
    };

    assert!(!response.lines.is_empty());
    assert_eq!(response.lines.len(), 2);
}
#[test]
fn test_parse_list_active_single_group() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec!["comp.lang.rust 12345 1000 y".to_string()],
    };

    let groups = commands::parse_list_active_response(response).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].name, "comp.lang.rust");
    assert_eq!(groups[0].high, 12345);
    assert_eq!(groups[0].low, 1000);
    assert_eq!(groups[0].status, "y");
}

#[test]
fn test_parse_list_active_multiple_groups() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![
            "comp.lang.rust 12345 1000 y".to_string(),
            "comp.lang.python 54321 5000 n".to_string(),
            "comp.lang.go 9999 100 m".to_string(),
        ],
    };

    let groups = commands::parse_list_active_response(response).unwrap();
    assert_eq!(groups.len(), 3);

    assert_eq!(groups[0].name, "comp.lang.rust");
    assert_eq!(groups[0].status, "y");

    assert_eq!(groups[1].name, "comp.lang.python");
    assert_eq!(groups[1].status, "n");

    assert_eq!(groups[2].name, "comp.lang.go");
    assert_eq!(groups[2].status, "m");
}

#[test]
fn test_parse_list_active_status_codes() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List".to_string(),
        lines: vec![
            "group.posting.allowed 100 1 y".to_string(),
            "group.posting.forbidden 200 10 n".to_string(),
            "group.moderated 300 20 m".to_string(),
        ],
    };

    let groups = commands::parse_list_active_response(response).unwrap();

    // y = posting allowed
    assert_eq!(groups[0].status, "y");
    // n = no posting
    assert_eq!(groups[1].status, "n");
    // m = moderated
    assert_eq!(groups[2].status, "m");
}

#[test]
fn test_parse_list_active_empty_list() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![],
    };

    let groups = commands::parse_list_active_response(response).unwrap();
    assert_eq!(groups.len(), 0);
}

#[test]
fn test_parse_list_active_article_numbers() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List".to_string(),
        lines: vec!["test.group 999999 1 y".to_string()],
    };

    let groups = commands::parse_list_active_response(response).unwrap();
    assert_eq!(groups[0].high, 999999);
    assert_eq!(groups[0].low, 1);
}
#[test]
fn test_parse_list_active_protocol_error() {
    let response = NntpResponse {
        code: 500,
        message: "Command not recognized".to_string(),
        lines: vec![],
    };

    let result = commands::parse_list_active_response(response);
    assert!(result.is_err());
}

#[test]
fn test_parse_list_active_malformed_line_skipped() {
    // Malformed lines should be skipped gracefully
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List".to_string(),
        lines: vec![
            "comp.lang.rust 12345 1000 y".to_string(),
            "malformed".to_string(), // Missing fields
            "comp.lang.go 9999 100 m".to_string(),
        ],
    };

    let groups = commands::parse_list_active_response(response).unwrap();
    // Should skip malformed line and parse the valid ones
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].name, "comp.lang.rust");
    assert_eq!(groups[1].name, "comp.lang.go");
}

#[test]
fn test_parse_list_active_invalid_numbers_defaults_to_zero() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List".to_string(),
        lines: vec!["test.group invalid also_invalid y".to_string()],
    };

    let groups = commands::parse_list_active_response(response).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].high, 0);
    assert_eq!(groups[0].low, 0);
}

// Real-World Examples

#[test]
fn test_list_active_typical_server_response() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "list of newsgroups follows".to_string(),
        lines: vec![
            "alt.binaries.test 12345 1000 y".to_string(),
            "comp.lang.rust 54321 5000 y".to_string(),
            "misc.test 100 1 n".to_string(),
            "news.announce.newusers 5000 100 m".to_string(),
        ],
    };

    let groups = commands::parse_list_active_response(response).unwrap();
    assert_eq!(groups.len(), 4);
    assert!(groups.iter().any(|g| g.name == "comp.lang.rust"));
}

#[test]
fn test_list_active_group_with_dots() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List".to_string(),
        lines: vec!["alt.binaries.multimedia.anime 999 1 y".to_string()],
    };

    let groups = commands::parse_list_active_response(response).unwrap();
    assert_eq!(groups[0].name, "alt.binaries.multimedia.anime");
}

#[test]
fn test_list_active_high_equals_low() {
    // Empty group: high == low
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List".to_string(),
        lines: vec!["empty.group 0 0 y".to_string()],
    };

    let groups = commands::parse_list_active_response(response).unwrap();
    assert_eq!(groups[0].high, 0);
    assert_eq!(groups[0].low, 0);
}

#[test]
fn test_list_active_large_numbers() {
    // Very large article numbers
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List".to_string(),
        lines: vec!["busy.group 18446744073709551615 1000000000 y".to_string()],
    };

    let groups = commands::parse_list_active_response(response).unwrap();
    assert!(groups[0].high > 1000000);
}

#[test]
fn test_list_active_no_such_newsgroup_pattern() {
    // Server may return empty list for non-matching pattern
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![],
    };

    let groups = commands::parse_list_active_response(response).unwrap();
    assert_eq!(groups.len(), 0);
}

#[test]
fn test_list_active_command_unavailable() {
    let response = NntpResponse {
        code: 502,
        message: "Command unavailable".to_string(),
        lines: vec![],
    };

    assert!(response.is_error());
    assert_eq!(response.code, 502);
}

#[test]
fn test_list_active_group_name_with_hyphen() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List".to_string(),
        lines: vec!["alt.binaries.test-group 100 1 y".to_string()],
    };

    let groups = commands::parse_list_active_response(response).unwrap();
    assert_eq!(groups[0].name, "alt.binaries.test-group");
}

#[test]
fn test_list_active_extra_whitespace() {
    // Some servers may have extra whitespace
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List".to_string(),
        lines: vec!["comp.lang.rust  12345  1000  y".to_string()],
    };

    let groups = commands::parse_list_active_response(response).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].name, "comp.lang.rust");
}

#[test]
fn test_list_active_status_default_on_empty() {
    // If status field is missing, line is malformed and skipped
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List".to_string(),
        lines: vec!["test.group 100 1".to_string()], // Only 3 fields - malformed
    };

    let groups = commands::parse_list_active_response(response).unwrap();
    // Malformed lines are skipped
    assert_eq!(groups.len(), 0);
}

#[test]
fn test_list_active_many_groups() {
    // Large response with many groups
    let lines: Vec<String> = (0..1000)
        .map(|i| format!("test.group.{} {} 1 y", i, i * 100))
        .collect();

    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines,
    };

    let groups = commands::parse_list_active_response(response).unwrap();
    assert_eq!(groups.len(), 1000);
}

#[test]
fn test_list_active_code_constant_value() {
    // Verify the response code constant is correct
    assert_eq!(codes::LIST_INFORMATION_FOLLOWS, 215);
}

#[test]
fn test_active_group_struct_fields() {
    // Verify ActiveGroup struct has correct fields
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List".to_string(),
        lines: vec!["test.group 100 50 y".to_string()],
    };

    let groups = commands::parse_list_active_response(response).unwrap();
    let group = &groups[0];

    // Test all fields are accessible
    assert_eq!(group.name, "test.group");
    assert_eq!(group.high, 100);
    assert_eq!(group.low, 50);
    assert_eq!(group.status, "y");
}

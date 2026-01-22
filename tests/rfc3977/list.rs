//! RFC 3977 Section 7.6 - LIST Command Tests
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc3977#section-7.6
//!
//! Tests for LIST variants including LIST ACTIVE and LIST NEWSGROUPS.

use nntp_rs::{codes, commands, NntpError, NntpResponse};

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

    let groups = commands::parse_list_active_response(&response).unwrap();
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

    let groups = commands::parse_list_active_response(&response).unwrap();
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

    let groups = commands::parse_list_active_response(&response).unwrap();

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

    let groups = commands::parse_list_active_response(&response).unwrap();
    assert_eq!(groups.len(), 0);
}

#[test]
fn test_parse_list_active_article_numbers() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List".to_string(),
        lines: vec!["test.group 999999 1 y".to_string()],
    };

    let groups = commands::parse_list_active_response(&response).unwrap();
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

    let result = commands::parse_list_active_response(&response);
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

    let groups = commands::parse_list_active_response(&response).unwrap();
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

    let groups = commands::parse_list_active_response(&response).unwrap();
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

    let groups = commands::parse_list_active_response(&response).unwrap();
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

    let groups = commands::parse_list_active_response(&response).unwrap();
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

    let groups = commands::parse_list_active_response(&response).unwrap();
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

    let groups = commands::parse_list_active_response(&response).unwrap();
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

    let groups = commands::parse_list_active_response(&response).unwrap();
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

    let groups = commands::parse_list_active_response(&response).unwrap();
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

    let groups = commands::parse_list_active_response(&response).unwrap();
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

    let groups = commands::parse_list_active_response(&response).unwrap();
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

    let groups = commands::parse_list_active_response(&response).unwrap();
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

    let groups = commands::parse_list_active_response(&response).unwrap();
    let group = &groups[0];

    // Test all fields are accessible
    assert_eq!(group.name, "test.group");
    assert_eq!(group.high, 100);
    assert_eq!(group.low, 50);
    assert_eq!(group.status, "y");
}

#[test]
fn test_list_newsgroups_command_format() {
    let cmd = commands::list_newsgroups("*");
    assert_eq!(cmd, "LIST NEWSGROUPS *\r\n");
}

#[test]
fn test_list_newsgroups_command_ends_with_crlf() {
    let cmd = commands::list_newsgroups("comp.*");
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_list_newsgroups_command_uppercase() {
    let cmd = commands::list_newsgroups("*");
    assert!(cmd.starts_with("LIST NEWSGROUPS"));
}

#[test]
fn test_list_newsgroups_with_wildmat() {
    let cmd = commands::list_newsgroups("alt.binaries.*");
    assert_eq!(cmd, "LIST NEWSGROUPS alt.binaries.*\r\n");
}

#[test]
fn test_list_newsgroups_all_groups() {
    let cmd = commands::list_newsgroups("*");
    assert!(cmd.contains("*"));
}
#[test]
fn test_list_newsgroups_response_code_215() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Descriptions follow".to_string(),
        lines: vec!["comp.lang.rust The Rust programming language".to_string()],
    };

    assert_eq!(response.code, 215);
    assert!(response.is_success());
    assert!(!response.is_error());
}

#[test]
fn test_list_newsgroups_response_is_success() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Descriptions follow".to_string(),
        lines: vec![],
    };

    assert!(response.is_success());
    assert!(!response.is_error());
    assert!(!response.is_continuation());
}

#[test]
fn test_list_newsgroups_response_has_multiline_data() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![
            "comp.lang.rust The Rust programming language".to_string(),
            "comp.lang.python Python discussion".to_string(),
        ],
    };

    assert!(!response.lines.is_empty());
    assert_eq!(response.lines.len(), 2);
}
#[test]
fn test_parse_list_newsgroups_single_group() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec!["comp.lang.rust The Rust programming language".to_string()],
    };

    let groups = commands::parse_list_newsgroups_response(&response).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].name, "comp.lang.rust");
    assert_eq!(groups[0].description, "The Rust programming language");
}

#[test]
fn test_parse_list_newsgroups_multiple_groups() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![
            "comp.lang.rust The Rust programming language".to_string(),
            "comp.lang.python Discussion about Python".to_string(),
            "alt.binaries.test Binary testing group".to_string(),
        ],
    };

    let groups = commands::parse_list_newsgroups_response(&response).unwrap();
    assert_eq!(groups.len(), 3);

    assert_eq!(groups[0].name, "comp.lang.rust");
    assert_eq!(groups[0].description, "The Rust programming language");

    assert_eq!(groups[1].name, "comp.lang.python");
    assert_eq!(groups[1].description, "Discussion about Python");

    assert_eq!(groups[2].name, "alt.binaries.test");
    assert_eq!(groups[2].description, "Binary testing group");
}

#[test]
fn test_parse_list_newsgroups_description_with_multiple_words() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List".to_string(),
        lines: vec!["comp.lang.rust A safe, concurrent, practical language".to_string()],
    };

    let groups = commands::parse_list_newsgroups_response(&response).unwrap();
    assert_eq!(
        groups[0].description,
        "A safe, concurrent, practical language"
    );
}

#[test]
fn test_parse_list_newsgroups_empty_list() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![],
    };

    let groups = commands::parse_list_newsgroups_response(&response).unwrap();
    assert_eq!(groups.len(), 0);
}

#[test]
fn test_parse_list_newsgroups_description_with_special_chars() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List".to_string(),
        lines: vec!["test.group Discussion about C++ & other topics (moderated)".to_string()],
    };

    let groups = commands::parse_list_newsgroups_response(&response).unwrap();
    assert_eq!(
        groups[0].description,
        "Discussion about C++ & other topics (moderated)"
    );
}
#[test]
fn test_parse_list_newsgroups_protocol_error() {
    let response = NntpResponse {
        code: 500,
        message: "Command not recognized".to_string(),
        lines: vec![],
    };

    let result = commands::parse_list_newsgroups_response(&response);
    assert!(result.is_err());
}

#[test]
fn test_parse_list_newsgroups_malformed_line_skipped() {
    // Malformed lines (no space separator) should be skipped gracefully
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List".to_string(),
        lines: vec![
            "comp.lang.rust The Rust programming language".to_string(),
            "malformed".to_string(), // No space separator
            "comp.lang.go Go programming".to_string(),
        ],
    };

    let groups = commands::parse_list_newsgroups_response(&response).unwrap();
    // Should skip malformed line and parse the valid ones
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].name, "comp.lang.rust");
    assert_eq!(groups[1].name, "comp.lang.go");
}

#[test]
fn test_parse_list_newsgroups_empty_description() {
    // Group with empty description (just whitespace after name)
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List".to_string(),
        lines: vec!["test.group ".to_string()],
    };

    let groups = commands::parse_list_newsgroups_response(&response).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].name, "test.group");
    assert_eq!(groups[0].description, "");
}

// Real-World Examples

#[test]
fn test_list_newsgroups_typical_server_response() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Descriptions follow".to_string(),
        lines: vec![
            "comp.lang.rust The Rust programming language".to_string(),
            "alt.binaries.test Testing of binary attachments".to_string(),
            "news.announce.newusers Explanatory postings for new users (Moderated)".to_string(),
        ],
    };

    let groups = commands::parse_list_newsgroups_response(&response).unwrap();
    assert_eq!(groups.len(), 3);
    assert!(groups.iter().any(|g| g.name == "comp.lang.rust"));
}

#[test]
fn test_list_newsgroups_group_with_dots() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List".to_string(),
        lines: vec!["alt.binaries.multimedia.anime Japanese animation discussion".to_string()],
    };

    let groups = commands::parse_list_newsgroups_response(&response).unwrap();
    assert_eq!(groups[0].name, "alt.binaries.multimedia.anime");
}

#[test]
fn test_list_newsgroups_long_description() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List".to_string(),
        lines: vec!["test.group This is a very long description that contains many words and explains the purpose of this newsgroup in great detail".to_string()],
    };

    let groups = commands::parse_list_newsgroups_response(&response).unwrap();
    assert!(groups[0].description.len() > 50);
}


#[test]
fn test_list_newsgroups_no_matching_pattern() {
    // Server may return empty list for non-matching pattern
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![],
    };

    let groups = commands::parse_list_newsgroups_response(&response).unwrap();
    assert_eq!(groups.len(), 0);
}

#[test]
fn test_list_newsgroups_command_unavailable() {
    let response = NntpResponse {
        code: 502,
        message: "Command unavailable".to_string(),
        lines: vec![],
    };

    assert!(response.is_error());
    assert_eq!(response.code, 502);
}


#[test]
fn test_list_newsgroups_group_name_with_hyphen() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List".to_string(),
        lines: vec!["alt.test-group Testing with hyphens".to_string()],
    };

    let groups = commands::parse_list_newsgroups_response(&response).unwrap();
    assert_eq!(groups[0].name, "alt.test-group");
}

#[test]
fn test_list_newsgroups_description_with_tabs() {
    // Some servers might use tabs
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List".to_string(),
        lines: vec!["comp.lang.rust\tThe Rust programming language".to_string()],
    };

    let groups = commands::parse_list_newsgroups_response(&response).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].name, "comp.lang.rust");
    assert_eq!(groups[0].description, "The Rust programming language");
}

#[test]
fn test_list_newsgroups_many_groups() {
    // Large response with many groups
    let lines: Vec<String> = (0..1000)
        .map(|i| format!("test.group.{} Description for group {}", i, i))
        .collect();

    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines,
    };

    let groups = commands::parse_list_newsgroups_response(&response).unwrap();
    assert_eq!(groups.len(), 1000);
}

#[test]
fn test_newsgroup_info_struct_fields() {
    // Verify NewsgroupInfo struct has correct fields
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List".to_string(),
        lines: vec!["test.group Test description".to_string()],
    };

    let groups = commands::parse_list_newsgroups_response(&response).unwrap();
    let group = &groups[0];

    // Test all fields are accessible
    assert_eq!(group.name, "test.group");
    assert_eq!(group.description, "Test description");
}

#[test]
fn test_list_newsgroups_extra_spaces_in_description() {
    // Extra spaces within description should be preserved
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List".to_string(),
        lines: vec!["test.group  Multiple  spaces  preserved".to_string()],
    };

    let groups = commands::parse_list_newsgroups_response(&response).unwrap();
    // Leading/trailing whitespace trimmed, but internal preserved
    assert_eq!(groups[0].description, "Multiple  spaces  preserved");
}

// LIST OVERVIEW.FMT Tests (RFC 3977 Section 8.4)

#[test]
fn test_list_overview_fmt_command_format() {
    let cmd = commands::list_overview_fmt();
    assert_eq!(cmd, "LIST OVERVIEW.FMT\r\n");
}

#[test]
fn test_list_overview_fmt_command_ends_with_crlf() {
    let cmd = commands::list_overview_fmt();
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_list_overview_fmt_command_uppercase() {
    let cmd = commands::list_overview_fmt();
    assert!(cmd.starts_with("LIST OVERVIEW.FMT"));
}

#[test]
fn test_parse_list_overview_fmt_response_success() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Order of fields in overview database.".to_string(),
        lines: vec![
            "Subject:".to_string(),
            "From:".to_string(),
            "Date:".to_string(),
            "Message-ID:".to_string(),
            "References:".to_string(),
            ":bytes".to_string(),
            ":lines".to_string(),
        ],
    };

    let fields = commands::parse_list_overview_fmt_response(&response).unwrap();
    assert_eq!(fields.len(), 7);
    assert_eq!(fields[0], "Subject:");
    assert_eq!(fields[1], "From:");
    assert_eq!(fields[5], ":bytes");
    assert_eq!(fields[6], ":lines");
}

#[test]
fn test_parse_list_overview_fmt_response_with_full_suffix() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Overview format".to_string(),
        lines: vec![
            "Subject:".to_string(),
            "From:".to_string(),
            "Date:".to_string(),
            "Message-ID:".to_string(),
            "References:".to_string(),
            ":bytes".to_string(),
            ":lines".to_string(),
            "Xref:full".to_string(),
            "Distribution:full".to_string(),
        ],
    };

    let fields = commands::parse_list_overview_fmt_response(&response).unwrap();
    assert_eq!(fields.len(), 9);
    assert_eq!(fields[7], "Xref:full");
    assert_eq!(fields[8], "Distribution:full");
}

#[test]
fn test_parse_list_overview_fmt_response_alternative_bytes_lines() {
    // RFC 3977: Bytes: and Lines: may be used instead of :bytes and :lines
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Overview format".to_string(),
        lines: vec![
            "Subject:".to_string(),
            "From:".to_string(),
            "Date:".to_string(),
            "Message-ID:".to_string(),
            "References:".to_string(),
            "Bytes:".to_string(),
            "Lines:".to_string(),
        ],
    };

    let fields = commands::parse_list_overview_fmt_response(&response).unwrap();
    assert_eq!(fields.len(), 7);
    assert_eq!(fields[5], "Bytes:");
    assert_eq!(fields[6], "Lines:");
}

#[test]
fn test_parse_list_overview_fmt_response_empty() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Empty overview format".to_string(),
        lines: vec![],
    };

    let fields = commands::parse_list_overview_fmt_response(&response).unwrap();
    assert_eq!(fields.len(), 0);
}

#[test]
fn test_parse_list_overview_fmt_response_minimal() {
    // Minimal valid format with required fields
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Overview format".to_string(),
        lines: vec![
            "Subject:".to_string(),
            "From:".to_string(),
            "Date:".to_string(),
            "Message-ID:".to_string(),
        ],
    };

    let fields = commands::parse_list_overview_fmt_response(&response).unwrap();
    assert_eq!(fields.len(), 4);
}

#[test]
fn test_parse_list_overview_fmt_response_extended() {
    // Extended format with many additional headers
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Overview format".to_string(),
        lines: vec![
            "Subject:".to_string(),
            "From:".to_string(),
            "Date:".to_string(),
            "Message-ID:".to_string(),
            "References:".to_string(),
            ":bytes".to_string(),
            ":lines".to_string(),
            "Xref:full".to_string(),
            "Distribution:full".to_string(),
            "Newsgroups:full".to_string(),
            "Path:full".to_string(),
            "Organization:".to_string(),
            "User-Agent:".to_string(),
        ],
    };

    let fields = commands::parse_list_overview_fmt_response(&response).unwrap();
    assert_eq!(fields.len(), 13);
    assert!(fields.contains(&"Organization:".to_string()));
    assert!(fields.contains(&"User-Agent:".to_string()));
}

#[test]
fn test_parse_list_overview_fmt_response_protocol_error() {
    let response = NntpResponse {
        code: 500,
        message: "Command not understood".to_string(),
        lines: vec![],
    };

    assert!(commands::parse_list_overview_fmt_response(&response).is_err());
}

#[test]
fn test_parse_list_overview_fmt_response_wrong_code() {
    let response = NntpResponse {
        code: 480,
        message: "Authentication required".to_string(),
        lines: vec![],
    };

    assert!(commands::parse_list_overview_fmt_response(&response).is_err());
}

#[test]
fn test_parse_list_overview_fmt_response_preserves_field_order() {
    // Order is important for parsing OVER responses
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Overview format".to_string(),
        lines: vec![
            "Subject:".to_string(),
            "From:".to_string(),
            "Date:".to_string(),
            "Message-ID:".to_string(),
            "References:".to_string(),
            ":bytes".to_string(),
            ":lines".to_string(),
        ],
    };

    let fields = commands::parse_list_overview_fmt_response(&response).unwrap();

    // Verify exact order preservation
    assert_eq!(fields[0], "Subject:");
    assert_eq!(fields[1], "From:");
    assert_eq!(fields[2], "Date:");
    assert_eq!(fields[3], "Message-ID:");
    assert_eq!(fields[4], "References:");
    assert_eq!(fields[5], ":bytes");
    assert_eq!(fields[6], ":lines");
}

#[test]
fn test_parse_list_overview_fmt_response_preserves_whitespace() {
    // Field names should be preserved exactly as returned
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Overview format".to_string(),
        lines: vec![
            "Subject:".to_string(),
            " From:".to_string(), // Leading space
            "Date: ".to_string(), // Trailing space
        ],
    };

    let fields = commands::parse_list_overview_fmt_response(&response).unwrap();
    assert_eq!(fields[0], "Subject:");
    assert_eq!(fields[1], " From:"); // Preserved
    assert_eq!(fields[2], "Date: "); // Preserved
}

#[test]
fn test_list_overview_fmt_real_world_example() {
    // Example from RFC 3977 Section 8.4
    let response = NntpResponse {
        code: 215,
        message: "Order of fields in overview database.".to_string(),
        lines: vec![
            "Subject:".to_string(),
            "From:".to_string(),
            "Date:".to_string(),
            "Message-ID:".to_string(),
            "References:".to_string(),
            ":bytes".to_string(),
            ":lines".to_string(),
            "Xref:full".to_string(),
            "Distribution:full".to_string(),
        ],
    };

    let fields = commands::parse_list_overview_fmt_response(&response).unwrap();
    assert_eq!(fields.len(), 9);

    // Verify standard headers
    assert_eq!(fields[0], "Subject:");
    assert_eq!(fields[1], "From:");
    assert_eq!(fields[2], "Date:");
    assert_eq!(fields[3], "Message-ID:");
    assert_eq!(fields[4], "References:");

    // Verify metadata
    assert_eq!(fields[5], ":bytes");
    assert_eq!(fields[6], ":lines");

    // Verify full headers
    assert_eq!(fields[7], "Xref:full");
    assert_eq!(fields[8], "Distribution:full");
}

#[test]
fn test_list_overview_fmt_case_sensitivity() {
    // Field names should be returned as-is (case preserved)
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Overview format".to_string(),
        lines: vec![
            "SUBJECT:".to_string(),
            "subject:".to_string(),
            "Subject:".to_string(),
        ],
    };

    let fields = commands::parse_list_overview_fmt_response(&response).unwrap();
    assert_eq!(fields[0], "SUBJECT:");
    assert_eq!(fields[1], "subject:");
    assert_eq!(fields[2], "Subject:");
}

#[test]
fn test_list_headers_command_format() {
    let cmd = commands::list_headers();
    assert_eq!(cmd, "LIST HEADERS\r\n");
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_list_headers_msgid_command_format() {
    let cmd = commands::list_headers_msgid();
    assert_eq!(cmd, "LIST HEADERS MSGID\r\n");
    assert!(cmd.ends_with("\r\n"));
}

#[test]
fn test_list_headers_range_command_format() {
    let cmd = commands::list_headers_range();
    assert_eq!(cmd, "LIST HEADERS RANGE\r\n");
    assert!(cmd.ends_with("\r\n"));
}

#[test]
fn test_list_headers_command_uppercase() {
    let cmd = commands::list_headers();
    assert!(cmd.starts_with("LIST HEADERS"));
}

#[test]
fn test_parse_list_headers_response_success() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "headers supported".to_string(),
        lines: vec![
            "Subject".to_string(),
            "From".to_string(),
            "Date".to_string(),
            "Message-ID".to_string(),
            "References".to_string(),
            ":lines".to_string(),
            ":bytes".to_string(),
        ],
    };

    let headers = commands::parse_list_headers_response(&response).unwrap();
    assert_eq!(headers.len(), 7);
    assert_eq!(headers[0], "Subject");
    assert_eq!(headers[1], "From");
    assert_eq!(headers[5], ":lines");
    assert_eq!(headers[6], ":bytes");
}

#[test]
fn test_parse_list_headers_response_with_colon_special() {
    // ":" means any header may be retrieved
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "metadata items supported".to_string(),
        lines: vec![":".to_string(), ":lines".to_string(), ":bytes".to_string()],
    };

    let headers = commands::parse_list_headers_response(&response).unwrap();
    assert_eq!(headers.len(), 3);
    assert_eq!(headers[0], ":");
    assert_eq!(headers[1], ":lines");
    assert_eq!(headers[2], ":bytes");
}

#[test]
fn test_parse_list_headers_response_rfc_example() {
    // Example from RFC 3977 Section 8.6
    let response = NntpResponse {
        code: 215,
        message: "headers supported:".to_string(),
        lines: vec![
            "Subject".to_string(),
            "Message-ID".to_string(),
            "Xref".to_string(),
        ],
    };

    let headers = commands::parse_list_headers_response(&response).unwrap();
    assert_eq!(headers.len(), 3);
    assert_eq!(headers[0], "Subject");
    assert_eq!(headers[1], "Message-ID");
    assert_eq!(headers[2], "Xref");
}

#[test]
fn test_parse_list_headers_response_with_metadata() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "headers supported".to_string(),
        lines: vec![
            "Subject".to_string(),
            "From".to_string(),
            ":lines".to_string(),
            ":bytes".to_string(),
            "Xref".to_string(),
        ],
    };

    let headers = commands::parse_list_headers_response(&response).unwrap();
    assert_eq!(headers.len(), 5);
    assert!(headers.contains(&"Subject".to_string()));
    assert!(headers.contains(&":lines".to_string()));
    assert!(headers.contains(&":bytes".to_string()));
}

#[test]
fn test_parse_list_headers_response_empty() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "no headers supported".to_string(),
        lines: vec![],
    };

    let headers = commands::parse_list_headers_response(&response).unwrap();
    assert_eq!(headers.len(), 0);
}

#[test]
fn test_parse_list_headers_response_protocol_error() {
    let response = NntpResponse {
        code: 500,
        message: "Command not understood".to_string(),
        lines: vec![],
    };

    assert!(commands::parse_list_headers_response(&response).is_err());
}

#[test]
fn test_parse_list_headers_response_wrong_code() {
    let response = NntpResponse {
        code: 480,
        message: "Authentication required".to_string(),
        lines: vec![],
    };

    assert!(commands::parse_list_headers_response(&response).is_err());
}

#[test]
fn test_parse_list_headers_response_preserves_case() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "headers supported".to_string(),
        lines: vec![
            "SUBJECT".to_string(),
            "subject".to_string(),
            "Subject".to_string(),
        ],
    };

    let headers = commands::parse_list_headers_response(&response).unwrap();
    assert_eq!(headers[0], "SUBJECT");
    assert_eq!(headers[1], "subject");
    assert_eq!(headers[2], "Subject");
}

#[test]
fn test_parse_list_headers_response_many_headers() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "headers supported".to_string(),
        lines: vec![
            "Subject".to_string(),
            "From".to_string(),
            "Date".to_string(),
            "Message-ID".to_string(),
            "References".to_string(),
            "In-Reply-To".to_string(),
            "Newsgroups".to_string(),
            "Path".to_string(),
            "Organization".to_string(),
            "User-Agent".to_string(),
            "Xref".to_string(),
            ":lines".to_string(),
            ":bytes".to_string(),
        ],
    };

    let headers = commands::parse_list_headers_response(&response).unwrap();
    assert_eq!(headers.len(), 13);
}

#[test]
fn test_parse_list_headers_response_special_characters() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "headers supported".to_string(),
        lines: vec![
            "X-Custom-Header".to_string(),
            "X-Spam-Score".to_string(),
            "Content-Type".to_string(),
        ],
    };

    let headers = commands::parse_list_headers_response(&response).unwrap();
    assert_eq!(headers.len(), 3);
    assert_eq!(headers[0], "X-Custom-Header");
    assert_eq!(headers[1], "X-Spam-Score");
}

#[test]
fn test_list_headers_real_world_msgid_example() {
    // Typical response for LIST HEADERS MSGID
    let response = NntpResponse {
        code: 215,
        message: "Headers supported for message-id".to_string(),
        lines: vec![
            "Subject".to_string(),
            "From".to_string(),
            "Date".to_string(),
            "Message-ID".to_string(),
            "References".to_string(),
            "Xref".to_string(),
        ],
    };

    let headers = commands::parse_list_headers_response(&response).unwrap();
    assert_eq!(headers.len(), 6);
    assert!(headers.contains(&"Message-ID".to_string()));
}

#[test]
fn test_list_headers_real_world_range_example() {
    // Typical response for LIST HEADERS RANGE
    let response = NntpResponse {
        code: 215,
        message: "Headers supported for range".to_string(),
        lines: vec![
            "Subject".to_string(),
            "From".to_string(),
            "Date".to_string(),
            "Message-ID".to_string(),
        ],
    };

    let headers = commands::parse_list_headers_response(&response).unwrap();
    assert_eq!(headers.len(), 4);
}

#[test]
fn test_list_active_times_command_format() {
    let cmd = commands::list_active_times("*");
    assert_eq!(cmd, "LIST ACTIVE.TIMES *\r\n");
    assert!(cmd.ends_with("\r\n"));
}

#[test]
fn test_list_active_times_command_with_wildmat() {
    let cmd = commands::list_active_times("comp.*");
    assert_eq!(cmd, "LIST ACTIVE.TIMES comp.*\r\n");
    assert!(cmd.ends_with("\r\n"));
}

#[test]
fn test_list_active_times_command_uppercase() {
    let cmd = commands::list_active_times("alt.test");
    assert!(cmd.starts_with("LIST ACTIVE.TIMES"));
    assert!(cmd.contains("alt.test"));
}

#[test]
fn test_parse_list_active_times_single_group() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec!["comp.lang.rust 1234567890 user@example.com".to_string()],
    };

    let groups = commands::parse_list_active_times_response(&response).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].name, "comp.lang.rust");
    assert_eq!(groups[0].timestamp, 1234567890);
    assert_eq!(groups[0].creator, "user@example.com");
}

#[test]
fn test_parse_list_active_times_multiple_groups() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![
            "comp.lang.rust 1234567890 user@example.com".to_string(),
            "alt.test 1234567900 admin@server.org".to_string(),
            "misc.test 9876543210 creator@host.net".to_string(),
        ],
    };

    let groups = commands::parse_list_active_times_response(&response).unwrap();
    assert_eq!(groups.len(), 3);

    assert_eq!(groups[0].name, "comp.lang.rust");
    assert_eq!(groups[0].timestamp, 1234567890);
    assert_eq!(groups[0].creator, "user@example.com");

    assert_eq!(groups[1].name, "alt.test");
    assert_eq!(groups[1].timestamp, 1234567900);
    assert_eq!(groups[1].creator, "admin@server.org");

    assert_eq!(groups[2].name, "misc.test");
    assert_eq!(groups[2].timestamp, 9876543210);
    assert_eq!(groups[2].creator, "creator@host.net");
}

#[test]
fn test_parse_list_active_times_protocol_error() {
    let response = NntpResponse {
        code: codes::NO_SUCH_GROUP,
        message: "No such newsgroup".to_string(),
        lines: vec![],
    };

    let result = commands::parse_list_active_times_response(&response);
    assert!(result.is_err());
}

#[test]
fn test_parse_list_active_times_empty_response() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![],
    };

    let groups = commands::parse_list_active_times_response(&response).unwrap();
    assert_eq!(groups.len(), 0);
}

#[test]
fn test_parse_list_active_times_malformed_line_skipped() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![
            "comp.lang.rust 1234567890 user@example.com".to_string(),
            "malformed line".to_string(), // Only 2 fields - should be skipped
            "alt.test 1234567900 admin@server.org".to_string(),
        ],
    };

    let groups = commands::parse_list_active_times_response(&response).unwrap();
    assert_eq!(groups.len(), 2); // Malformed line skipped
    assert_eq!(groups[0].name, "comp.lang.rust");
    assert_eq!(groups[1].name, "alt.test");
}

#[test]
fn test_parse_list_active_times_invalid_timestamp() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec!["comp.lang.rust invalid user@example.com".to_string()],
    };

    let groups = commands::parse_list_active_times_response(&response).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].timestamp, 0); // Invalid timestamp defaults to 0
}

#[test]
fn test_parse_list_active_times_creator_with_spaces() {
    // Note: RFC 3977 doesn't specify how to handle creators with spaces
    // In practice, this shouldn't happen, but we test the current behavior
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec!["comp.lang.rust 1234567890 user@example.com some extra data".to_string()],
    };

    let groups = commands::parse_list_active_times_response(&response).unwrap();
    assert_eq!(groups.len(), 1);
    // split_whitespace() means we only get the first token after timestamp
    assert_eq!(groups[0].creator, "user@example.com");
}

#[test]
fn test_parse_list_active_times_large_timestamp() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec!["alt.test 2147483647 admin@server.org".to_string()],
    };

    let groups = commands::parse_list_active_times_response(&response).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].timestamp, 2147483647); // Max 32-bit signed timestamp (year 2038)
}

#[test]
fn test_parse_list_active_times_zero_timestamp() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec!["alt.test 0 admin@server.org".to_string()],
    };

    let groups = commands::parse_list_active_times_response(&response).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].timestamp, 0); // Unix epoch
}

#[test]
fn test_list_active_times_real_world_example() {
    // Based on RFC 3977 Section 7.6.4 example
    let response = NntpResponse {
        code: 215,
        message: "information follows".to_string(),
        lines: vec![
            "misc.test 930445408 <katy@example.com>".to_string(),
            "alt.rfc-writers.recovery 930562309 <emv@msen.com>".to_string(),
            "tx.natives.recovery 930678923 <sob@academ.com>".to_string(),
        ],
    };

    let groups = commands::parse_list_active_times_response(&response).unwrap();
    assert_eq!(groups.len(), 3);

    assert_eq!(groups[0].name, "misc.test");
    assert_eq!(groups[0].timestamp, 930445408);
    assert_eq!(groups[0].creator, "<katy@example.com>");

    assert_eq!(groups[1].name, "alt.rfc-writers.recovery");
    assert_eq!(groups[1].timestamp, 930562309);
    assert_eq!(groups[1].creator, "<emv@msen.com>");

    assert_eq!(groups[2].name, "tx.natives.recovery");
    assert_eq!(groups[2].timestamp, 930678923);
    assert_eq!(groups[2].creator, "<sob@academ.com>");
}

#[test]
fn test_list_active_times_wrong_response_code() {
    let response = NntpResponse {
        code: 500,
        message: "Command not recognized".to_string(),
        lines: vec![],
    };

    let result = commands::parse_list_active_times_response(&response);
    assert!(result.is_err());
    match result {
        Err(NntpError::Protocol { code, .. }) => assert_eq!(code, 500),
        _ => panic!("Expected Protocol error"),
    }
}

#[test]
fn test_list_active_times_special_characters_in_group_name() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec!["alt.test-group_with.special-chars 1234567890 user@example.com".to_string()],
    };

    let groups = commands::parse_list_active_times_response(&response).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].name, "alt.test-group_with.special-chars");
}

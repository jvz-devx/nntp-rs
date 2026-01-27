//! RFC 3977 Section 7.6.6 - LIST NEWSGROUPS Command Tests
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc3977#section-7.6.6
//!
//! Tests for the LIST NEWSGROUPS command and response parsing.

use nntp_rs::{NntpResponse, codes, commands};

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

    let groups = commands::parse_list_newsgroups_response(response).unwrap();
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

    let groups = commands::parse_list_newsgroups_response(response).unwrap();
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

    let groups = commands::parse_list_newsgroups_response(response).unwrap();
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

    let groups = commands::parse_list_newsgroups_response(response).unwrap();
    assert_eq!(groups.len(), 0);
}

#[test]
fn test_parse_list_newsgroups_description_with_special_chars() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List".to_string(),
        lines: vec!["test.group Discussion about C++ & other topics (moderated)".to_string()],
    };

    let groups = commands::parse_list_newsgroups_response(response).unwrap();
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

    let result = commands::parse_list_newsgroups_response(response);
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

    let groups = commands::parse_list_newsgroups_response(response).unwrap();
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

    let groups = commands::parse_list_newsgroups_response(response).unwrap();
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

    let groups = commands::parse_list_newsgroups_response(response).unwrap();
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

    let groups = commands::parse_list_newsgroups_response(response).unwrap();
    assert_eq!(groups[0].name, "alt.binaries.multimedia.anime");
}

#[test]
fn test_list_newsgroups_long_description() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List".to_string(),
        lines: vec!["test.group This is a very long description that contains many words and explains the purpose of this newsgroup in great detail".to_string()],
    };

    let groups = commands::parse_list_newsgroups_response(response).unwrap();
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

    let groups = commands::parse_list_newsgroups_response(response).unwrap();
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

    let groups = commands::parse_list_newsgroups_response(response).unwrap();
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

    let groups = commands::parse_list_newsgroups_response(response).unwrap();
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

    let groups = commands::parse_list_newsgroups_response(response).unwrap();
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

    let groups = commands::parse_list_newsgroups_response(response).unwrap();
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

    let groups = commands::parse_list_newsgroups_response(response).unwrap();
    // Leading/trailing whitespace trimmed, but internal preserved
    assert_eq!(groups[0].description, "Multiple  spaces  preserved");
}

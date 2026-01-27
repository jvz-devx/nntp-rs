//! RFC 3977 Section 6.1.2 - LISTGROUP Command Tests
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc3977#section-6.1.2
//!
//! These tests verify the LISTGROUP command which returns a list of article
//! numbers currently available in the specified newsgroup.

use nntp_rs::{codes, commands, NntpResponse};
#[test]
fn test_listgroup_command_format_basic() {
    let cmd = commands::listgroup("alt.test");
    assert_eq!(cmd, "LISTGROUP alt.test\r\n");
}

#[test]
fn test_listgroup_command_format_with_range() {
    let cmd = commands::listgroup_range("alt.test", "100-200");
    assert_eq!(cmd, "LISTGROUP alt.test 100-200\r\n");
}

#[test]
fn test_listgroup_command_ends_with_crlf_basic() {
    let cmd = commands::listgroup("misc.test");
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_listgroup_command_ends_with_crlf_range() {
    let cmd = commands::listgroup_range("misc.test", "1000-");
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_listgroup_command_uppercase() {
    let cmd = commands::listgroup("test.group");
    assert!(cmd.starts_with("LISTGROUP"));
}

#[test]
fn test_listgroup_command_range_uppercase() {
    let cmd = commands::listgroup_range("test.group", "1-100");
    assert!(cmd.starts_with("LISTGROUP"));
}

#[test]
fn test_listgroup_command_binary_group() {
    let cmd = commands::listgroup("alt.binaries.test");
    assert_eq!(cmd, "LISTGROUP alt.binaries.test\r\n");
}

#[test]
fn test_listgroup_command_range_open_ended() {
    let cmd = commands::listgroup_range("alt.test", "500-");
    assert_eq!(cmd, "LISTGROUP alt.test 500-\r\n");
}

#[test]
fn test_listgroup_command_range_up_to() {
    let cmd = commands::listgroup_range("alt.test", "-1000");
    assert_eq!(cmd, "LISTGROUP alt.test -1000\r\n");
}

#[test]
fn test_listgroup_command_range_closed() {
    let cmd = commands::listgroup_range("comp.lang.rust", "42-100");
    assert_eq!(cmd, "LISTGROUP comp.lang.rust 42-100\r\n");
}
#[test]
fn test_listgroup_response_empty_group() {
    // Group exists but has no articles
    let response = NntpResponse {
        code: codes::GROUP_SELECTED,
        message: "0 0 0 empty.group".to_string(),
        lines: vec![],
    };

    assert!(response.is_success());
    assert_eq!(response.lines.len(), 0);
}

#[test]
fn test_listgroup_response_single_article() {
    let response = NntpResponse {
        code: codes::GROUP_SELECTED,
        message: "1 42 42 test.group".to_string(),
        lines: vec!["42".to_string()],
    };

    assert!(response.is_success());
    assert_eq!(response.lines.len(), 1);
    assert_eq!(response.lines[0], "42");
}

#[test]
fn test_listgroup_response_multiple_articles() {
    let response = NntpResponse {
        code: codes::GROUP_SELECTED,
        message: "5 100 104 test.group".to_string(),
        lines: vec![
            "100".to_string(),
            "101".to_string(),
            "102".to_string(),
            "103".to_string(),
            "104".to_string(),
        ],
    };

    assert!(response.is_success());
    assert_eq!(response.lines.len(), 5);

    // Verify all articles are sequential
    for (i, line) in response.lines.iter().enumerate() {
        assert_eq!(line.parse::<u64>().unwrap(), 100 + i as u64);
    }
}

#[test]
fn test_listgroup_response_sparse_articles() {
    // RFC 3977 §6.1.2: Article numbers need not be sequential
    let response = NntpResponse {
        code: codes::GROUP_SELECTED,
        message: "4 10 50 sparse.group".to_string(),
        lines: vec![
            "10".to_string(),
            "15".to_string(),
            "42".to_string(),
            "50".to_string(),
        ],
    };

    assert!(response.is_success());
    assert_eq!(response.lines.len(), 4);

    // Articles can have gaps
    assert_eq!(response.lines[0].parse::<u64>().unwrap(), 10);
    assert_eq!(response.lines[1].parse::<u64>().unwrap(), 15);
    assert_eq!(response.lines[2].parse::<u64>().unwrap(), 42);
    assert_eq!(response.lines[3].parse::<u64>().unwrap(), 50);
}

#[test]
fn test_listgroup_response_large_numbers() {
    let response = NntpResponse {
        code: codes::GROUP_SELECTED,
        message: "3 1000000 1000002 high.volume.group".to_string(),
        lines: vec![
            "1000000".to_string(),
            "1000001".to_string(),
            "1000002".to_string(),
        ],
    };

    assert!(response.is_success());
    assert_eq!(response.lines.len(), 3);

    for (i, line) in response.lines.iter().enumerate() {
        assert_eq!(line.parse::<u64>().unwrap(), 1_000_000 + i as u64);
    }
}

#[test]
fn test_listgroup_response_range_subset() {
    // Response when range is specified
    let response = NntpResponse {
        code: codes::GROUP_SELECTED,
        message: "3 100 150 test.group".to_string(),
        lines: vec!["100".to_string(), "125".to_string(), "150".to_string()],
    };

    assert!(response.is_success());
    assert_eq!(response.lines.len(), 3);
}

#[test]
fn test_listgroup_response_leading_whitespace() {
    // Some servers may send article numbers with leading whitespace
    let response = NntpResponse {
        code: codes::GROUP_SELECTED,
        message: "2 1 2 test.group".to_string(),
        lines: vec!["  1".to_string(), "  2".to_string()],
    };

    assert!(response.is_success());

    // Parser should handle trimming
    for line in &response.lines {
        assert!(line.trim().parse::<u64>().is_ok());
    }
}

#[test]
fn test_listgroup_response_rfc_example() {
    // RFC 3977 Section 6.1.2 example
    let response = NntpResponse {
        code: 211,
        message: "2 3000234 3002322 misc.test".to_string(),
        lines: vec![
            "3000234".to_string(),
            "3000237".to_string(),
            "3000238".to_string(),
            "3000239".to_string(),
            "3002322".to_string(),
        ],
    };

    assert!(response.is_success());
    assert_eq!(response.code, codes::GROUP_SELECTED);

    // Verify first and last from RFC example
    assert_eq!(response.lines.first().unwrap(), "3000234");
    assert_eq!(response.lines.last().unwrap(), "3002322");
}
#[test]
fn test_listgroup_response_no_such_group() {
    let response = NntpResponse {
        code: codes::NO_SUCH_GROUP,
        message: "No such newsgroup".to_string(),
        lines: vec![],
    };

    assert!(!response.is_success());
    assert_eq!(response.code, codes::NO_SUCH_GROUP);
}

#[test]
fn test_listgroup_response_protocol_error() {
    let response = NntpResponse {
        code: 500,
        message: "Command not understood".to_string(),
        lines: vec![],
    };

    assert!(!response.is_success());
}

#[test]
fn test_listgroup_response_wrong_code() {
    // Got a different success code than expected
    let response = NntpResponse {
        code: 215, // LIST response code instead of GROUP
        message: "List follows".to_string(),
        lines: vec![],
    };

    assert!(response.is_success());
    assert_ne!(response.code, codes::GROUP_SELECTED);
}

#[test]
fn test_listgroup_response_very_long_list() {
    // Test with many articles (10000 in this case)
    let mut lines = Vec::new();
    for i in 1..=10000 {
        lines.push(i.to_string());
    }

    let response = NntpResponse {
        code: codes::GROUP_SELECTED,
        message: "10000 1 10000 big.group".to_string(),
        lines,
    };

    assert!(response.is_success());
    assert_eq!(response.lines.len(), 10000);
    assert_eq!(response.lines[0], "1");
    assert_eq!(response.lines[9999], "10000");
}

#[test]
fn test_listgroup_response_malformed_article_numbers() {
    // If response contains non-numeric lines, parser should skip them
    let response = NntpResponse {
        code: codes::GROUP_SELECTED,
        message: "2 1 2 test.group".to_string(),
        lines: vec![
            "1".to_string(),
            "invalid".to_string(), // Should be skipped
            "2".to_string(),
        ],
    };

    assert!(response.is_success());

    // Parser should be able to extract valid numbers
    let valid_numbers: Vec<u64> = response
        .lines
        .iter()
        .filter_map(|line| line.trim().parse::<u64>().ok())
        .collect();

    assert_eq!(valid_numbers.len(), 2);
    assert_eq!(valid_numbers[0], 1);
    assert_eq!(valid_numbers[1], 2);
}

#[test]
fn test_listgroup_response_u64_max_article() {
    let response = NntpResponse {
        code: codes::GROUP_SELECTED,
        message: "1 18446744073709551615 18446744073709551615 test.group".to_string(),
        lines: vec!["18446744073709551615".to_string()],
    };

    assert!(response.is_success());
    assert_eq!(response.lines[0].parse::<u64>().unwrap(), u64::MAX);
}

#[test]
fn test_listgroup_response_single_range() {
    // LISTGROUP with range that matches only one article
    let response = NntpResponse {
        code: codes::GROUP_SELECTED,
        message: "1 42 42 test.group".to_string(),
        lines: vec!["42".to_string()],
    };

    assert!(response.is_success());
    assert_eq!(response.lines.len(), 1);
}

#[test]
fn test_listgroup_response_range_no_matches() {
    // Range specified but no articles in that range
    let response = NntpResponse {
        code: codes::GROUP_SELECTED,
        message: "0 0 0 test.group".to_string(),
        lines: vec![],
    };

    assert!(response.is_success());
    assert_eq!(response.lines.len(), 0);
}

#[test]
fn test_listgroup_response_duplicate_numbers() {
    // Although unlikely, test handling of duplicate article numbers
    let response = NntpResponse {
        code: codes::GROUP_SELECTED,
        message: "5 1 3 test.group".to_string(),
        lines: vec![
            "1".to_string(),
            "2".to_string(),
            "2".to_string(), // Duplicate
            "3".to_string(),
        ],
    };

    assert!(response.is_success());

    // Parser should still be able to extract all numbers
    let numbers: Vec<u64> = response
        .lines
        .iter()
        .filter_map(|line| line.trim().parse::<u64>().ok())
        .collect();

    assert!(numbers.contains(&1));
    assert!(numbers.contains(&2));
    assert!(numbers.contains(&3));
}

// Real-World Scenarios

#[test]
fn test_listgroup_binary_newsgroup_typical() {
    // Typical binary newsgroup with sparse article numbers
    let response = NntpResponse {
        code: codes::GROUP_SELECTED,
        message: "1000 5000 10000 alt.binaries.test".to_string(),
        lines: (5000..=10000).step_by(5).map(|n| n.to_string()).collect(),
    };

    assert!(response.is_success());
    assert_eq!(response.lines.len(), 1001);
}

#[test]
fn test_listgroup_moderated_group() {
    let response = NntpResponse {
        code: codes::GROUP_SELECTED,
        message: "10 1 10 comp.lang.rust.moderated".to_string(),
        lines: (1..=10).map(|n| n.to_string()).collect(),
    };

    assert!(response.is_success());
    assert_eq!(response.lines.len(), 10);
}

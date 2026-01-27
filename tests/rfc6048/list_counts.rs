//! RFC 6048 Section 3 - LIST COUNTS Tests
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc6048#section-3

use nntp_rs::{NntpResponse, codes, commands};
#[test]
fn test_list_counts_command_format() {
    let cmd = commands::list_counts("*");
    assert_eq!(cmd, "LIST COUNTS *\r\n");
}

#[test]
fn test_list_counts_with_wildmat() {
    let cmd = commands::list_counts("comp.*");
    assert_eq!(cmd, "LIST COUNTS comp.*\r\n");
}

#[test]
fn test_list_counts_ends_with_crlf() {
    let cmd = commands::list_counts("alt.binaries.*");
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_list_counts_command_uppercase() {
    let cmd = commands::list_counts("test");
    assert!(cmd.starts_with("LIST COUNTS"));
}
#[test]
fn test_parse_list_counts_response_basic() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List of newsgroups follows".to_string(),
        lines: vec![
            "comp.lang.rust 1234 1000 12345 y".to_string(),
            "alt.binaries.test 5678 2000 23456 n".to_string(),
        ],
    };

    let groups = commands::parse_list_counts_response(response).unwrap();
    assert_eq!(groups.len(), 2);

    assert_eq!(groups[0].name, "comp.lang.rust");
    assert_eq!(groups[0].count, 1234);
    assert_eq!(groups[0].low, 1000);
    assert_eq!(groups[0].high, 12345);
    assert_eq!(groups[0].status, "y");

    assert_eq!(groups[1].name, "alt.binaries.test");
    assert_eq!(groups[1].count, 5678);
    assert_eq!(groups[1].low, 2000);
    assert_eq!(groups[1].high, 23456);
    assert_eq!(groups[1].status, "n");
}

#[test]
fn test_parse_list_counts_response_moderated() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec!["news.announce.important 100 5 104 m".to_string()],
    };

    let groups = commands::parse_list_counts_response(response).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].status, "m");
}

#[test]
fn test_parse_list_counts_response_zero_count() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec!["empty.group 0 0 0 n".to_string()],
    };

    let groups = commands::parse_list_counts_response(response).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].count, 0);
    assert_eq!(groups[0].low, 0);
    assert_eq!(groups[0].high, 0);
}

#[test]
fn test_parse_list_counts_response_large_numbers() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec!["huge.group 9999999999 1000000 10000000000 y".to_string()],
    };

    let groups = commands::parse_list_counts_response(response).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].count, 9999999999);
    assert_eq!(groups[0].high, 10000000000);
}

#[test]
fn test_parse_list_counts_response_empty() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "No matching newsgroups".to_string(),
        lines: vec![],
    };

    let groups = commands::parse_list_counts_response(response).unwrap();
    assert_eq!(groups.len(), 0);
}

#[test]
fn test_parse_list_counts_response_multiple_groups() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![
            "comp.lang.c 500 100 599 y".to_string(),
            "comp.lang.c++ 750 200 949 y".to_string(),
            "comp.lang.java 1000 300 1299 m".to_string(),
            "comp.lang.python 1250 400 1649 y".to_string(),
        ],
    };

    let groups = commands::parse_list_counts_response(response).unwrap();
    assert_eq!(groups.len(), 4);
    assert_eq!(groups[0].name, "comp.lang.c");
    assert_eq!(groups[1].name, "comp.lang.c++");
    assert_eq!(groups[2].name, "comp.lang.java");
    assert_eq!(groups[3].name, "comp.lang.python");
}
#[test]
fn test_parse_list_counts_response_protocol_error() {
    let response = NntpResponse {
        code: 500,
        message: "Command not recognized".to_string(),
        lines: vec![],
    };

    let result = commands::parse_list_counts_response(response);
    assert!(result.is_err());
}

#[test]
fn test_parse_list_counts_response_malformed_line() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![
            "comp.lang.rust 1234 1000 12345 y".to_string(),
            "malformed line".to_string(), // Should be skipped
            "alt.test 100 50 149 n".to_string(),
        ],
    };

    let groups = commands::parse_list_counts_response(response).unwrap();
    assert_eq!(groups.len(), 2); // Malformed line skipped
    assert_eq!(groups[0].name, "comp.lang.rust");
    assert_eq!(groups[1].name, "alt.test");
}

#[test]
fn test_parse_list_counts_response_missing_fields() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![
            "comp.lang.rust 1234 1000 12345 y".to_string(),
            "incomplete.group 100 50".to_string(), // Missing high and status
            "alt.test 200 100 299 m".to_string(),
        ],
    };

    let groups = commands::parse_list_counts_response(response).unwrap();
    assert_eq!(groups.len(), 2); // Line with missing fields skipped
    assert_eq!(groups[0].name, "comp.lang.rust");
    assert_eq!(groups[1].name, "alt.test");
}

#[test]
fn test_parse_list_counts_response_wrong_success_code() {
    let response = NntpResponse {
        code: 200, // Wrong success code
        message: "OK".to_string(),
        lines: vec!["comp.lang.rust 1234 1000 12345 y".to_string()],
    };

    // Should still parse because is_success() checks 2xx
    let groups = commands::parse_list_counts_response(response).unwrap();
    assert_eq!(groups.len(), 1);
}

#[test]
fn test_parse_list_counts_response_extra_whitespace() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec!["comp.lang.rust  1234  1000  12345  y".to_string()],
    };

    let groups = commands::parse_list_counts_response(response).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].count, 1234);
}

#[test]
fn test_parse_list_counts_response_invalid_numbers() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![
            "good.group 100 50 149 y".to_string(),
            "bad.group abc def ghi n".to_string(), // Invalid numbers
            "another.group 200 100 299 m".to_string(),
        ],
    };

    let groups = commands::parse_list_counts_response(response).unwrap();
    assert_eq!(groups.len(), 3); // Bad numbers become 0
    assert_eq!(groups[0].name, "good.group");
    assert_eq!(groups[1].name, "bad.group");
    assert_eq!(groups[1].count, 0);
    assert_eq!(groups[1].low, 0);
    assert_eq!(groups[1].high, 0);
}

#[test]
fn test_parse_list_counts_response_group_names_with_dots() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![
            "alt.binaries.multimedia.erotica 5000 1000 5999 n".to_string(),
            "comp.lang.c++.moderated 300 100 399 m".to_string(),
        ],
    };

    let groups = commands::parse_list_counts_response(response).unwrap();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].name, "alt.binaries.multimedia.erotica");
    assert_eq!(groups[1].name, "comp.lang.c++.moderated");
}

// Real-world Scenarios

#[test]
fn test_list_counts_binary_newsgroup() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![
            "alt.binaries.hdtv.x264 150000 50000 199999 n".to_string(),
            "alt.binaries.movies 200000 100000 299999 n".to_string(),
        ],
    };

    let groups = commands::parse_list_counts_response(response).unwrap();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].count, 150000);
    assert_eq!(groups[1].count, 200000);
}

#[test]
fn test_list_counts_text_newsgroup() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![
            "comp.lang.rust 450 100 549 y".to_string(),
            "news.announce.newusers 25 1 25 m".to_string(),
        ],
    };

    let groups = commands::parse_list_counts_response(response).unwrap();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].status, "y");
    assert_eq!(groups[1].status, "m");
}

#[test]
fn test_list_counts_count_vs_range() {
    // Count might differ from (high - low) due to deleted articles
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![
            // Count is 800, but high-low = 999 (199 deleted articles)
            "sparse.group 800 1000 1999 y".to_string(),
        ],
    };

    let groups = commands::parse_list_counts_response(response).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].count, 800);
    assert_eq!(groups[0].low, 1000);
    assert_eq!(groups[0].high, 1999);
    // Verify count != high - low
    let range_size = groups[0].high - groups[0].low;
    assert_ne!(groups[0].count, range_size);
}

#[test]
fn test_rfc6048_example() {
    // Example from RFC 6048 Section 3
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "information follows".to_string(),
        lines: vec![
            "misc.test 3002322 3000234 3002600 y".to_string(),
            "comp.risks 442001 1 442001 m".to_string(),
            "alt.rfc-writers.recovery 4 1 4 y".to_string(),
        ],
    };

    let groups = commands::parse_list_counts_response(response).unwrap();
    assert_eq!(groups.len(), 3);

    // misc.test has estimated count higher than article range (sparse articles)
    assert_eq!(groups[0].name, "misc.test");
    assert_eq!(groups[0].count, 3002322);
    assert_eq!(groups[0].low, 3000234);
    assert_eq!(groups[0].high, 3002600);

    // comp.risks has count equal to high (no gaps, starts at 1)
    assert_eq!(groups[1].name, "comp.risks");
    assert_eq!(groups[1].count, 442001);
    assert_eq!(groups[1].low, 1);
    assert_eq!(groups[1].high, 442001);

    // alt.rfc-writers.recovery is a small group
    assert_eq!(groups[2].name, "alt.rfc-writers.recovery");
    assert_eq!(groups[2].count, 4);
    assert_eq!(groups[2].low, 1);
    assert_eq!(groups[2].high, 4);
}

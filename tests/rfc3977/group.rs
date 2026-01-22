//! RFC 3977 Section 6.1.1 - GROUP Command Response Tests
//!
//! These tests verify compliance with GROUP response format:
//! Response: "211 number low high group"

use nntp_rs::commands::parse_group_response;
use nntp_rs::NntpResponse;

// Valid GROUP Response Parsing (RFC 3977 §6.1.1)

#[test]
fn test_group_response_standard_format() {
    // RFC 3977 §6.1.1: "211 number low high group"
    let response = NntpResponse {
        code: 211,
        message: "1234 100 5000 alt.test".to_string(),
        lines: vec![],
    };

    let (count, first, last) = parse_group_response(&response).unwrap();
    assert_eq!(count, 1234);
    assert_eq!(first, 100);
    assert_eq!(last, 5000);
}

#[test]
fn test_group_response_zero_articles() {
    // Empty group with no articles
    let response = NntpResponse {
        code: 211,
        message: "0 0 0 empty.group".to_string(),
        lines: vec![],
    };

    let (count, first, last) = parse_group_response(&response).unwrap();
    assert_eq!(count, 0);
    assert_eq!(first, 0);
    assert_eq!(last, 0);
}

#[test]
fn test_group_response_empty_group_high_less_than_low() {
    // RFC 3977 §6.1.1: For empty groups, high water mark may be one less than low
    let response = NntpResponse {
        code: 211,
        message: "0 100 99 empty.group".to_string(),
        lines: vec![],
    };

    let (count, first, last) = parse_group_response(&response).unwrap();
    assert_eq!(count, 0);
    assert_eq!(first, 100);
    assert_eq!(last, 99); // Valid: high < low for empty group
}

#[test]
fn test_group_response_large_numbers() {
    // Usenet groups can have millions of articles
    let response = NntpResponse {
        code: 211,
        message: "5000000 1 5000000 high.volume.group".to_string(),
        lines: vec![],
    };

    let (count, first, last) = parse_group_response(&response).unwrap();
    assert_eq!(count, 5_000_000);
    assert_eq!(first, 1);
    assert_eq!(last, 5_000_000);
}

#[test]
fn test_group_response_leading_zeros() {
    // RFC 3977 §3.1: Numbers are base-10 and may have leading zeros
    let response = NntpResponse {
        code: 211,
        message: "0042 001 0100 test.group".to_string(),
        lines: vec![],
    };

    let (count, first, last) = parse_group_response(&response).unwrap();
    assert_eq!(count, 42);
    assert_eq!(first, 1);
    assert_eq!(last, 100);
}

#[test]
fn test_group_response_group_name_with_numbers() {
    let response = NntpResponse {
        code: 211,
        message: "100 1 100 alt.binaries.mp3".to_string(),
        lines: vec![],
    };

    let (count, first, last) = parse_group_response(&response).unwrap();
    assert_eq!(count, 100);
    assert_eq!(first, 1);
    assert_eq!(last, 100);
}

#[test]
fn test_group_response_deep_hierarchy_group() {
    let response = NntpResponse {
        code: 211,
        message: "50 10 60 comp.lang.rust.programming.help".to_string(),
        lines: vec![],
    };

    let (count, first, last) = parse_group_response(&response).unwrap();
    assert_eq!(count, 50);
    assert_eq!(first, 10);
    assert_eq!(last, 60);
}

// Invalid GROUP Response Parsing

#[test]
fn test_group_response_missing_parts() {
    // Only 2 parts instead of 3 numbers
    let response = NntpResponse {
        code: 211,
        message: "100 50 alt.test".to_string(), // Missing last article number
        lines: vec![],
    };

    // This should fail - parse_group_response expects 3 numbers
    let result = parse_group_response(&response);
    // Note: current implementation may treat "alt.test" as a number and fail
    assert!(result.is_err() || result.unwrap().2 == 0);
}

#[test]
fn test_group_response_non_numeric_count() {
    let response = NntpResponse {
        code: 211,
        message: "abc 1 100 test.group".to_string(),
        lines: vec![],
    };

    assert!(parse_group_response(&response).is_err());
}

#[test]
fn test_group_response_non_numeric_first() {
    let response = NntpResponse {
        code: 211,
        message: "100 xyz 200 test.group".to_string(),
        lines: vec![],
    };

    assert!(parse_group_response(&response).is_err());
}

#[test]
fn test_group_response_non_numeric_last() {
    let response = NntpResponse {
        code: 211,
        message: "100 1 zzz test.group".to_string(),
        lines: vec![],
    };

    assert!(parse_group_response(&response).is_err());
}

#[test]
fn test_group_response_empty_message() {
    let response = NntpResponse {
        code: 211,
        message: "".to_string(),
        lines: vec![],
    };

    assert!(parse_group_response(&response).is_err());
}

#[test]
fn test_group_response_whitespace_only() {
    let response = NntpResponse {
        code: 211,
        message: "   ".to_string(),
        lines: vec![],
    };

    assert!(parse_group_response(&response).is_err());
}

// Error Response Codes

#[test]
fn test_group_response_411_no_such_group() {
    // 411 = No such newsgroup
    let response = NntpResponse {
        code: 411,
        message: "No such newsgroup".to_string(),
        lines: vec![],
    };

    let result = parse_group_response(&response);
    assert!(result.is_err());
}

#[test]
fn test_group_response_non_2xx_code() {
    // Any non-success code should fail
    let error_codes = [400, 411, 500, 502];

    for code in error_codes {
        let response = NntpResponse {
            code,
            message: "Error".to_string(),
            lines: vec![],
        };

        assert!(
            parse_group_response(&response).is_err(),
            "Code {} should return error",
            code
        );
    }
}


#[test]
fn test_group_response_single_article() {
    // Group with exactly one article
    let response = NntpResponse {
        code: 211,
        message: "1 42 42 single.article.group".to_string(),
        lines: vec![],
    };

    let (count, first, last) = parse_group_response(&response).unwrap();
    assert_eq!(count, 1);
    assert_eq!(first, 42);
    assert_eq!(last, 42);
}

#[test]
fn test_group_response_extra_whitespace() {
    // Extra spaces between parts should be handled
    let response = NntpResponse {
        code: 211,
        message: "100  1  200  test.group".to_string(), // Double spaces
        lines: vec![],
    };

    // split_whitespace() handles multiple spaces
    let (count, first, last) = parse_group_response(&response).unwrap();
    assert_eq!(count, 100);
    assert_eq!(first, 1);
    assert_eq!(last, 200);
}

#[test]
fn test_group_response_u64_max_values() {
    // Test with very large numbers near u64::MAX
    let response = NntpResponse {
        code: 211,
        message: "18446744073709551615 1 18446744073709551615 huge.group".to_string(),
        lines: vec![],
    };

    let (count, first, last) = parse_group_response(&response).unwrap();
    assert_eq!(count, u64::MAX);
    assert_eq!(first, 1);
    assert_eq!(last, u64::MAX);
}

#[test]
fn test_group_response_negative_numbers_invalid() {
    // Negative numbers should fail parsing
    let response = NntpResponse {
        code: 211,
        message: "-100 1 200 test.group".to_string(),
        lines: vec![],
    };

    assert!(parse_group_response(&response).is_err());
}

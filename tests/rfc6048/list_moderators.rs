//! RFC 6048 Section 5 - LIST MODERATORS Tests
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc6048#section-5

use nntp_rs::{NntpResponse, codes, commands};
#[test]
fn test_list_moderators_command_format() {
    let cmd = commands::list_moderators();
    assert_eq!(cmd, "LIST MODERATORS\r\n");
}

#[test]
fn test_list_moderators_ends_with_crlf() {
    let cmd = commands::list_moderators();
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_list_moderators_command_uppercase() {
    let cmd = commands::list_moderators();
    assert!(cmd.starts_with("LIST MODERATORS"));
}

#[test]
fn test_list_moderators_no_arguments() {
    let cmd = commands::list_moderators();
    // Should be exactly "LIST MODERATORS\r\n" with no arguments
    assert_eq!(cmd, "LIST MODERATORS\r\n");
}
#[test]
fn test_parse_list_moderators_response_basic() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List of submission address templates follows".to_string(),
        lines: vec![
            "foo.bar:announce@example.com".to_string(),
            "local.*:%s@localhost".to_string(),
        ],
    };

    let moderators = commands::parse_list_moderators_response(response).unwrap();
    assert_eq!(moderators.len(), 2);

    assert_eq!(moderators[0].pattern, "foo.bar");
    assert_eq!(moderators[0].address, "announce@example.com");

    assert_eq!(moderators[1].pattern, "local.*");
    assert_eq!(moderators[1].address, "%s@localhost");
}

#[test]
fn test_parse_list_moderators_response_empty() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![],
    };

    let moderators = commands::parse_list_moderators_response(response).unwrap();
    assert_eq!(moderators.len(), 0);
}

#[test]
fn test_parse_list_moderators_response_single() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec!["alt.test:test@moderators.example.com".to_string()],
    };

    let moderators = commands::parse_list_moderators_response(response).unwrap();
    assert_eq!(moderators.len(), 1);
    assert_eq!(moderators[0].pattern, "alt.test");
    assert_eq!(moderators[0].address, "test@moderators.example.com");
}

#[test]
fn test_parse_list_moderators_response_multiple() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Moderators list follows".to_string(),
        lines: vec![
            "foo.bar:announce@example.com".to_string(),
            "local.*:%s@localhost".to_string(),
            "*:%s@moderators.example.com".to_string(),
        ],
    };

    let moderators = commands::parse_list_moderators_response(response).unwrap();
    assert_eq!(moderators.len(), 3);

    assert_eq!(moderators[0].pattern, "foo.bar");
    assert_eq!(moderators[0].address, "announce@example.com");

    assert_eq!(moderators[1].pattern, "local.*");
    assert_eq!(moderators[1].address, "%s@localhost");

    assert_eq!(moderators[2].pattern, "*");
    assert_eq!(moderators[2].address, "%s@moderators.example.com");
}

#[test]
fn test_parse_list_moderators_response_with_percent_s() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![
            "comp.*:%s@comp-moderators.example.com".to_string(),
            "news.*:%s@news-moderators.example.com".to_string(),
        ],
    };

    let moderators = commands::parse_list_moderators_response(response).unwrap();
    assert_eq!(moderators.len(), 2);
    assert_eq!(moderators[0].pattern, "comp.*");
    assert_eq!(moderators[0].address, "%s@comp-moderators.example.com");
    assert_eq!(moderators[1].pattern, "news.*");
    assert_eq!(moderators[1].address, "%s@news-moderators.example.com");
}

#[test]
fn test_parse_list_moderators_response_with_double_percent() {
    // %% represents a literal % character
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec!["test.group:admin%%support@example.com".to_string()],
    };

    let moderators = commands::parse_list_moderators_response(response).unwrap();
    assert_eq!(moderators.len(), 1);
    assert_eq!(moderators[0].pattern, "test.group");
    assert_eq!(moderators[0].address, "admin%%support@example.com");
}

#[test]
fn test_parse_list_moderators_response_wildmat_patterns() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![
            "comp.lang.*:%s@comp-lang-moderators.example.com".to_string(),
            "alt.binaries.*:%s@alt-binaries.example.com".to_string(),
            "*:%s@default-moderators.example.com".to_string(),
        ],
    };

    let moderators = commands::parse_list_moderators_response(response).unwrap();
    assert_eq!(moderators.len(), 3);

    // Verify wildmat patterns are preserved
    assert_eq!(moderators[0].pattern, "comp.lang.*");
    assert_eq!(moderators[1].pattern, "alt.binaries.*");
    assert_eq!(moderators[2].pattern, "*");
}
#[test]
fn test_parse_list_moderators_response_protocol_error() {
    let response = NntpResponse {
        code: 500,
        message: "Command not understood".to_string(),
        lines: vec![],
    };

    let result = commands::parse_list_moderators_response(response);
    assert!(result.is_err());
}

#[test]
fn test_parse_list_moderators_response_wrong_code() {
    let response = NntpResponse {
        code: 200,
        message: "OK".to_string(),
        lines: vec!["foo.bar:announce@example.com".to_string()],
    };

    // Should still parse successfully as it's a 2xx code
    let moderators = commands::parse_list_moderators_response(response).unwrap();
    assert_eq!(moderators.len(), 1);
}

#[test]
fn test_parse_list_moderators_response_malformed_no_colon() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![
            "foo.bar:announce@example.com".to_string(),
            "malformed_line_without_colon".to_string(), // This should be skipped
            "local.*:%s@localhost".to_string(),
        ],
    };

    let moderators = commands::parse_list_moderators_response(response).unwrap();
    // Should skip the malformed line
    assert_eq!(moderators.len(), 2);
    assert_eq!(moderators[0].pattern, "foo.bar");
    assert_eq!(moderators[1].pattern, "local.*");
}

#[test]
fn test_parse_list_moderators_response_address_with_colon() {
    // IPv6 addresses or URLs with colons
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec!["test.group:http://moderator.example.com:8080/submit".to_string()],
    };

    let moderators = commands::parse_list_moderators_response(response).unwrap();
    assert_eq!(moderators.len(), 1);
    assert_eq!(moderators[0].pattern, "test.group");
    // Everything after the first colon should be the address
    assert_eq!(
        moderators[0].address,
        "http://moderator.example.com:8080/submit"
    );
}

#[test]
fn test_parse_list_moderators_response_empty_pattern() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![":announce@example.com".to_string()],
    };

    let moderators = commands::parse_list_moderators_response(response).unwrap();
    assert_eq!(moderators.len(), 1);
    assert_eq!(moderators[0].pattern, "");
    assert_eq!(moderators[0].address, "announce@example.com");
}

#[test]
fn test_parse_list_moderators_response_empty_address() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec!["foo.bar:".to_string()],
    };

    let moderators = commands::parse_list_moderators_response(response).unwrap();
    assert_eq!(moderators.len(), 1);
    assert_eq!(moderators[0].pattern, "foo.bar");
    assert_eq!(moderators[0].address, "");
}

#[test]
fn test_parse_list_moderators_response_complex_addresses() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![
            "test.1:%s-moderator@example.com".to_string(),
            "test.2:moderator+%s@example.com".to_string(),
            "test.3:%s@sub.moderators.example.com".to_string(),
        ],
    };

    let moderators = commands::parse_list_moderators_response(response).unwrap();
    assert_eq!(moderators.len(), 3);
    assert_eq!(moderators[0].address, "%s-moderator@example.com");
    assert_eq!(moderators[1].address, "moderator+%s@example.com");
    assert_eq!(moderators[2].address, "%s@sub.moderators.example.com");
}

// Real-World Scenarios

#[test]
fn test_parse_list_moderators_response_rfc_example() {
    // Example from RFC 6048 Section 5
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List of submission address templates follows".to_string(),
        lines: vec![
            "foo.bar:announce@example.com".to_string(),
            "local.*:%s@localhost".to_string(),
            "*:%s@moderators.example.com".to_string(),
        ],
    };

    let moderators = commands::parse_list_moderators_response(response).unwrap();
    assert_eq!(moderators.len(), 3);

    // Verify specific patterns from RFC example
    assert_eq!(moderators[0].pattern, "foo.bar");
    assert_eq!(moderators[0].address, "announce@example.com");

    assert_eq!(moderators[1].pattern, "local.*");
    assert_eq!(moderators[1].address, "%s@localhost");

    assert_eq!(moderators[2].pattern, "*");
    assert_eq!(moderators[2].address, "%s@moderators.example.com");
}

#[test]
fn test_parse_list_moderators_response_ordering_matters() {
    // The order matters - first matching line is used
    // More specific patterns should come before general ones
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![
            "comp.lang.rust:rust-moderators@example.com".to_string(), // Most specific
            "comp.lang.*:%s@comp-lang.example.com".to_string(),       // Less specific
            "comp.*:%s@comp.example.com".to_string(),                 // Even less specific
            "*:%s@default.example.com".to_string(),                   // Least specific (catch-all)
        ],
    };

    let moderators = commands::parse_list_moderators_response(response).unwrap();
    assert_eq!(moderators.len(), 4);

    // Verify the ordering is preserved
    assert_eq!(moderators[0].pattern, "comp.lang.rust");
    assert_eq!(moderators[1].pattern, "comp.lang.*");
    assert_eq!(moderators[2].pattern, "comp.*");
    assert_eq!(moderators[3].pattern, "*");
}

#[test]
fn test_parse_list_moderators_response_typical_server() {
    // Typical server response with common patterns
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Moderators list follows".to_string(),
        lines: vec![
            "news.announce.newusers:announce-newusers@news.example.com".to_string(),
            "alt.binaries.sounds.mp3.*:mp3-moderators@alt.example.com".to_string(),
            "comp.lang.c++.moderated:cplusplus@comp.example.com".to_string(),
            "misc.*:%s@misc-moderators.example.com".to_string(),
        ],
    };

    let moderators = commands::parse_list_moderators_response(response).unwrap();
    assert_eq!(moderators.len(), 4);

    // Verify newsgroup names with special characters
    assert!(moderators[0].pattern.contains('.'));
    assert!(moderators[1].pattern.contains("*"));
    assert!(moderators[2].pattern.contains("++"));
}

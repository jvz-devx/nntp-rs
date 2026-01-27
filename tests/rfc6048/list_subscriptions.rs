//! RFC 6048 Section 7 - LIST SUBSCRIPTIONS Tests
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc6048#section-7

use nntp_rs::{NntpResponse, codes, commands};
#[test]
fn test_list_subscriptions_command_format() {
    let cmd = commands::list_subscriptions();
    assert_eq!(cmd, "LIST SUBSCRIPTIONS\r\n");
}

#[test]
fn test_list_subscriptions_ends_with_crlf() {
    let cmd = commands::list_subscriptions();
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_list_subscriptions_command_uppercase() {
    let cmd = commands::list_subscriptions();
    assert!(cmd.starts_with("LIST SUBSCRIPTIONS"));
}

#[test]
fn test_list_subscriptions_no_arguments() {
    let cmd = commands::list_subscriptions();
    // Should be exactly "LIST SUBSCRIPTIONS\r\n" with no arguments
    assert_eq!(cmd, "LIST SUBSCRIPTIONS\r\n");
}
#[test]
fn test_parse_list_subscriptions_response_basic() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Default subscription list follows".to_string(),
        lines: vec![
            "comp.lang.rust".to_string(),
            "comp.programming".to_string(),
            "news.announce.newusers".to_string(),
        ],
    };

    let subs = commands::parse_list_subscriptions_response(response).unwrap();
    assert_eq!(subs.len(), 3);
    assert_eq!(subs[0], "comp.lang.rust");
    assert_eq!(subs[1], "comp.programming");
    assert_eq!(subs[2], "news.announce.newusers");
}

#[test]
fn test_parse_list_subscriptions_response_empty() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Subscription list follows".to_string(),
        lines: vec![],
    };

    let subs = commands::parse_list_subscriptions_response(response).unwrap();
    assert_eq!(subs.len(), 0);
}

#[test]
fn test_parse_list_subscriptions_response_single_group() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec!["news.announce.important".to_string()],
    };

    let subs = commands::parse_list_subscriptions_response(response).unwrap();
    assert_eq!(subs.len(), 1);
    assert_eq!(subs[0], "news.announce.important");
}

#[test]
fn test_parse_list_subscriptions_response_multiple_groups() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Subscriptions".to_string(),
        lines: vec![
            "comp.lang.c".to_string(),
            "comp.lang.python".to_string(),
            "comp.lang.javascript".to_string(),
            "comp.os.linux.announce".to_string(),
            "news.announce.newusers".to_string(),
            "news.answers".to_string(),
        ],
    };

    let subs = commands::parse_list_subscriptions_response(response).unwrap();
    assert_eq!(subs.len(), 6);
    assert_eq!(subs[0], "comp.lang.c");
    assert_eq!(subs[5], "news.answers");
}

#[test]
fn test_parse_list_subscriptions_filters_empty_lines() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List".to_string(),
        lines: vec![
            "comp.lang.rust".to_string(),
            "".to_string(), // Empty line should be filtered
            "news.announce.newusers".to_string(),
            "".to_string(), // Another empty line
        ],
    };

    let subs = commands::parse_list_subscriptions_response(response).unwrap();
    assert_eq!(subs.len(), 2);
    assert_eq!(subs[0], "comp.lang.rust");
    assert_eq!(subs[1], "news.announce.newusers");
}

#[test]
fn test_parse_list_subscriptions_hierarchical_names() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Subscriptions".to_string(),
        lines: vec![
            "alt.test".to_string(),
            "local.general".to_string(),
            "de.comp.lang.rust".to_string(),
            "fr.comp.lang.python".to_string(),
        ],
    };

    let subs = commands::parse_list_subscriptions_response(response).unwrap();
    assert_eq!(subs.len(), 4);
    assert!(subs.contains(&"de.comp.lang.rust".to_string()));
    assert!(subs.contains(&"local.general".to_string()));
}
#[test]
fn test_parse_list_subscriptions_protocol_error() {
    let response = NntpResponse {
        code: 500,
        message: "Command not recognized".to_string(),
        lines: vec![],
    };

    let result = commands::parse_list_subscriptions_response(response);
    assert!(result.is_err());
}

#[test]
fn test_parse_list_subscriptions_wrong_success_code() {
    let response = NntpResponse {
        code: 200,
        message: "OK".to_string(),
        lines: vec!["comp.test".to_string()],
    };

    let result = commands::parse_list_subscriptions_response(response);
    // Should succeed since code 200 is 2xx (success range)
    assert!(result.is_ok());
}

#[test]
fn test_parse_list_subscriptions_error_code_401() {
    let response = NntpResponse {
        code: 401,
        message: "Permission denied".to_string(),
        lines: vec![],
    };

    let result = commands::parse_list_subscriptions_response(response);
    assert!(result.is_err());
}

#[test]
fn test_parse_list_subscriptions_error_code_502() {
    let response = NntpResponse {
        code: 502,
        message: "Command not supported".to_string(),
        lines: vec![],
    };

    let result = commands::parse_list_subscriptions_response(response);
    assert!(result.is_err());
}

#[test]
fn test_parse_list_subscriptions_very_long_list() {
    let mut lines = Vec::new();
    for i in 0..500 {
        lines.push(format!("group.hierarchy.{}", i));
    }

    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Long subscription list".to_string(),
        lines,
    };

    let subs = commands::parse_list_subscriptions_response(response).unwrap();
    assert_eq!(subs.len(), 500);
    assert_eq!(subs[0], "group.hierarchy.0");
    assert_eq!(subs[499], "group.hierarchy.499");
}

#[test]
fn test_parse_list_subscriptions_long_group_names() {
    let long_name = format!("alt.{}", "very.".repeat(20).trim_end_matches('.'));
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List".to_string(),
        lines: vec![long_name.clone()],
    };

    let subs = commands::parse_list_subscriptions_response(response).unwrap();
    assert_eq!(subs.len(), 1);
    assert_eq!(subs[0], long_name);
}

#[test]
fn test_parse_list_subscriptions_special_characters() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Subscriptions".to_string(),
        lines: vec![
            "alt.test-group".to_string(),
            "alt.test_group".to_string(),
            "alt.test+group".to_string(),
        ],
    };

    let subs = commands::parse_list_subscriptions_response(response).unwrap();
    assert_eq!(subs.len(), 3);
    assert!(subs.contains(&"alt.test-group".to_string()));
    assert!(subs.contains(&"alt.test_group".to_string()));
    assert!(subs.contains(&"alt.test+group".to_string()));
}

#[test]
fn test_parse_list_subscriptions_all_empty_lines() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Empty list".to_string(),
        lines: vec!["".to_string(), "".to_string(), "".to_string()],
    };

    let subs = commands::parse_list_subscriptions_response(response).unwrap();
    assert_eq!(subs.len(), 0);
}

// Real-World Scenarios

#[test]
fn test_list_subscriptions_rfc_6048_section_7_example() {
    // Example based on RFC 6048 Section 7
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Default subscription list follows".to_string(),
        lines: vec![
            "news.announce.newusers".to_string(),
            "news.answers".to_string(),
            "comp.lang.c".to_string(),
        ],
    };

    let subs = commands::parse_list_subscriptions_response(response).unwrap();
    assert_eq!(subs.len(), 3);
    assert_eq!(subs[0], "news.announce.newusers");
    assert_eq!(subs[1], "news.answers");
    assert_eq!(subs[2], "comp.lang.c");
}

#[test]
fn test_list_subscriptions_typical_server_response() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Recommended newsgroups".to_string(),
        lines: vec![
            "news.announce.important".to_string(),
            "news.announce.newusers".to_string(),
            "news.groups".to_string(),
            "news.admin.misc".to_string(),
            "comp.lang.python".to_string(),
            "comp.lang.javascript".to_string(),
            "comp.os.linux.misc".to_string(),
            "alt.usage.english".to_string(),
        ],
    };

    let subs = commands::parse_list_subscriptions_response(response).unwrap();
    assert_eq!(subs.len(), 8);
    assert!(subs.contains(&"news.announce.important".to_string()));
    assert!(subs.contains(&"comp.lang.python".to_string()));
    assert!(subs.contains(&"alt.usage.english".to_string()));
}

#[test]
fn test_list_subscriptions_regional_server() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Regional subscription list".to_string(),
        lines: vec![
            "local.announce".to_string(),
            "local.general".to_string(),
            "local.test".to_string(),
            "de.comp.lang.c".to_string(),
            "de.newusers".to_string(),
        ],
    };

    let subs = commands::parse_list_subscriptions_response(response).unwrap();
    assert_eq!(subs.len(), 5);
    assert_eq!(subs[0], "local.announce");
    assert_eq!(subs[4], "de.newusers");
}

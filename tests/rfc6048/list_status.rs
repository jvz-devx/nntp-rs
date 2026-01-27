//! RFC 6048 Section 3 - Extended LIST ACTIVE Status Values Tests
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc6048#section-3

use nntp_rs::{codes, commands, NntpResponse};
#[test]
fn test_status_j_junk_group() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List of newsgroups follows".to_string(),
        lines: vec!["alt.binaries.spam 0 0 j".to_string()],
    };

    let groups = commands::parse_list_active_response(response).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].name, "alt.binaries.spam");
    assert_eq!(groups[0].status, "j");
}

#[test]
fn test_status_x_no_local_posting() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List of newsgroups follows".to_string(),
        lines: vec!["comp.lang.c 12345 1000 x".to_string()],
    };

    let groups = commands::parse_list_active_response(response).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].name, "comp.lang.c");
    assert_eq!(groups[0].high, 12345);
    assert_eq!(groups[0].low, 1000);
    assert_eq!(groups[0].status, "x");
}

#[test]
fn test_status_alias_simple() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List of newsgroups follows".to_string(),
        lines: vec!["comp.lang.c++ 100 1 =comp.lang.cplusplus".to_string()],
    };

    let groups = commands::parse_list_active_response(response).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].name, "comp.lang.c++");
    assert_eq!(groups[0].high, 100);
    assert_eq!(groups[0].low, 1);
    assert_eq!(groups[0].status, "=comp.lang.cplusplus");
}

#[test]
fn test_status_alias_complex_name() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List of newsgroups follows".to_string(),
        lines: vec!["alt.test.old 50 1 =alt.test.new.hierarchical".to_string()],
    };

    let groups = commands::parse_list_active_response(response).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].status, "=alt.test.new.hierarchical");
}

#[test]
fn test_mixed_status_values() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List of newsgroups follows".to_string(),
        lines: vec![
            "comp.lang.rust 12345 1000 y".to_string(),
            "comp.lang.python 54321 2000 m".to_string(),
            "alt.binaries.test 0 0 n".to_string(),
            "alt.binaries.spam 0 0 j".to_string(),
            "comp.lang.c 1000 1 x".to_string(),
            "comp.lang.c++ 100 1 =comp.lang.cplusplus".to_string(),
        ],
    };

    let groups = commands::parse_list_active_response(response).unwrap();
    assert_eq!(groups.len(), 6);

    assert_eq!(groups[0].name, "comp.lang.rust");
    assert_eq!(groups[0].status, "y");

    assert_eq!(groups[1].name, "comp.lang.python");
    assert_eq!(groups[1].status, "m");

    assert_eq!(groups[2].name, "alt.binaries.test");
    assert_eq!(groups[2].status, "n");

    assert_eq!(groups[3].name, "alt.binaries.spam");
    assert_eq!(groups[3].status, "j");

    assert_eq!(groups[4].name, "comp.lang.c");
    assert_eq!(groups[4].status, "x");

    assert_eq!(groups[5].name, "comp.lang.c++");
    assert_eq!(groups[5].status, "=comp.lang.cplusplus");
}
#[test]
fn test_newgroups_extended_status() {
    let response = NntpResponse {
        code: codes::NEW_NEWSGROUPS_FOLLOW,
        message: "New newsgroups follow".to_string(),
        lines: vec![
            "alt.test.junk 0 0 j".to_string(),
            "comp.test.nolocalpost 10 1 x".to_string(),
            "misc.test.alias 5 1 =misc.test.canonical".to_string(),
        ],
    };

    let groups = commands::parse_newgroups_response(response).unwrap();
    assert_eq!(groups.len(), 3);

    assert_eq!(groups[0].status, "j");
    assert_eq!(groups[1].status, "x");
    assert_eq!(groups[2].status, "=misc.test.canonical");
}
#[test]
fn test_list_counts_extended_status() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List of newsgroups follows".to_string(),
        lines: vec![
            "alt.binaries.spam 0 0 0 j".to_string(),
            "comp.lang.c 100 1000 12345 x".to_string(),
            "comp.lang.c++ 50 1 100 =comp.lang.cplusplus".to_string(),
        ],
    };

    let groups = commands::parse_list_counts_response(response).unwrap();
    assert_eq!(groups.len(), 3);

    assert_eq!(groups[0].name, "alt.binaries.spam");
    assert_eq!(groups[0].count, 0);
    assert_eq!(groups[0].status, "j");

    assert_eq!(groups[1].name, "comp.lang.c");
    assert_eq!(groups[1].count, 100);
    assert_eq!(groups[1].status, "x");

    assert_eq!(groups[2].name, "comp.lang.c++");
    assert_eq!(groups[2].count, 50);
    assert_eq!(groups[2].status, "=comp.lang.cplusplus");
}
#[test]
fn test_rfc6048_section_3_junk_example() {
    // RFC 6048 Section 3 discusses "j" status for junk/spam groups
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List of newsgroups follows".to_string(),
        lines: vec!["junk 0 0 j".to_string()],
    };

    let groups = commands::parse_list_active_response(response).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].name, "junk");
    assert_eq!(groups[0].status, "j");
}

#[test]
fn test_rfc6048_section_3_alias_example() {
    // RFC 6048 Section 3 discusses "=" status for aliases
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List of newsgroups follows".to_string(),
        lines: vec!["alt.test 0 0 =alt.test.moderated".to_string()],
    };

    let groups = commands::parse_list_active_response(response).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].status, "=alt.test.moderated");
}
#[test]
fn test_backward_compatibility_single_char_status() {
    // Ensure original single-character status values still work
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List of newsgroups follows".to_string(),
        lines: vec![
            "comp.lang.rust 12345 1000 y".to_string(),
            "comp.lang.python 54321 2000 n".to_string(),
            "comp.lang.go 99999 3000 m".to_string(),
        ],
    };

    let groups = commands::parse_list_active_response(response).unwrap();
    assert_eq!(groups.len(), 3);
    assert_eq!(groups[0].status, "y");
    assert_eq!(groups[1].status, "n");
    assert_eq!(groups[2].status, "m");
}

#[test]
fn test_alias_with_special_characters() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List of newsgroups follows".to_string(),
        lines: vec!["alt.test-old 10 1 =alt.test_new".to_string()],
    };

    let groups = commands::parse_list_active_response(response).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].status, "=alt.test_new");
}

#[test]
fn test_zero_article_numbers_with_junk() {
    // Junk groups often have 0 articles
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List of newsgroups follows".to_string(),
        lines: vec!["alt.spam 0 0 j".to_string()],
    };

    let groups = commands::parse_list_active_response(response).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].high, 0);
    assert_eq!(groups[0].low, 0);
    assert_eq!(groups[0].status, "j");
}

// Real-World Scenarios

#[test]
fn test_real_world_server_response() {
    // Simulates a real server with mixed status values
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List of newsgroups follows".to_string(),
        lines: vec![
            "comp.lang.rust 12345 1000 y".to_string(),
            "comp.lang.c++.moderated 54321 2000 m".to_string(),
            "comp.lang.c++ 100 1 =comp.lang.cplusplus".to_string(),
            "alt.binaries.spam 0 0 j".to_string(),
            "news.admin.net-abuse.email 999 1 x".to_string(),
        ],
    };

    let groups = commands::parse_list_active_response(response).unwrap();
    assert_eq!(groups.len(), 5);

    // Verify each group
    assert_eq!(groups[0].status, "y");
    assert_eq!(groups[1].status, "m");
    assert_eq!(groups[2].status, "=comp.lang.cplusplus");
    assert_eq!(groups[3].status, "j");
    assert_eq!(groups[4].status, "x");
}

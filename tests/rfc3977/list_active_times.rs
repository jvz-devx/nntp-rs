//! RFC 3977 Section 7.6.4 - LIST ACTIVE.TIMES Command Tests
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc3977#section-7.6.4
//!
//! Tests for the LIST ACTIVE.TIMES command and response parsing.

use nntp_rs::{codes, commands, NntpError, NntpResponse};
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

    let groups = commands::parse_list_active_times_response(response).unwrap();
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

    let groups = commands::parse_list_active_times_response(response).unwrap();
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

    let result = commands::parse_list_active_times_response(response);
    assert!(result.is_err());
}

#[test]
fn test_parse_list_active_times_empty_response() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![],
    };

    let groups = commands::parse_list_active_times_response(response).unwrap();
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

    let groups = commands::parse_list_active_times_response(response).unwrap();
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

    let groups = commands::parse_list_active_times_response(response).unwrap();
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

    let groups = commands::parse_list_active_times_response(response).unwrap();
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

    let groups = commands::parse_list_active_times_response(response).unwrap();
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

    let groups = commands::parse_list_active_times_response(response).unwrap();
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

    let groups = commands::parse_list_active_times_response(response).unwrap();
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

    let result = commands::parse_list_active_times_response(response);
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

    let groups = commands::parse_list_active_times_response(response).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].name, "alt.test-group_with.special-chars");
}

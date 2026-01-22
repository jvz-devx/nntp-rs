//! RFC 3977 Section 5.2 - CAPABILITIES Command Tests
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc3977#section-5.2
//!
//! The CAPABILITIES command allows a client to determine which extensions
//! and features the server supports.

use nntp_rs::{codes, commands, Capabilities, NntpResponse};
#[test]
fn test_capabilities_command_format() {
    let cmd = commands::capabilities();
    assert_eq!(cmd, "CAPABILITIES\r\n");
}

#[test]
fn test_capabilities_command_ends_with_crlf() {
    let cmd = commands::capabilities();
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_capabilities_with_keyword_format() {
    let cmd = commands::capabilities_with_keyword("AUTHINFO");
    assert_eq!(cmd, "CAPABILITIES AUTHINFO\r\n");
}
#[test]
fn test_parse_capabilities_response_basic() {
    let response = NntpResponse {
        code: codes::CAPABILITY_LIST,
        message: "Capability list follows".to_string(),
        lines: vec![
            "VERSION 2".to_string(),
            "READER".to_string(),
            "POST".to_string(),
            "IHAVE".to_string(),
        ],
    };

    let caps = Capabilities::parse(&response.lines);

    assert!(caps.has("VERSION"));
    assert!(caps.has("READER"));
    assert!(caps.has("POST"));
    assert!(caps.has("IHAVE"));
}

#[test]
fn test_parse_capabilities_with_arguments() {
    let response = NntpResponse {
        code: codes::CAPABILITY_LIST,
        message: "Capability list".to_string(),
        lines: vec![
            "VERSION 2".to_string(),
            "READER".to_string(),
            "COMPRESS DEFLATE GZIP".to_string(),
            "AUTHINFO USER SASL".to_string(),
        ],
    };

    let caps = Capabilities::parse(&response.lines);

    // Test capability presence
    assert!(caps.has("COMPRESS"));
    assert!(caps.has("AUTHINFO"));

    // Test arguments
    let compress_args = caps.get_args("COMPRESS").unwrap();
    assert_eq!(compress_args.len(), 2);
    assert_eq!(compress_args[0], "DEFLATE");
    assert_eq!(compress_args[1], "GZIP");

    let authinfo_args = caps.get_args("AUTHINFO").unwrap();
    assert_eq!(authinfo_args.len(), 2);
    assert_eq!(authinfo_args[0], "USER");
    assert_eq!(authinfo_args[1], "SASL");
}

#[test]
fn test_parse_capabilities_case_insensitive() {
    let response = NntpResponse {
        code: codes::CAPABILITY_LIST,
        message: "Capability list".to_string(),
        lines: vec![
            "version 2".to_string(),
            "ReAdEr".to_string(),
            "COMPRESS deflate".to_string(),
        ],
    };

    let caps = Capabilities::parse(&response.lines);

    // All these should work regardless of case
    assert!(caps.has("VERSION"));
    assert!(caps.has("version"));
    assert!(caps.has("Version"));
    assert!(caps.has("READER"));
    assert!(caps.has("reader"));
    assert!(caps.has("COMPRESS"));
}

#[test]
fn test_parse_capabilities_empty_response() {
    let response = NntpResponse {
        code: codes::CAPABILITY_LIST,
        message: "Capability list".to_string(),
        lines: vec![],
    };

    let caps = Capabilities::parse(&response.lines);
    assert_eq!(caps.list().len(), 0);
}

#[test]
fn test_parse_capabilities_with_empty_lines() {
    let response = NntpResponse {
        code: codes::CAPABILITY_LIST,
        message: "Capability list".to_string(),
        lines: vec![
            "".to_string(),
            "VERSION 2".to_string(),
            "".to_string(),
            "READER".to_string(),
            "".to_string(),
        ],
    };

    let caps = Capabilities::parse(&response.lines);

    // Empty lines should be ignored
    assert!(caps.has("VERSION"));
    assert!(caps.has("READER"));
    assert_eq!(caps.list().len(), 2);
}
#[test]
fn test_has_arg_checks_specific_argument() {
    let response = NntpResponse {
        code: codes::CAPABILITY_LIST,
        message: "Capability list".to_string(),
        lines: vec!["COMPRESS DEFLATE GZIP".to_string()],
    };

    let caps = Capabilities::parse(&response.lines);

    assert!(caps.has_arg("COMPRESS", "DEFLATE"));
    assert!(caps.has_arg("COMPRESS", "GZIP"));
    assert!(!caps.has_arg("COMPRESS", "BZIP2"));
}

#[test]
fn test_has_arg_missing_capability() {
    let response = NntpResponse {
        code: codes::CAPABILITY_LIST,
        message: "Capability list".to_string(),
        lines: vec!["VERSION 2".to_string()],
    };

    let caps = Capabilities::parse(&response.lines);

    assert!(!caps.has_arg("STREAMING", "CHECK"));
    assert!(!caps.has_arg("NONEXISTENT", "ARG"));
}

#[test]
fn test_has_arg_case_insensitive() {
    let response = NntpResponse {
        code: codes::CAPABILITY_LIST,
        message: "Capability list".to_string(),
        lines: vec!["compress deflate".to_string()],
    };

    let caps = Capabilities::parse(&response.lines);

    assert!(caps.has_arg("COMPRESS", "deflate"));
    assert!(caps.has_arg("compress", "DEFLATE"));
    assert!(caps.has_arg("Compress", "Deflate"));
}

#[test]
fn test_get_args_returns_none_for_missing_capability() {
    let response = NntpResponse {
        code: codes::CAPABILITY_LIST,
        message: "Capability list".to_string(),
        lines: vec!["VERSION 2".to_string()],
    };

    let caps = Capabilities::parse(&response.lines);

    assert!(caps.get_args("STREAMING").is_none());
    assert!(caps.get_args("NONEXISTENT").is_none());
}

#[test]
fn test_list_returns_all_capabilities() {
    let response = NntpResponse {
        code: codes::CAPABILITY_LIST,
        message: "Capability list".to_string(),
        lines: vec![
            "VERSION 2".to_string(),
            "READER".to_string(),
            "POST".to_string(),
        ],
    };

    let caps = Capabilities::parse(&response.lines);

    let list = caps.list();
    assert_eq!(list.len(), 3);
    assert!(list.contains(&"VERSION".to_string()));
    assert!(list.contains(&"READER".to_string()));
    assert!(list.contains(&"POST".to_string()));
}
#[test]
fn test_capability_list_response_code() {
    let response = NntpResponse {
        code: codes::CAPABILITY_LIST,
        message: "Capability list".to_string(),
        lines: vec!["VERSION 2".to_string()],
    };

    assert_eq!(response.code, 101);
}

#[test]
fn test_capability_response_is_not_success() {
    // Code 101 is informational (1xx), not success (2xx)
    let response = NntpResponse {
        code: codes::CAPABILITY_LIST,
        message: "Capability list".to_string(),
        lines: vec![],
    };

    assert!(!response.is_success());
    assert!(!response.is_continuation());
    assert!(!response.is_error());
}

// Real-World Examples

#[test]
fn test_parse_capabilities_typical_usenet_provider() {
    // Typical response from a Usenet provider
    let response = NntpResponse {
        code: codes::CAPABILITY_LIST,
        message: "Capability list follows".to_string(),
        lines: vec![
            "VERSION 2".to_string(),
            "READER".to_string(),
            "POST".to_string(),
            "IHAVE".to_string(),
            "NEWNEWS".to_string(),
            "HDR".to_string(),
            "OVER".to_string(),
            "LIST ACTIVE NEWSGROUPS OVERVIEW.FMT".to_string(),
            "COMPRESS DEFLATE".to_string(),
            "AUTHINFO USER".to_string(),
            "STARTTLS".to_string(),
        ],
    };

    let caps = Capabilities::parse(&response.lines);

    // Basic capabilities
    assert!(caps.has("VERSION"));
    assert!(caps.has("READER"));
    assert!(caps.has("POST"));

    // Article access
    assert!(caps.has("HDR"));
    assert!(caps.has("OVER"));

    // LIST variants
    assert!(caps.has("LIST"));
    let list_args = caps.get_args("LIST").unwrap();
    assert!(list_args.contains(&"ACTIVE".to_string()));
    assert!(list_args.contains(&"NEWSGROUPS".to_string()));
    assert!(list_args.contains(&"OVERVIEW.FMT".to_string()));

    // Extensions
    assert!(caps.has("COMPRESS"));
    assert!(caps.has_arg("COMPRESS", "DEFLATE"));
    assert!(caps.has("AUTHINFO"));
    assert!(caps.has_arg("AUTHINFO", "USER"));
    assert!(caps.has("STARTTLS"));
}

#[test]
fn test_parse_capabilities_minimal_server() {
    // Minimal RFC 3977 compliant server
    let response = NntpResponse {
        code: codes::CAPABILITY_LIST,
        message: "Capability list".to_string(),
        lines: vec![
            "VERSION 2".to_string(),
            "READER".to_string(),
            "IHAVE".to_string(),
        ],
    };

    let caps = Capabilities::parse(&response.lines);

    assert!(caps.has("VERSION"));
    assert_eq!(caps.get_args("VERSION").unwrap()[0], "2");
    assert!(caps.has("READER"));
    assert!(caps.has("IHAVE"));
    assert_eq!(caps.list().len(), 3);
}


#[test]
fn test_capabilities_with_multiple_spaces() {
    let response = NntpResponse {
        code: codes::CAPABILITY_LIST,
        message: "Capability list".to_string(),
        lines: vec!["COMPRESS  DEFLATE   GZIP".to_string()],
    };

    let caps = Capabilities::parse(&response.lines);

    assert!(caps.has("COMPRESS"));
    let args = caps.get_args("COMPRESS").unwrap();
    assert_eq!(args.len(), 2);
    assert_eq!(args[0], "DEFLATE");
    assert_eq!(args[1], "GZIP");
}

#[test]
fn test_capabilities_with_tabs() {
    let response = NntpResponse {
        code: codes::CAPABILITY_LIST,
        message: "Capability list".to_string(),
        lines: vec!["COMPRESS\tDEFLATE\tGZIP".to_string()],
    };

    let caps = Capabilities::parse(&response.lines);

    assert!(caps.has("COMPRESS"));
    let args = caps.get_args("COMPRESS").unwrap();
    assert_eq!(args.len(), 2);
}

#[test]
fn test_capability_no_arguments() {
    let response = NntpResponse {
        code: codes::CAPABILITY_LIST,
        message: "Capability list".to_string(),
        lines: vec!["READER".to_string()],
    };

    let caps = Capabilities::parse(&response.lines);

    assert!(caps.has("READER"));
    let args = caps.get_args("READER").unwrap();
    assert_eq!(args.len(), 0);
}

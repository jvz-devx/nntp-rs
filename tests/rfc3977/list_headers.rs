//! RFC 3977 Section 8.6 - LIST HEADERS Command Tests
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc3977#section-8.6
//!
//! Tests for the LIST HEADERS command and response parsing.

use nntp_rs::{NntpResponse, codes, commands};
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

    let headers = commands::parse_list_headers_response(response).unwrap();
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

    let headers = commands::parse_list_headers_response(response).unwrap();
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

    let headers = commands::parse_list_headers_response(response).unwrap();
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

    let headers = commands::parse_list_headers_response(response).unwrap();
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

    let headers = commands::parse_list_headers_response(response).unwrap();
    assert_eq!(headers.len(), 0);
}

#[test]
fn test_parse_list_headers_response_protocol_error() {
    let response = NntpResponse {
        code: 500,
        message: "Command not understood".to_string(),
        lines: vec![],
    };

    assert!(commands::parse_list_headers_response(response).is_err());
}

#[test]
fn test_parse_list_headers_response_wrong_code() {
    let response = NntpResponse {
        code: 480,
        message: "Authentication required".to_string(),
        lines: vec![],
    };

    assert!(commands::parse_list_headers_response(response).is_err());
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

    let headers = commands::parse_list_headers_response(response).unwrap();
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

    let headers = commands::parse_list_headers_response(response).unwrap();
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

    let headers = commands::parse_list_headers_response(response).unwrap();
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

    let headers = commands::parse_list_headers_response(response).unwrap();
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

    let headers = commands::parse_list_headers_response(response).unwrap();
    assert_eq!(headers.len(), 4);
}

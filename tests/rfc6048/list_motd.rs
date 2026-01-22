//! RFC 6048 Section 6 - LIST MOTD Tests
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc6048#section-6

use nntp_rs::{codes, commands, NntpResponse};
#[test]
fn test_list_motd_command_format() {
    let cmd = commands::list_motd();
    assert_eq!(cmd, "LIST MOTD\r\n");
}

#[test]
fn test_list_motd_ends_with_crlf() {
    let cmd = commands::list_motd();
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_list_motd_command_uppercase() {
    let cmd = commands::list_motd();
    assert!(cmd.starts_with("LIST MOTD"));
}

#[test]
fn test_list_motd_no_arguments() {
    let cmd = commands::list_motd();
    // Should be exactly "LIST MOTD\r\n" with no arguments
    assert_eq!(cmd, "LIST MOTD\r\n");
}
#[test]
fn test_parse_list_motd_response_basic() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Message of the day follows".to_string(),
        lines: vec![
            "Welcome to our NNTP server!".to_string(),
            "Server maintenance scheduled for midnight.".to_string(),
        ],
    };

    let motd = commands::parse_list_motd_response(&response).unwrap();
    assert_eq!(motd.len(), 2);
    assert_eq!(motd[0], "Welcome to our NNTP server!");
    assert_eq!(motd[1], "Server maintenance scheduled for midnight.");
}

#[test]
fn test_parse_list_motd_response_empty() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "MOTD follows".to_string(),
        lines: vec![],
    };

    let motd = commands::parse_list_motd_response(&response).unwrap();
    assert_eq!(motd.len(), 0);
}

#[test]
fn test_parse_list_motd_response_single_line() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "MOTD follows".to_string(),
        lines: vec!["Server is running normally.".to_string()],
    };

    let motd = commands::parse_list_motd_response(&response).unwrap();
    assert_eq!(motd.len(), 1);
    assert_eq!(motd[0], "Server is running normally.");
}

#[test]
fn test_parse_list_motd_response_multiple_lines() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Message of the day".to_string(),
        lines: vec![
            "========================================".to_string(),
            "Welcome to ACME News Server".to_string(),
            "========================================".to_string(),
            "".to_string(),
            "Server uptime: 99.9%".to_string(),
            "Contact: admin@example.com".to_string(),
            "".to_string(),
            "Enjoy your stay!".to_string(),
        ],
    };

    let motd = commands::parse_list_motd_response(&response).unwrap();
    assert_eq!(motd.len(), 8);
    assert_eq!(motd[0], "========================================");
    assert_eq!(motd[1], "Welcome to ACME News Server");
    assert_eq!(motd[3], ""); // Empty lines are preserved
    assert_eq!(motd[7], "Enjoy your stay!");
}

#[test]
fn test_parse_list_motd_preserves_empty_lines() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "MOTD".to_string(),
        lines: vec!["Line 1".to_string(), "".to_string(), "Line 3".to_string()],
    };

    let motd = commands::parse_list_motd_response(&response).unwrap();
    assert_eq!(motd.len(), 3);
    assert_eq!(motd[0], "Line 1");
    assert_eq!(motd[1], ""); // Empty line preserved
    assert_eq!(motd[2], "Line 3");
}

#[test]
fn test_parse_list_motd_with_special_characters() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "MOTD".to_string(),
        lines: vec![
            "Server: news.example.com:119".to_string(),
            "Support: <admin@example.com>".to_string(),
            "Important: !!! Read the FAQ !!!".to_string(),
        ],
    };

    let motd = commands::parse_list_motd_response(&response).unwrap();
    assert_eq!(motd.len(), 3);
    assert_eq!(motd[0], "Server: news.example.com:119");
    assert_eq!(motd[1], "Support: <admin@example.com>");
    assert_eq!(motd[2], "Important: !!! Read the FAQ !!!");
}
#[test]
fn test_parse_list_motd_protocol_error() {
    let response = NntpResponse {
        code: 500,
        message: "Command not recognized".to_string(),
        lines: vec![],
    };

    let result = commands::parse_list_motd_response(&response);
    assert!(result.is_err());
}

#[test]
fn test_parse_list_motd_wrong_success_code() {
    let response = NntpResponse {
        code: 200,
        message: "OK".to_string(),
        lines: vec!["Test".to_string()],
    };

    let result = commands::parse_list_motd_response(&response);
    // Should succeed since code 200 is 2xx (success range)
    assert!(result.is_ok());
}

#[test]
fn test_parse_list_motd_error_code_401() {
    let response = NntpResponse {
        code: 401,
        message: "Permission denied".to_string(),
        lines: vec![],
    };

    let result = commands::parse_list_motd_response(&response);
    assert!(result.is_err());
}

#[test]
fn test_parse_list_motd_error_code_502() {
    let response = NntpResponse {
        code: 502,
        message: "Command not supported".to_string(),
        lines: vec![],
    };

    let result = commands::parse_list_motd_response(&response);
    assert!(result.is_err());
}


#[test]
fn test_parse_list_motd_very_long_motd() {
    let mut lines = Vec::new();
    for i in 0..1000 {
        lines.push(format!("Line {}: This is a test line", i));
    }

    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Long MOTD".to_string(),
        lines,
    };

    let motd = commands::parse_list_motd_response(&response).unwrap();
    assert_eq!(motd.len(), 1000);
    assert_eq!(motd[0], "Line 0: This is a test line");
    assert_eq!(motd[999], "Line 999: This is a test line");
}

#[test]
fn test_parse_list_motd_long_lines() {
    let long_line = "=".repeat(500);
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "MOTD".to_string(),
        lines: vec![long_line.clone()],
    };

    let motd = commands::parse_list_motd_response(&response).unwrap();
    assert_eq!(motd.len(), 1);
    assert_eq!(motd[0].len(), 500);
    assert_eq!(motd[0], long_line);
}

#[test]
fn test_parse_list_motd_unicode_characters() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "MOTD".to_string(),
        lines: vec![
            "欢迎 (Welcome in Chinese)".to_string(),
            "Привет (Hello in Russian)".to_string(),
            "مرحبا (Hello in Arabic)".to_string(),
        ],
    };

    let motd = commands::parse_list_motd_response(&response).unwrap();
    assert_eq!(motd.len(), 3);
    assert_eq!(motd[0], "欢迎 (Welcome in Chinese)");
    assert_eq!(motd[1], "Привет (Hello in Russian)");
    assert_eq!(motd[2], "مرحبا (Hello in Arabic)");
}

#[test]
fn test_parse_list_motd_whitespace_only_lines() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "MOTD".to_string(),
        lines: vec![
            "Header".to_string(),
            "   ".to_string(), // Spaces only
            "Body".to_string(),
        ],
    };

    let motd = commands::parse_list_motd_response(&response).unwrap();
    assert_eq!(motd.len(), 3);
    assert_eq!(motd[0], "Header");
    assert_eq!(motd[1], "   "); // Whitespace preserved
    assert_eq!(motd[2], "Body");
}

// Real-World Scenarios

#[test]
fn test_list_motd_rfc_6048_section_6_example() {
    // Example based on RFC 6048 Section 6
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Message of the day follows".to_string(),
        lines: vec![
            "Welcome to news.example.com".to_string(),
            "".to_string(),
            "Please report any issues to admin@example.com".to_string(),
        ],
    };

    let motd = commands::parse_list_motd_response(&response).unwrap();
    assert_eq!(motd.len(), 3);
    assert_eq!(motd[0], "Welcome to news.example.com");
    assert_eq!(motd[1], ""); // Blank line for formatting
    assert_eq!(motd[2], "Please report any issues to admin@example.com");
}

#[test]
fn test_list_motd_typical_server_response() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "MOTD".to_string(),
        lines: vec![
            "================================================".to_string(),
            "      News Server - news.acme.org              ".to_string(),
            "================================================".to_string(),
            "".to_string(),
            "Welcome! This server provides access to:".to_string(),
            "  - comp.* hierarchy".to_string(),
            "  - alt.* hierarchy".to_string(),
            "  - local.* groups".to_string(),
            "".to_string(),
            "Retention: 30 days for text groups".to_string(),
            "           7 days for binary groups".to_string(),
            "".to_string(),
            "For support, email: support@acme.org".to_string(),
        ],
    };

    let motd = commands::parse_list_motd_response(&response).unwrap();
    assert_eq!(motd.len(), 13);
    assert!(motd[0].contains("====="));
    assert!(motd[4].contains("Welcome"));
    assert!(motd[12].contains("support@acme.org"));
}

#[test]
fn test_list_motd_maintenance_announcement() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Server message".to_string(),
        lines: vec![
            "*** MAINTENANCE NOTICE ***".to_string(),
            "".to_string(),
            "Scheduled downtime: 2024-02-01 00:00-02:00 UTC".to_string(),
            "Reason: Hardware upgrades".to_string(),
            "".to_string(),
            "All services will be unavailable during this time.".to_string(),
            "We apologize for any inconvenience.".to_string(),
        ],
    };

    let motd = commands::parse_list_motd_response(&response).unwrap();
    assert_eq!(motd.len(), 7);
    assert!(motd[0].contains("MAINTENANCE"));
    assert!(motd[2].contains("2024-02-01"));
}

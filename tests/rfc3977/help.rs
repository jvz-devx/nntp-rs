//! RFC 3977 Section 7.2 - HELP Command Tests
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc3977#section-7.2
//!
//! The HELP command requests help text from the server, which typically includes
//! a list of supported commands and server-specific information.

use nntp_rs::{codes, commands, NntpResponse};
#[test]
fn test_help_command_format() {
    let cmd = commands::help();
    assert_eq!(cmd, "HELP\r\n");
}

#[test]
fn test_help_command_ends_with_crlf() {
    let cmd = commands::help();
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_help_command_uppercase() {
    let cmd = commands::help();
    assert!(cmd.starts_with("HELP"));
}

#[test]
fn test_help_command_no_arguments() {
    // HELP takes no arguments
    let cmd = commands::help();
    assert_eq!(cmd.trim_end(), "HELP");
}
#[test]
fn test_help_response_code_100() {
    let response = NntpResponse {
        code: codes::HELP_TEXT_FOLLOWS,
        message: "Help text follows".to_string(),
        lines: vec!["ARTICLE".to_string(), "GROUP".to_string()],
    };

    assert_eq!(response.code, 100);
    // 1xx codes are informational, not "success" (2xx)
    assert!(!response.is_success());
    assert!(!response.is_error());
}

#[test]
fn test_help_response_is_informational() {
    let response = NntpResponse {
        code: codes::HELP_TEXT_FOLLOWS,
        message: "Help".to_string(),
        lines: vec![],
    };

    // 1xx codes are informational, not "success" (2xx) or "error" (4xx/5xx)
    assert!(!response.is_success());
    assert!(!response.is_error());
    assert!(!response.is_continuation());
}

#[test]
fn test_help_response_has_multiline_data() {
    // HELP always returns multi-line response with help text
    let response = NntpResponse {
        code: codes::HELP_TEXT_FOLLOWS,
        message: "Help text follows".to_string(),
        lines: vec![
            "ARTICLE [message-id|number]".to_string(),
            "BODY [message-id|number]".to_string(),
            "HEAD [message-id|number]".to_string(),
        ],
    };

    assert!(!response.lines.is_empty());
    assert_eq!(response.lines.len(), 3);
}
#[test]
fn test_help_typical_response() {
    let response = NntpResponse {
        code: codes::HELP_TEXT_FOLLOWS,
        message: "Help text follows (multi-line)".to_string(),
        lines: vec![
            "ARTICLE [message-id|number]".to_string(),
            "BODY [message-id|number]".to_string(),
            "CAPABILITIES".to_string(),
            "GROUP newsgroup".to_string(),
            "HEAD [message-id|number]".to_string(),
            "HELP".to_string(),
            "QUIT".to_string(),
        ],
    };

    assert_eq!(response.code, 100);
    assert!(response.lines.len() >= 5);
}

#[test]
fn test_help_response_message_contains_help() {
    let response = NntpResponse {
        code: codes::HELP_TEXT_FOLLOWS,
        message: "Help text follows".to_string(),
        lines: vec![],
    };

    assert!(
        response.message.to_lowercase().contains("help")
            || response.message.to_lowercase().contains("text")
    );
}

// Real-World Examples

#[test]
fn test_help_minimal_server_response() {
    // Minimal help response from a basic server
    let response = NntpResponse {
        code: codes::HELP_TEXT_FOLLOWS,
        message: "100 Help text follows".to_string(),
        lines: vec![
            "ARTICLE".to_string(),
            "BODY".to_string(),
            "GROUP".to_string(),
            "HEAD".to_string(),
            "QUIT".to_string(),
        ],
    };

    assert_eq!(response.code, 100);
    assert!(response.lines.len() >= 5);
}

#[test]
fn test_help_detailed_server_response() {
    // Detailed help response with command descriptions
    let response = NntpResponse {
        code: codes::HELP_TEXT_FOLLOWS,
        message: "Help text follows".to_string(),
        lines: vec![
            "The following commands are recognized:".to_string(),
            "".to_string(),
            "ARTICLE [message-id|number] - Retrieve article".to_string(),
            "BODY [message-id|number] - Retrieve article body".to_string(),
            "CAPABILITIES - List server capabilities".to_string(),
            "GROUP newsgroup - Select newsgroup".to_string(),
            "HEAD [message-id|number] - Retrieve article headers".to_string(),
            "HELP - This help text".to_string(),
            "LIST - List newsgroups".to_string(),
            "QUIT - Close connection".to_string(),
        ],
    };

    assert_eq!(response.code, 100);
    assert!(response.lines.len() >= 8);
}

#[test]
fn test_help_with_server_info() {
    // Some servers include version/info in help text
    let response = NntpResponse {
        code: codes::HELP_TEXT_FOLLOWS,
        message: "Help text follows".to_string(),
        lines: vec![
            "NNTP Server v1.0".to_string(),
            "".to_string(),
            "Supported commands:".to_string(),
            "ARTICLE, BODY, GROUP, HEAD, HELP, LIST, QUIT".to_string(),
        ],
    };

    assert_eq!(response.code, 100);
}

#[test]
fn test_help_empty_lines_preserved() {
    // Help text may contain blank lines for formatting
    let response = NntpResponse {
        code: codes::HELP_TEXT_FOLLOWS,
        message: "Help".to_string(),
        lines: vec![
            "Basic Commands:".to_string(),
            "".to_string(),
            "ARTICLE".to_string(),
            "GROUP".to_string(),
            "".to_string(),
            "Info Commands:".to_string(),
            "".to_string(),
            "HELP".to_string(),
        ],
    };

    // Empty lines should be preserved
    assert!(response.lines.contains(&"".to_string()));
}
#[test]
fn test_help_error_response_is_error() {
    // Some servers may not support HELP (unlikely)
    let response = NntpResponse {
        code: 502,
        message: "Command unavailable".to_string(),
        lines: vec![],
    };

    assert!(response.is_error());
    assert!(!response.is_success());
}

#[test]
fn test_help_authentication_may_be_required() {
    // Some servers may require authentication before HELP
    let response = NntpResponse {
        code: 480,
        message: "Authentication required".to_string(),
        lines: vec![],
    };

    assert!(response.is_error());
    assert_eq!(response.code, 480);
}

#[test]
fn test_help_empty_help_text() {
    // Edge case: Server returns no help lines
    let response = NntpResponse {
        code: codes::HELP_TEXT_FOLLOWS,
        message: "Help text follows".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 100);
    // 1xx codes are informational, not "success" (2xx)
    assert!(!response.is_error());
}

#[test]
fn test_help_very_long_help_text() {
    // Some servers may have extensive help
    let lines: Vec<String> = (0..100).map(|i| format!("Command {}", i)).collect();
    let response = NntpResponse {
        code: codes::HELP_TEXT_FOLLOWS,
        message: "Help text follows".to_string(),
        lines,
    };

    assert_eq!(response.code, 100);
    assert_eq!(response.lines.len(), 100);
}

#[test]
fn test_help_unicode_in_help_text() {
    // Some servers may include unicode in help text
    let response = NntpResponse {
        code: codes::HELP_TEXT_FOLLOWS,
        message: "Help text follows".to_string(),
        lines: vec![
            "Welcome to NNTP Server 📰".to_string(),
            "Commands: ARTICLE, GROUP, QUIT".to_string(),
        ],
    };

    assert_eq!(response.code, 100);
    assert!(response.lines[0].contains("📰"));
}

#[test]
fn test_help_code_constant_value() {
    // Verify the response code constant is correct
    assert_eq!(codes::HELP_TEXT_FOLLOWS, 100);
}

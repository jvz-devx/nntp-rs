//! RFC 3977 Section 3.1 - Response Line Parsing Tests
//!
//! These tests verify compliance with NNTP response format requirements:
//! - Responses begin with a three-digit status code
//! - Status code is followed by optional space and message
//! - Response line must not exceed 512 octets

use nntp_rs::commands::parse_response_line;

// Valid Response Parsing (RFC 3977 §3.1)

#[test]
fn test_response_three_digit_code_with_message() {
    let (code, msg) = parse_response_line("200 server ready").unwrap();
    assert_eq!(code, 200);
    assert_eq!(msg, "server ready");
}

#[test]
fn test_response_three_digit_code_only() {
    // RFC 3977 §3.1: "status indicator" followed by optional text
    // A bare "200" with no space or message is valid
    let (code, msg) = parse_response_line("200").unwrap();
    assert_eq!(code, 200);
    assert_eq!(msg, "");
}

#[test]
fn test_response_with_empty_message_after_space() {
    // "200 " should parse as code 200 with empty message
    let (code, msg) = parse_response_line("200 ").unwrap();
    assert_eq!(code, 200);
    assert_eq!(msg, "");
}

#[test]
fn test_response_all_2xx_success_codes() {
    // Test representative 2xx codes from RFC 3977
    let test_cases = [
        ("200 posting allowed", 200),
        ("201 no posting", 201),
        ("205 closing connection", 205),
        ("206 compression active", 206),
        ("211 1234 5 6789 group.name", 211),
        ("220 0 <msgid> article follows", 220),
        ("221 0 <msgid> head follows", 221),
        ("222 0 <msgid> body follows", 222),
        ("224 overview info follows", 224),
        ("281 authentication accepted", 281),
    ];

    for (input, expected_code) in test_cases {
        let (code, _) = parse_response_line(input).unwrap();
        assert_eq!(code, expected_code, "Failed for input: {}", input);
    }
}

#[test]
fn test_response_3xx_continuation_codes() {
    let test_cases = [("340 send article", 340), ("381 password required", 381)];

    for (input, expected_code) in test_cases {
        let (code, _) = parse_response_line(input).unwrap();
        assert_eq!(code, expected_code, "Failed for input: {}", input);
    }
}

#[test]
fn test_response_4xx_error_codes() {
    let test_cases = [
        ("400 service unavailable", 400),
        ("411 no such group", 411),
        ("412 no group selected", 412),
        ("420 no current article", 420),
        ("423 no article with that number", 423),
        ("430 no article with that message-id", 430),
        ("481 authentication rejected", 481),
        ("482 authentication out of sequence", 482),
    ];

    for (input, expected_code) in test_cases {
        let (code, _) = parse_response_line(input).unwrap();
        assert_eq!(code, expected_code, "Failed for input: {}", input);
    }
}

#[test]
fn test_response_5xx_permanent_error_codes() {
    let test_cases = [
        ("500 command not recognized", 500),
        ("501 syntax error", 501),
        ("502 access denied", 502),
        ("503 program fault", 503),
    ];

    for (input, expected_code) in test_cases {
        let (code, _) = parse_response_line(input).unwrap();
        assert_eq!(code, expected_code, "Failed for input: {}", input);
    }
}

#[test]
fn test_response_base10_numeric_code() {
    // RFC 3977 §3.1: "All numeric arguments are in base 10"
    // Leading zeros should be handled correctly
    let (code, _) = parse_response_line("042 test").unwrap();
    assert_eq!(code, 42);
}

#[test]
fn test_response_message_preserves_content() {
    // Message should preserve everything after the space
    let (_, msg) = parse_response_line("200 Hello, World! How are you?").unwrap();
    assert_eq!(msg, "Hello, World! How are you?");
}

#[test]
fn test_response_message_with_special_chars() {
    // Message can contain various characters
    let (code, msg) = parse_response_line("200 Test <msg@id> [INFO] {data}").unwrap();
    assert_eq!(code, 200);
    assert_eq!(msg, "Test <msg@id> [INFO] {data}");
}

// Invalid Response Parsing (RFC 3977 §3.1)

#[test]
fn test_response_empty_string_is_invalid() {
    assert!(parse_response_line("").is_err());
}

#[test]
fn test_response_whitespace_only_is_invalid() {
    assert!(parse_response_line("   ").is_err());
}

#[test]
fn test_response_two_digit_code_is_invalid() {
    // Must be exactly 3 digits
    assert!(parse_response_line("20 OK").is_err());
}

#[test]
fn test_response_one_digit_code_is_invalid() {
    assert!(parse_response_line("2 OK").is_err());
}

#[test]
fn test_response_four_digit_code_is_rejected() {
    // RFC 3977 §3.1 requires exactly 3 digits for the status code.
    // Parser rejects 4-digit codes as malformed responses.
    assert!(parse_response_line("2000 OK").is_err());
    assert!(parse_response_line("9999 some message").is_err());
}

#[test]
fn test_response_non_digit_in_code_is_invalid() {
    assert!(parse_response_line("2x0 OK").is_err());
    assert!(parse_response_line("x00 OK").is_err());
    assert!(parse_response_line("20x OK").is_err());
}

#[test]
fn test_response_negative_code_is_invalid() {
    // "-200" starts with '-', not a digit
    assert!(parse_response_line("-200 OK").is_err());
}

#[test]
fn test_response_alpha_only_is_invalid() {
    assert!(parse_response_line("ABC OK").is_err());
    assert!(parse_response_line("OK").is_err());
}

#[test]
fn test_response_space_prefix_is_invalid() {
    // Response must start with 3-digit code, not space
    assert!(parse_response_line(" 200 OK").is_err());
}

#[test]
fn test_response_code_000() {
    // 000 is technically 3 digits, should parse
    let (code, msg) = parse_response_line("000 test").unwrap();
    assert_eq!(code, 0);
    assert_eq!(msg, "test");
}

#[test]
fn test_response_code_999() {
    // 999 is valid 3-digit code
    let (code, msg) = parse_response_line("999 test").unwrap();
    assert_eq!(code, 999);
    assert_eq!(msg, "test");
}

#[test]
fn test_response_long_message() {
    // RFC allows up to 512 octets total, but our parser should handle longer gracefully
    let long_msg = "x".repeat(1000);
    let input = format!("200 {}", long_msg);
    let (code, msg) = parse_response_line(&input).unwrap();
    assert_eq!(code, 200);
    assert_eq!(msg, long_msg);
}

#[test]
fn test_response_unicode_message() {
    // NNTP uses UTF-8, messages can contain unicode
    let (code, msg) = parse_response_line("200 Willkommen! Bienvenue! \u{1F44B}").unwrap();
    assert_eq!(code, 200);
    assert!(msg.contains("Willkommen"));
    assert!(msg.contains("\u{1F44B}"));
}

#[test]
fn test_response_multiple_spaces_in_message() {
    // Multiple spaces should be preserved
    let (_, msg) = parse_response_line("200 hello    world").unwrap();
    assert_eq!(msg, "hello    world");
}

// RFC 3977 §3.1 - 1xx Informative Response Codes
//
// RFC 3977 defines 1xx codes as informative responses. These are used for
// multi-line help text and server date/time information.

#[test]
fn test_response_1xx_informative_codes() {
    // RFC 3977: 1xx = Informative message
    let test_cases = [
        ("100 help text follows", 100),
        ("101 capability list", 101),
        ("111 20240101120000 server date and time", 111),
    ];

    for (input, expected_code) in test_cases {
        let (code, _) = parse_response_line(input).unwrap();
        assert_eq!(code, expected_code, "Failed for input: {}", input);
    }
}

#[test]
fn test_response_100_help_text() {
    // RFC 3977 §7.2: 100 = Help text follows (multi-line)
    let (code, msg) = parse_response_line("100 Help text follows").unwrap();
    assert_eq!(code, 100);
    assert!(msg.to_lowercase().contains("help"));
}

#[test]
fn test_response_101_capability_list() {
    // RFC 3977 §5.2: 101 = Capability list follows (multi-line)
    let (code, msg) = parse_response_line("101 Capability list:").unwrap();
    assert_eq!(code, 101);
    assert_eq!(msg, "Capability list:");
}

#[test]
fn test_response_111_server_date() {
    // RFC 3977 §7.1: 111 yyyymmddhhmmss = Server date and time
    // Format: 111 yyyymmddhhmmss
    let (code, msg) = parse_response_line("111 20240115123456").unwrap();
    assert_eq!(code, 111);
    assert_eq!(msg, "20240115123456");
    // Verify timestamp format (14 digits)
    assert_eq!(msg.len(), 14);
    assert!(msg.chars().all(|c| c.is_ascii_digit()));
}

#[test]
fn test_response_111_with_optional_text() {
    // Some servers add optional text after the timestamp
    let (code, msg) = parse_response_line("111 20240115123456 UTC").unwrap();
    assert_eq!(code, 111);
    assert!(msg.starts_with("20240115123456"));
}

#[test]
fn test_response_1xx_is_informative() {
    // RFC 3977 §3.1: 1xx codes are informative
    // They are NOT success, continuation, or error
    use nntp_rs::NntpResponse;

    let response = NntpResponse {
        code: 100,
        message: "Help text follows".to_string(),
        lines: vec![],
    };

    // 1xx is informative - not a success, continuation, or error in traditional sense
    assert!(!response.is_success()); // 1xx != 2xx
    assert!(!response.is_continuation()); // 1xx != 3xx
    assert!(!response.is_error()); // 1xx != 4xx/5xx
}

// RFC 3977 §3.1 - Complete 3xx Continuation Response Codes
//
// RFC 3977 defines 3xx codes as continuation responses requiring client action.

#[test]
fn test_response_3xx_all_continuation_codes() {
    // RFC 3977: 3xx = Continuation response (client must provide more data)
    let test_cases = [
        ("335 send article to be transferred", 335), // IHAVE
        ("340 send article to be posted", 340),      // POST
        ("381 password required", 381),              // AUTHINFO USER
        ("383 continue with SASL exchange", 383),    // AUTHINFO SASL (RFC 4643)
    ];

    for (input, expected_code) in test_cases {
        let (code, _) = parse_response_line(input).unwrap();
        assert_eq!(code, expected_code, "Failed for input: {}", input);
    }
}

#[test]
fn test_response_335_ihave_send_article() {
    // RFC 3977 §6.2.1: 335 = Send article to be transferred (IHAVE)
    let (code, msg) = parse_response_line("335 Send article to be transferred").unwrap();
    assert_eq!(code, 335);
    assert!(msg.to_lowercase().contains("article"));
}

#[test]
fn test_response_340_post_send_article() {
    // RFC 3977 §6.3.1: 340 = Send article to be posted (POST)
    let (code, msg) = parse_response_line("340 Send article to be posted").unwrap();
    assert_eq!(code, 340);
    assert!(msg.to_lowercase().contains("article"));
}

#[test]
fn test_response_381_password_required() {
    // RFC 4643 §2.3: 381 = Password required (AUTHINFO USER)
    let (code, msg) = parse_response_line("381 Enter password").unwrap();
    assert_eq!(code, 381);
    assert!(msg.to_lowercase().contains("password"));
}

#[test]
fn test_response_383_sasl_continue() {
    // RFC 4643 §2.4: 383 = Continue with SASL exchange
    let (code, msg) = parse_response_line("383 Y2hhbGxlbmdl").unwrap(); // Base64 challenge
    assert_eq!(code, 383);
    // Message contains Base64-encoded SASL challenge
    assert!(!msg.is_empty());
}

#[test]
fn test_response_3xx_is_continuation() {
    // RFC 3977 §3.1: 3xx codes require client to send more data
    use nntp_rs::NntpResponse;

    for code in [335, 340, 381, 383] {
        let response = NntpResponse {
            code,
            message: "Continue".to_string(),
            lines: vec![],
        };

        assert!(
            response.is_continuation(),
            "Code {} should be continuation",
            code
        );
        assert!(
            !response.is_success(),
            "Code {} should not be success",
            code
        );
        assert!(!response.is_error(), "Code {} should not be error", code);
    }
}

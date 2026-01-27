//! RFC 3977 Section 3.1 - Command Format Tests
//!
//! These tests verify compliance with NNTP command format requirements:
//! - Commands terminate with CRLF
//! - Commands must not exceed 512 octets
//! - Keywords are case-insensitive (but we send uppercase)
//!
//! This file contains tests that validate RFC compliance and provide
//! executable documentation for command formats. Trivial tests that
//! only verify string concatenation (which Rust's compiler guarantees)
//! have been removed to reduce noise and improve signal-to-noise ratio.

use nntp_rs::commands;

// Command Termination (RFC 3977 §3.1)

#[test]
fn test_all_commands_end_with_crlf() {
    // RFC 3977 §3.1: "Commands in NNTP MUST use the canonical CRLF"
    let test_commands = vec![
        commands::authinfo_user("user"),
        commands::authinfo_pass("pass"),
        commands::group("alt.test"),
        commands::article("<msgid@example>"),
        commands::head("12345"),
        commands::body("12345"),
        commands::xover("1-100"),
        commands::compress_deflate().to_string(),
        commands::xfeature_compress_gzip().to_string(),
        commands::quit().to_string(),
        commands::list().to_string(),
        commands::stat("12345"),
    ];

    for cmd in &test_commands {
        assert!(
            cmd.ends_with("\r\n"),
            "Command does not end with CRLF: {:?}",
            cmd
        );
    }
}

#[test]
fn test_commands_only_one_crlf() {
    // Commands should have exactly one CRLF at the end
    // This prevents command injection attacks
    let cmd = commands::group("test");
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

// AUTHINFO Commands (RFC 4643)

#[test]
fn test_authinfo_user_format() {
    let cmd = commands::authinfo_user("testuser");
    assert_eq!(cmd, "AUTHINFO USER testuser\r\n");
}

#[test]
fn test_authinfo_user_with_spaces() {
    // Usernames shouldn't have spaces, but test handling
    let cmd = commands::authinfo_user("user name");
    assert_eq!(cmd, "AUTHINFO USER user name\r\n");
}

#[test]
fn test_authinfo_pass_with_special_chars() {
    let cmd = commands::authinfo_pass("p@ss!w0rd#$%");
    assert_eq!(cmd, "AUTHINFO PASS p@ss!w0rd#$%\r\n");
}

// GROUP Command (RFC 3977 §6.1.1)

#[test]
fn test_group_command_simple() {
    let cmd = commands::group("alt.test");
    assert_eq!(cmd, "GROUP alt.test\r\n");
}

// ARTICLE/HEAD/BODY Commands (RFC 3977 §6.2)

#[test]
fn test_article_command_with_number() {
    let cmd = commands::article("12345");
    assert_eq!(cmd, "ARTICLE 12345\r\n");
}

#[test]
fn test_article_command_with_message_id() {
    let cmd = commands::article("<unique-id@news.example.com>");
    assert_eq!(cmd, "ARTICLE <unique-id@news.example.com>\r\n");
}

// XOVER Command (RFC 3977 §8.3)

#[test]
fn test_xover_single_article() {
    let cmd = commands::xover("12345");
    assert_eq!(cmd, "XOVER 12345\r\n");
}

#[test]
fn test_xover_range() {
    let cmd = commands::xover("100-200");
    assert_eq!(cmd, "XOVER 100-200\r\n");
}

// Compression Commands (RFC 8054 / XFEATURE)

#[test]
fn test_compress_deflate_format() {
    let cmd = commands::compress_deflate();
    assert_eq!(cmd, "COMPRESS DEFLATE\r\n");
}

#[test]
fn test_xfeature_compress_gzip_format() {
    let cmd = commands::xfeature_compress_gzip();
    assert_eq!(cmd, "XFEATURE COMPRESS GZIP\r\n");
}

// Command Length Validation (RFC 3977 §3.1)

#[test]
fn test_command_under_512_octets() {
    // RFC 3977 §3.1: "command line MUST NOT exceed 512 octets"
    // Most commands are well under this limit
    let cmd = commands::group("alt.test");
    assert!(cmd.len() <= 512);
}

#[test]
fn test_long_group_name_command_length() {
    // Even with a very long group name, should work
    let long_group = "a".repeat(400);
    let cmd = commands::group(&long_group);
    // GROUP + space + name + CRLF = 6 + 400 + 2 = 408 < 512
    assert!(cmd.len() <= 512);
}

#[test]
fn test_long_message_id_command_length() {
    // Very long message-ID
    let long_id = format!("<{}@example.com>", "x".repeat(450));
    let cmd = commands::article(&long_id);
    // This exceeds 512 but the library doesn't enforce the limit
    // The server will reject if too long
    assert!(!cmd.is_empty());
}

// Keyword Case (RFC 3977 §3.1)

#[test]
fn test_keywords_are_uppercase() {
    // RFC 3977 §3.1: Keywords are case-insensitive, but convention is UPPERCASE
    assert!(commands::group("test").starts_with("GROUP "));
    assert!(commands::article("1").starts_with("ARTICLE "));
    assert!(commands::head("1").starts_with("HEAD "));
    assert!(commands::body("1").starts_with("BODY "));
    assert!(commands::xover("1").starts_with("XOVER "));
    assert!(commands::quit().starts_with("QUIT"));
    assert!(commands::list().starts_with("LIST"));
    assert!(commands::stat("1").starts_with("STAT "));
    assert!(commands::compress_deflate().starts_with("COMPRESS "));
    assert!(commands::authinfo_user("u").starts_with("AUTHINFO USER "));
    assert!(commands::authinfo_pass("p").starts_with("AUTHINFO PASS "));
}

// All Commands End With CRLF

#[test]
fn test_all_new_commands_end_with_crlf() {
    // RFC 3977 §3.1: All commands must end with CRLF
    let commands_to_test = vec![
        commands::post().to_string(),
        commands::ihave("<msgid@example>"),
        commands::mode_reader().to_string(),
        commands::mode_stream().to_string(),
        commands::capabilities().to_string(),
        commands::help().to_string(),
        commands::date().to_string(),
        commands::newgroups("20240101", "000000"),
        commands::newnews("*", "20240101", "000000"),
        commands::next().to_string(),
        commands::last().to_string(),
        commands::hdr("Subject", "1-10"),
        commands::over("1-100"),
        commands::list_active("*"),
        commands::list_headers().to_string(),
        commands::listgroup("alt.test"),
        commands::authinfo_sasl("PLAIN"),
        commands::starttls().to_string(),
    ];

    for cmd in &commands_to_test {
        assert!(
            cmd.ends_with("\r\n"),
            "Command does not end with CRLF: {:?}",
            cmd
        );
        // Also verify exactly one CRLF
        assert_eq!(
            cmd.matches("\r\n").count(),
            1,
            "Command has multiple CRLFs: {:?}",
            cmd
        );
    }
}

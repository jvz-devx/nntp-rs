//! RFC 3977 Section 8.3 - OVER/XOVER Response Parsing Tests
//!
//! These tests verify compliance with XOVER response format:
//! - Tab-separated fields: article#, subject, from, date, message-id, references, bytes, lines
//! - Optional additional fields (like XREF) may be present

use nntp_rs::commands::parse_xover_line;

// Valid XOVER Line Parsing (RFC 3977 §8.3)

#[test]
fn test_xover_standard_8_fields() {
    // Standard XOVER format with exactly 8 fields
    let line = "12345\tTest Subject\tauthor@example.com\tMon, 1 Jan 2024 00:00:00 +0000\t<msgid@example>\t<ref@example>\t1024\t50";

    let entry = parse_xover_line(line).unwrap();

    assert_eq!(entry.article_number, 12345);
    assert_eq!(entry.subject, "Test Subject");
    assert_eq!(entry.author, "author@example.com");
    assert_eq!(entry.date, "Mon, 1 Jan 2024 00:00:00 +0000");
    assert_eq!(entry.message_id, "<msgid@example>");
    assert_eq!(entry.references, "<ref@example>");
    assert_eq!(entry.bytes, 1024);
    assert_eq!(entry.lines, 50);
}

#[test]
fn test_xover_with_extra_fields() {
    // Some servers add XREF or other fields after the 8 required fields
    let line =
        "12345\tSubject\tFrom\tDate\t<msgid>\t<refs>\t1000\t100\txref:server group:12345\textra";

    let entry = parse_xover_line(line).unwrap();

    // Should parse the first 8 fields correctly, ignoring extras
    assert_eq!(entry.article_number, 12345);
    assert_eq!(entry.subject, "Subject");
    assert_eq!(entry.bytes, 1000);
    assert_eq!(entry.lines, 100);
}

#[test]
fn test_xover_empty_references() {
    // References can be empty if article is not a reply
    let line = "12345\tSubject\tFrom\tDate\t<msgid>\t\t1000\t100";

    let entry = parse_xover_line(line).unwrap();

    assert_eq!(entry.references, "");
}

#[test]
fn test_xover_empty_subject() {
    // Subject can technically be empty
    let line = "12345\t\tFrom\tDate\t<msgid>\t<refs>\t1000\t100";

    let entry = parse_xover_line(line).unwrap();

    assert_eq!(entry.subject, "");
}

#[test]
fn test_xover_multiple_references() {
    // References field can contain multiple message-IDs
    let refs = "<ref1@a> <ref2@b> <ref3@c>";
    let line = format!("12345\tSubject\tFrom\tDate\t<msgid>\t{}\t1000\t100", refs);

    let entry = parse_xover_line(&line).unwrap();

    assert_eq!(entry.references, refs);
}

#[test]
fn test_xover_large_article_number() {
    // Very large article numbers
    let line = "999999999\tSubject\tFrom\tDate\t<msgid>\t\t1000\t100";

    let entry = parse_xover_line(line).unwrap();

    assert_eq!(entry.article_number, 999_999_999);
}

#[test]
fn test_xover_large_byte_count() {
    // Large article (multi-megabyte)
    let line = "12345\tSubject\tFrom\tDate\t<msgid>\t\t52428800\t1000000";

    let entry = parse_xover_line(line).unwrap();

    assert_eq!(entry.bytes, 52_428_800); // 50 MB
    assert_eq!(entry.lines, 1_000_000);
}

#[test]
fn test_xover_zero_bytes_lines() {
    // Edge case: 0 bytes and lines
    let line = "12345\tSubject\tFrom\tDate\t<msgid>\t\t0\t0";

    let entry = parse_xover_line(line).unwrap();

    assert_eq!(entry.bytes, 0);
    assert_eq!(entry.lines, 0);
}

#[test]
fn test_xover_complex_subject() {
    // Subject with special characters
    let line = "12345\tRe: [PATCH v2] Fix: \"bug\" in <module>\tFrom\tDate\t<msgid>\t\t1000\t100";

    let entry = parse_xover_line(line).unwrap();

    assert_eq!(entry.subject, "Re: [PATCH v2] Fix: \"bug\" in <module>");
}

#[test]
fn test_xover_complex_author() {
    // From header with name and email
    let line = "12345\tSubject\t\"John Doe\" <john@example.com>\tDate\t<msgid>\t\t1000\t100";

    let entry = parse_xover_line(line).unwrap();

    assert_eq!(entry.author, "\"John Doe\" <john@example.com>");
}

#[test]
fn test_xover_article_number_zero() {
    // RFC 3977 §8.3: Article number is 0 when requesting by message-id
    let line = "0\tSubject\tFrom\tDate\t<msgid>\t\t1000\t100";

    let entry = parse_xover_line(line).unwrap();

    assert_eq!(entry.article_number, 0);
}

// Invalid XOVER Line Parsing

#[test]
fn test_xover_missing_fields() {
    // Only 7 fields (missing lines count)
    let line = "12345\tSubject\tFrom\tDate\t<msgid>\t<refs>\t1000";

    assert!(parse_xover_line(line).is_err());
}

#[test]
fn test_xover_only_article_number() {
    // Only 1 field
    let line = "12345";

    assert!(parse_xover_line(line).is_err());
}

#[test]
fn test_xover_empty_line() {
    assert!(parse_xover_line("").is_err());
}

#[test]
fn test_xover_no_tabs() {
    // Spaces instead of tabs
    let line = "12345 Subject From Date <msgid> <refs> 1000 100";

    // This should fail because fields are space-separated, not tab-separated
    // Our implementation splits on tabs, so this becomes 1 field
    assert!(parse_xover_line(line).is_err());
}

// Graceful Handling of Malformed Data
//
// NOTE: These tests document CLIENT-SIDE DEFENSIVE PARSING, beyond RFC 3977.
// RFC 3977 §8.3 requires servers to send valid numeric values in the
// article-number, bytes, and lines fields. Servers MUST NOT send non-numeric
// data. However, our parser is lenient and defaults to 0 for unparseable values
// to protect against malformed server responses in the wild.
//
// This is NOT RFC-compliant behavior testing - it's robustness testing.

#[test]
fn test_xover_non_numeric_article_number() {
    // CLIENT-SIDE DEFENSIVE PARSING (not RFC 3977 compliant server behavior)
    // RFC 3977 requires numeric article number; we default to 0 for robustness
    let line = "abc\tSubject\tFrom\tDate\t<msgid>\t\t1000\t100";

    let entry = parse_xover_line(line).unwrap();

    assert_eq!(entry.article_number, 0); // Defaults to 0 on parse error
}

#[test]
fn test_xover_non_numeric_bytes() {
    // CLIENT-SIDE DEFENSIVE PARSING (not RFC 3977 compliant server behavior)
    // RFC 3977 requires numeric bytes field; we default to 0 for robustness
    let line = "12345\tSubject\tFrom\tDate\t<msgid>\t\tabc\t100";

    let entry = parse_xover_line(line).unwrap();

    assert_eq!(entry.bytes, 0); // Defaults to 0 on parse error
}

#[test]
fn test_xover_non_numeric_lines() {
    // CLIENT-SIDE DEFENSIVE PARSING (not RFC 3977 compliant server behavior)
    // RFC 3977 requires numeric lines field; we default to 0 for robustness
    let line = "12345\tSubject\tFrom\tDate\t<msgid>\t\t1000\txyz";

    let entry = parse_xover_line(line).unwrap();

    assert_eq!(entry.lines, 0); // Defaults to 0 on parse error
}

#[test]
fn test_xover_negative_numbers() {
    // CLIENT-SIDE DEFENSIVE PARSING (not RFC 3977 compliant server behavior)
    // RFC 3977 requires non-negative values; negative numbers fail usize parse
    let line = "12345\tSubject\tFrom\tDate\t<msgid>\t\t-1000\t-100";

    let entry = parse_xover_line(line).unwrap();

    // Negative numbers fail usize parse, default to 0
    assert_eq!(entry.bytes, 0);
    assert_eq!(entry.lines, 0);
}

// RFC 3977 §8.3 Specific Requirements

#[test]
fn test_xover_tab_separated() {
    // RFC requires TAB (\t) as separator
    let line = "1\ta\tb\tc\t<d>\te\t10\t5";
    let entry = parse_xover_line(line).unwrap();

    assert_eq!(entry.article_number, 1);
    assert_eq!(entry.subject, "a");
    assert_eq!(entry.author, "b");
}

#[test]
fn test_xover_preserves_internal_spaces() {
    // Spaces within fields should be preserved
    let line = "12345\tHello World Test\tJohn Doe <john@example.com>\tMon, 1 Jan 2024\t<msgid>\t<ref1> <ref2>\t1000\t100";

    let entry = parse_xover_line(line).unwrap();

    assert_eq!(entry.subject, "Hello World Test");
    assert_eq!(entry.author, "John Doe <john@example.com>");
    assert_eq!(entry.references, "<ref1> <ref2>");
}

#[test]
fn test_xover_unicode_subject() {
    // UTF-8 subjects
    let line = "12345\t日本語の件名\tauthor@example.com\tDate\t<msgid>\t\t1000\t100";

    let entry = parse_xover_line(line).unwrap();

    assert_eq!(entry.subject, "日本語の件名");
}

#[test]
fn test_xover_emoji_in_subject() {
    let line = "12345\t🎉 Celebration! 🎊\tauthor@example.com\tDate\t<msgid>\t\t1000\t100";

    let entry = parse_xover_line(line).unwrap();

    assert!(entry.subject.contains("🎉"));
}
#[test]
fn test_xover_giganews_format() {
    // Some servers include XREF as 9th field
    let line =
        "12345\tSubject\tFrom\tDate\t<msgid>\t\t1000\t100\txref:news.example.com group:12345";

    let entry = parse_xover_line(line).unwrap();

    assert_eq!(entry.article_number, 12345);
    // Extra fields should be ignored
}

#[test]
fn test_xover_empty_all_optional_fields() {
    // Subject, references can be empty
    let line = "12345\t\tFrom\tDate\t<msgid>\t\t1000\t100";

    let entry = parse_xover_line(line).unwrap();

    assert_eq!(entry.subject, "");
    assert_eq!(entry.references, "");
}

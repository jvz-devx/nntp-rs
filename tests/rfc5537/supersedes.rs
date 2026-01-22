//! RFC 5537 Section 5.4 - Supersedes Header Tests
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc5537#section-5.4
//! Reference: https://datatracker.ietf.org/doc/html/rfc5536#section-3.2.12

use nntp_rs::article::{parse_article, ArticleBuilder};

// PARSING TESTS

#[test]
fn test_parse_supersedes_basic() {
    let raw = r#"From: user@example.com
Subject: Updated article
Newsgroups: test.group
Date: Mon, 20 Jan 2025 12:00:00 +0000
Message-ID: <new123@example.com>
Path: news.example.com!not-for-mail
Supersedes: <old123@example.com>

This is the updated body.
"#;

    let article = parse_article(raw).unwrap();
    assert_eq!(
        article.headers.supersedes,
        Some("<old123@example.com>".to_string())
    );
    assert_eq!(article.headers.control, None);
}

#[test]
fn test_parse_supersedes_case_insensitive() {
    let raw = r#"From: user@example.com
Subject: Test
Newsgroups: test.group
Date: Mon, 20 Jan 2025 12:00:00 +0000
Message-ID: <new@example.com>
Path: news.example.com!not-for-mail
SUPERSEDES: <old@example.com>

Body.
"#;

    let article = parse_article(raw).unwrap();
    assert_eq!(
        article.headers.supersedes,
        Some("<old@example.com>".to_string())
    );
}

#[test]
fn test_parse_supersedes_with_whitespace() {
    let raw = r#"From: user@example.com
Subject: Test
Newsgroups: test.group
Date: Mon, 20 Jan 2025 12:00:00 +0000
Message-ID: <new@example.com>
Path: news.example.com!not-for-mail
Supersedes:   <old@example.com>

Body.
"#;

    let article = parse_article(raw).unwrap();
    assert_eq!(
        article.headers.supersedes,
        Some("<old@example.com>".to_string())
    );
}

#[test]
fn test_parse_supersedes_complex_message_id() {
    let raw = r#"From: user@example.com
Subject: Test
Newsgroups: test.group
Date: Mon, 20 Jan 2025 12:00:00 +0000
Message-ID: <new@example.com>
Path: news.example.com!not-for-mail
Supersedes: <abc-123.xyz_789@sub.domain.example.com>

Body.
"#;

    let article = parse_article(raw).unwrap();
    assert_eq!(
        article.headers.supersedes,
        Some("<abc-123.xyz_789@sub.domain.example.com>".to_string())
    );
}

#[test]
fn test_parse_without_supersedes() {
    let raw = r#"From: user@example.com
Subject: Test
Newsgroups: test.group
Date: Mon, 20 Jan 2025 12:00:00 +0000
Message-ID: <msg@example.com>
Path: news.example.com!not-for-mail

Body.
"#;

    let article = parse_article(raw).unwrap();
    assert_eq!(article.headers.supersedes, None);
}

// BUILDER TESTS

#[test]
fn test_builder_supersedes_basic() {
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Updated article")
        .newsgroups(vec!["test.group"])
        .body("This is the updated body.")
        .supersedes("<old123@example.com>")
        .build()
        .unwrap();

    assert_eq!(
        article.headers.supersedes,
        Some("<old123@example.com>".to_string())
    );
    assert_eq!(article.headers.control, None);
}

#[test]
fn test_builder_supersedes_serialization() {
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Updated")
        .newsgroups(vec!["test.group"])
        .body("Body.")
        .supersedes("<old@example.com>")
        .build()
        .unwrap();

    let serialized = article.serialize_for_posting().unwrap();
    assert!(serialized.contains("Supersedes: <old@example.com>\r\n"));
}

#[test]
fn test_builder_without_supersedes() {
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Regular article")
        .newsgroups(vec!["test.group"])
        .body("Body.")
        .build()
        .unwrap();

    assert_eq!(article.headers.supersedes, None);

    let serialized = article.serialize_for_posting().unwrap();
    assert!(!serialized.contains("Supersedes:"));
}

// MUTUAL EXCLUSIVITY TESTS (RFC 5536 Section 3.2.12)

#[test]
fn test_supersedes_mutually_exclusive_with_control() {
    // RFC 5536 Section 3.2.12: Article MUST NOT have both Supersedes and Control
    let result = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Test")
        .newsgroups(vec!["test.group"])
        .body("Body.")
        .supersedes("<old@example.com>")
        .control("cancel <other@example.com>")
        .build();

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Supersedes"));
    assert!(err_msg.contains("Control"));
}

#[test]
fn test_control_mutually_exclusive_with_supersedes() {
    // Test the opposite order
    let result = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Test")
        .newsgroups(vec!["test.group"])
        .body("Body.")
        .control("newgroup test.new")
        .supersedes("<old@example.com>")
        .build();

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Supersedes"));
    assert!(err_msg.contains("Control"));
}

#[test]
fn test_supersedes_without_control_allowed() {
    let result = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Test")
        .newsgroups(vec!["test.group"])
        .body("Body.")
        .supersedes("<old@example.com>")
        .build();

    assert!(result.is_ok());
}

#[test]
fn test_control_without_supersedes_allowed() {
    let result = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Test")
        .newsgroups(vec!["test.group"])
        .body("Body.")
        .control("cancel <msg@example.com>")
        .build();

    assert!(result.is_ok());
}

// REAL-WORLD EXAMPLES

#[test]
fn test_real_world_supersedes_article() {
    // Complete supersedes article with typical headers
    let article = ArticleBuilder::new()
        .from("John Doe <john@example.com>")
        .subject("Corrected: Important announcement")
        .newsgroups(vec!["comp.lang.rust", "comp.programming"])
        .body("This is the corrected version of my previous announcement.")
        .supersedes("<original-123@example.com>")
        .organization("Example Organization")
        .references(vec![
            "<thread-root@example.com>".to_string(),
            "<previous-reply@example.com>".to_string(),
        ])
        .build()
        .unwrap();

    assert_eq!(
        article.headers.supersedes,
        Some("<original-123@example.com>".to_string())
    );
    assert_eq!(article.headers.control, None);
    assert_eq!(article.headers.from, "John Doe <john@example.com>");
    assert_eq!(
        article.headers.organization,
        Some("Example Organization".to_string())
    );
}

#[test]
fn test_real_world_supersedes_serialization() {
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Corrected article")
        .newsgroups(vec!["test.group"])
        .body("This replaces the previous article.")
        .supersedes("<abc123@example.com>")
        .build()
        .unwrap();

    let serialized = article.serialize_for_posting().unwrap();

    // Verify required headers present
    assert!(serialized.contains("From: user@example.com\r\n"));
    assert!(serialized.contains("Subject: Corrected article\r\n"));
    assert!(serialized.contains("Newsgroups: test.group\r\n"));
    assert!(serialized.contains("Supersedes: <abc123@example.com>\r\n"));

    // Verify blank line separator and body
    assert!(serialized.contains("\r\n\r\n"));
    assert!(serialized.contains("This replaces the previous article."));
}

// EDGE CASES

#[test]
fn test_supersedes_empty_string() {
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Test")
        .newsgroups(vec!["test.group"])
        .body("Body.")
        .supersedes("")
        .build()
        .unwrap();

    // Empty string is technically valid, though not useful
    assert_eq!(article.headers.supersedes, Some("".to_string()));
}

#[test]
fn test_supersedes_without_angle_brackets() {
    // RFC allows message-IDs with or without angle brackets in headers
    let raw = r#"From: user@example.com
Subject: Test
Newsgroups: test.group
Date: Mon, 20 Jan 2025 12:00:00 +0000
Message-ID: <new@example.com>
Path: news.example.com!not-for-mail
Supersedes: old123@example.com

Body.
"#;

    let article = parse_article(raw).unwrap();
    assert_eq!(
        article.headers.supersedes,
        Some("old123@example.com".to_string())
    );
}

#[test]
fn test_supersedes_multiline_folding() {
    // RFC 5322 allows header folding with continuation lines
    let raw = r#"From: user@example.com
Subject: Test
Newsgroups: test.group
Date: Mon, 20 Jan 2025 12:00:00 +0000
Message-ID: <new@example.com>
Path: news.example.com!not-for-mail
Supersedes: <very-long-message-id-
 that-continues-on-next-line@example.com>

Body.
"#;

    let article = parse_article(raw).unwrap();
    assert!(article.headers.supersedes.is_some());
    // Header folding should be handled by unfold_header
}

//! RFC 5536 Article Builder Tests
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc5536
//!
//! Tests for ArticleBuilder and article serialization functionality.

use nntp_rs::article::ArticleBuilder;
#[test]
fn test_builder_minimal() {
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Test Subject")
        .newsgroups(vec!["test.group"])
        .body("Test body")
        .build()
        .unwrap();

    assert_eq!(article.headers.from, "user@example.com");
    assert_eq!(article.headers.subject, "Test Subject");
    assert_eq!(article.headers.newsgroups, vec!["test.group"]);
    assert_eq!(article.body, "Test body");
    assert!(!article.headers.date.is_empty());
    assert!(!article.headers.message_id.is_empty());
    assert_eq!(article.headers.path, "not-for-mail");
}

#[test]
fn test_builder_multiple_newsgroups() {
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Cross-posted article")
        .newsgroups(vec!["comp.lang.rust", "comp.programming"])
        .body("Test")
        .build()
        .unwrap();

    assert_eq!(article.headers.newsgroups.len(), 2);
    assert_eq!(article.headers.newsgroups[0], "comp.lang.rust");
    assert_eq!(article.headers.newsgroups[1], "comp.programming");
}

#[test]
fn test_builder_add_newsgroup() {
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Test")
        .add_newsgroup("test.group1")
        .add_newsgroup("test.group2")
        .body("Test")
        .build()
        .unwrap();

    assert_eq!(article.headers.newsgroups.len(), 2);
    assert!(article
        .headers
        .newsgroups
        .contains(&"test.group1".to_string()));
    assert!(article
        .headers
        .newsgroups
        .contains(&"test.group2".to_string()));
}

#[test]
fn test_builder_with_optional_headers() {
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Test")
        .newsgroups(vec!["test.group"])
        .body("Test body")
        .organization("Test Organization")
        .user_agent("nntp-rs/0.1.0")
        .reply_to("reply@example.com")
        .build()
        .unwrap();

    assert_eq!(
        article.headers.organization,
        Some("Test Organization".to_string())
    );
    assert_eq!(
        article.headers.user_agent,
        Some("nntp-rs/0.1.0".to_string())
    );
    assert_eq!(
        article.headers.reply_to,
        Some("reply@example.com".to_string())
    );
}

// Auto-Generation Tests

#[test]
fn test_builder_auto_generates_date() {
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Test")
        .newsgroups(vec!["test.group"])
        .body("Test")
        .build()
        .unwrap();

    // Should have a date
    assert!(!article.headers.date.is_empty());
    // Should be in RFC 5322 format (contains comma and timezone)
    assert!(article.headers.date.contains(','));
    assert!(article.headers.date.contains('+') || article.headers.date.contains('-'));
}

#[test]
fn test_builder_custom_date() {
    let custom_date = "Mon, 20 Jan 2025 12:00:00 +0000";
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Test")
        .newsgroups(vec!["test.group"])
        .body("Test")
        .date(custom_date)
        .build()
        .unwrap();

    assert_eq!(article.headers.date, custom_date);
}

#[test]
fn test_builder_auto_generates_message_id() {
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Test")
        .newsgroups(vec!["test.group"])
        .body("Test")
        .build()
        .unwrap();

    // Should have a message-id
    assert!(!article.headers.message_id.is_empty());
    // Should be in <local@domain> format
    assert!(article.headers.message_id.starts_with('<'));
    assert!(article.headers.message_id.ends_with('>'));
    assert!(article.headers.message_id.contains('@'));
}

#[test]
fn test_builder_message_id_uses_from_domain() {
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Test")
        .newsgroups(vec!["test.group"])
        .body("Test")
        .build()
        .unwrap();

    // Message-ID should use domain from From header
    assert!(article.headers.message_id.contains("@example.com>"));
}

#[test]
fn test_builder_message_id_handles_from_with_name() {
    let article = ArticleBuilder::new()
        .from("John Doe <user@example.com>")
        .subject("Test")
        .newsgroups(vec!["test.group"])
        .body("Test")
        .build()
        .unwrap();

    // Should extract domain from email in angle brackets
    assert!(article.headers.message_id.contains("@example.com>"));
}

#[test]
fn test_builder_custom_message_id() {
    let custom_id = "<custom123@example.com>";
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Test")
        .newsgroups(vec!["test.group"])
        .body("Test")
        .message_id(custom_id)
        .build()
        .unwrap();

    assert_eq!(article.headers.message_id, custom_id);
}

#[test]
fn test_builder_default_path() {
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Test")
        .newsgroups(vec!["test.group"])
        .body("Test")
        .build()
        .unwrap();

    assert_eq!(article.headers.path, "not-for-mail");
}

#[test]
fn test_builder_custom_path() {
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Test")
        .newsgroups(vec!["test.group"])
        .body("Test")
        .path("news.example.com!not-for-mail")
        .build()
        .unwrap();

    assert_eq!(article.headers.path, "news.example.com!not-for-mail");
}
#[test]
fn test_builder_with_references() {
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Re: Original Subject")
        .newsgroups(vec!["test.group"])
        .body("Reply body")
        .references(vec!["<msg1@example.com>", "<msg2@example.com>"])
        .build()
        .unwrap();

    assert!(article.headers.references.is_some());
    let refs = article.headers.references.unwrap();
    assert_eq!(refs.len(), 2);
    assert_eq!(refs[0], "<msg1@example.com>");
    assert_eq!(refs[1], "<msg2@example.com>");
}

#[test]
fn test_builder_with_followup_to() {
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Test")
        .newsgroups(vec!["test.group"])
        .body("Test")
        .followup_to(vec!["test.followup"])
        .build()
        .unwrap();

    assert!(article.headers.followup_to.is_some());
    assert_eq!(article.headers.followup_to.unwrap(), vec!["test.followup"]);
}
#[test]
fn test_builder_missing_from_fails() {
    let result = ArticleBuilder::new()
        .subject("Test")
        .newsgroups(vec!["test.group"])
        .body("Test")
        .build();

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("From header is required"));
}

#[test]
fn test_builder_missing_subject_fails() {
    let result = ArticleBuilder::new()
        .from("user@example.com")
        .newsgroups(vec!["test.group"])
        .body("Test")
        .build();

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Subject header is required"));
}

#[test]
fn test_builder_missing_newsgroups_fails() {
    let result = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Test")
        .body("Test")
        .build();

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("At least one newsgroup is required"));
}

#[test]
fn test_builder_empty_newsgroups_fails() {
    let result = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Test")
        .newsgroups(Vec::<String>::new())
        .body("Test")
        .build();

    assert!(result.is_err());
}
#[test]
fn test_serialize_for_posting_basic() {
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Test Subject")
        .newsgroups(vec!["test.group"])
        .body("Test body")
        .build()
        .unwrap();

    let serialized = article.serialize_for_posting().unwrap();

    // Should contain required headers
    assert!(serialized.contains("From: user@example.com\r\n"));
    assert!(serialized.contains("Subject: Test Subject\r\n"));
    assert!(serialized.contains("Newsgroups: test.group\r\n"));
    assert!(serialized.contains("Path: not-for-mail\r\n"));

    // Should have CRLF line endings
    assert!(serialized.contains("\r\n"));

    // Should have blank line between headers and body
    assert!(serialized.contains("\r\n\r\n"));

    // Should contain body
    assert!(serialized.contains("Test body"));
}

#[test]
fn test_serialize_for_posting_multiple_newsgroups() {
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Test")
        .newsgroups(vec!["comp.lang.rust", "comp.programming"])
        .body("Test")
        .build()
        .unwrap();

    let serialized = article.serialize_for_posting().unwrap();

    assert!(serialized.contains("Newsgroups: comp.lang.rust,comp.programming\r\n"));
}

#[test]
fn test_serialize_for_posting_with_optional_headers() {
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Test")
        .newsgroups(vec!["test.group"])
        .body("Test")
        .organization("Test Org")
        .user_agent("nntp-rs/0.1.0")
        .build()
        .unwrap();

    let serialized = article.serialize_for_posting().unwrap();

    assert!(serialized.contains("Organization: Test Org\r\n"));
    assert!(serialized.contains("User-Agent: nntp-rs/0.1.0\r\n"));
}

#[test]
fn test_serialize_for_posting_dot_stuffing() {
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Test")
        .newsgroups(vec!["test.group"])
        .body(".This line starts with a dot\nNormal line\n.Another dotted line")
        .build()
        .unwrap();

    let serialized = article.serialize_for_posting().unwrap();

    // Lines starting with '.' should be dot-stuffed
    assert!(serialized.contains("..This line starts with a dot\r\n"));
    assert!(serialized.contains("Normal line\r\n"));
    assert!(serialized.contains("..Another dotted line\r\n"));
}

#[test]
fn test_serialize_for_posting_with_references() {
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Re: Test")
        .newsgroups(vec!["test.group"])
        .body("Reply")
        .references(vec!["<msg1@example.com>", "<msg2@example.com>"])
        .build()
        .unwrap();

    let serialized = article.serialize_for_posting().unwrap();

    assert!(serialized.contains("References: <msg1@example.com> <msg2@example.com>\r\n"));
}

#[test]
fn test_build_for_posting_shortcut() {
    let serialized = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Test")
        .newsgroups(vec!["test.group"])
        .body("Test body")
        .build_for_posting()
        .unwrap();

    // Should work the same as build() + serialize_for_posting()
    assert!(serialized.contains("From: user@example.com\r\n"));
    assert!(serialized.contains("\r\n\r\n"));
    assert!(serialized.contains("Test body"));
}
#[test]
fn test_builder_with_extra_headers() {
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Test")
        .newsgroups(vec!["test.group"])
        .body("Test")
        .extra_header("X-Custom-Header", "Custom Value")
        .extra_header("X-Another", "Another Value")
        .build()
        .unwrap();

    assert_eq!(article.headers.extra.len(), 2);
    assert_eq!(
        article.headers.extra.get("X-Custom-Header"),
        Some(&"Custom Value".to_string())
    );
    assert_eq!(
        article.headers.extra.get("X-Another"),
        Some(&"Another Value".to_string())
    );
}

#[test]
fn test_serialize_includes_extra_headers() {
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Test")
        .newsgroups(vec!["test.group"])
        .body("Test")
        .extra_header("X-Mailer", "nntp-rs")
        .build()
        .unwrap();

    let serialized = article.serialize_for_posting().unwrap();

    assert!(serialized.contains("X-Mailer: nntp-rs\r\n"));
}
#[test]
fn test_builder_control_cancel() {
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject("cmsg cancel <original@example.com>")
        .newsgroups(vec!["test.group"])
        .body("Cancel message")
        .control("cancel <original@example.com>")
        .build()
        .unwrap();

    assert_eq!(
        article.headers.control,
        Some("cancel <original@example.com>".to_string())
    );
}

#[test]
fn test_builder_moderated_approved() {
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Test")
        .newsgroups(vec!["comp.lang.rust.moderated"])
        .body("Approved post")
        .approved("moderator@example.com")
        .build()
        .unwrap();

    assert_eq!(
        article.headers.approved,
        Some("moderator@example.com".to_string())
    );
}


#[test]
fn test_builder_empty_body() {
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Empty article")
        .newsgroups(vec!["test.group"])
        .body("")
        .build()
        .unwrap();

    assert_eq!(article.body, "");
}

#[test]
fn test_builder_multiline_body() {
    let body = "Line 1\nLine 2\nLine 3";
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Test")
        .newsgroups(vec!["test.group"])
        .body(body)
        .build()
        .unwrap();

    assert_eq!(article.body, body);
}

#[test]
fn test_builder_body_with_unicode() {
    let body = "Unicode test: 你好 мир 🦀";
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject("Unicode Test")
        .newsgroups(vec!["test.group"])
        .body(body)
        .build()
        .unwrap();

    assert_eq!(article.body, body);
}

#[test]
fn test_builder_long_subject() {
    let long_subject = "A".repeat(200);
    let article = ArticleBuilder::new()
        .from("user@example.com")
        .subject(&long_subject)
        .newsgroups(vec!["test.group"])
        .body("Test")
        .build()
        .unwrap();

    assert_eq!(article.headers.subject, long_subject);
}

#[test]
fn test_builder_complex_from_address() {
    let article = ArticleBuilder::new()
        .from("\"John Doe\" <john.doe@example.com>")
        .subject("Test")
        .newsgroups(vec!["test.group"])
        .body("Test")
        .build()
        .unwrap();

    assert_eq!(article.headers.from, "\"John Doe\" <john.doe@example.com>");
}

#[test]
fn test_builder_real_world_post() {
    let article = ArticleBuilder::new()
        .from("John Doe <john@example.com>")
        .subject("Question about Rust lifetime")
        .newsgroups(vec!["comp.lang.rust"])
        .organization("Example Inc")
        .user_agent("nntp-rs/0.1.0")
        .body("I have a question about lifetime annotations...\n\nCan someone help?")
        .build()
        .unwrap();

    let serialized = article.serialize_for_posting().unwrap();

    // Verify complete article structure
    assert!(serialized.contains("From: John Doe <john@example.com>\r\n"));
    assert!(serialized.contains("Subject: Question about Rust lifetime\r\n"));
    assert!(serialized.contains("Newsgroups: comp.lang.rust\r\n"));
    assert!(serialized.contains("Organization: Example Inc\r\n"));
    assert!(serialized.contains("User-Agent: nntp-rs/0.1.0\r\n"));
    assert!(serialized.contains("\r\n\r\n"));
    assert!(serialized.contains("I have a question about lifetime annotations"));
}

#[test]
fn test_builder_real_world_reply() {
    let article = ArticleBuilder::new()
        .from("Jane Smith <jane@example.com>")
        .subject("Re: Question about Rust lifetime")
        .newsgroups(vec!["comp.lang.rust"])
        .references(vec!["<original@example.com>", "<reply1@example.com>"])
        .reply_to("jane@example.com")
        .organization("Test Corp")
        .body("You can solve this by using 'static lifetime...")
        .build()
        .unwrap();

    assert_eq!(article.headers.references.as_ref().unwrap().len(), 2);
    assert!(article.headers.reply_to.is_some());
}

#[test]
fn test_builder_binary_newsgroup_post() {
    let article = ArticleBuilder::new()
        .from("uploader@example.com")
        .subject("[1/10] - \"file.rar\" yEnc (1/100)")
        .newsgroups(vec!["alt.binaries.test"])
        .user_agent("nntp-rs/0.1.0")
        .body("=ybegin part=1 total=100 line=128 size=1000000 name=file.rar\n...")
        .build()
        .unwrap();

    assert!(article.headers.subject.contains("yEnc"));
    assert_eq!(article.headers.newsgroups[0], "alt.binaries.test");
}

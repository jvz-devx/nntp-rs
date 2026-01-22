//! RFC 5536 Section 3 - Header Tests
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc5536#section-3
//!
//! Tests for parsing article headers conforming to RFC 5536.

use nntp_rs::{parse_article, parse_headers};
#[test]
fn test_parse_minimal_required_headers() {
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc123@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com!not-for-mail
Subject: Test Article";

    let headers = parse_headers(headers_text).unwrap();

    assert_eq!(headers.date, "Mon, 20 Jan 2025 12:00:00 +0000");
    assert_eq!(headers.from, "user@example.com");
    assert_eq!(headers.message_id, "<abc123@example.com>");
    assert_eq!(headers.newsgroups, vec!["comp.lang.rust"]);
    assert_eq!(headers.path, "news.example.com!not-for-mail");
    assert_eq!(headers.subject, "Test Article");
}

#[test]
fn test_parse_headers_case_insensitive() {
    // RFC 5536: Header names are case-insensitive
    let headers_text = "\
DATE: Mon, 20 Jan 2025 12:00:00 +0000
fRoM: user@example.com
message-id: <abc123@example.com>
NEWSGROUPS: comp.lang.rust
path: news.example.com!not-for-mail
Subject: Test";

    let headers = parse_headers(headers_text).unwrap();

    assert_eq!(headers.date, "Mon, 20 Jan 2025 12:00:00 +0000");
    assert_eq!(headers.from, "user@example.com");
    assert_eq!(headers.message_id, "<abc123@example.com>");
    assert_eq!(headers.subject, "Test");
}

#[test]
fn test_parse_headers_with_space_after_colon() {
    // RFC 5536: At least one space should follow the colon
    let headers_text = "\
Date:  Mon, 20 Jan 2025 12:00:00 +0000
From:user@example.com
Message-ID:   <abc123@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com
Subject:Test Subject";

    let headers = parse_headers(headers_text).unwrap();

    assert_eq!(headers.date, "Mon, 20 Jan 2025 12:00:00 +0000");
    assert_eq!(headers.from, "user@example.com");
    assert_eq!(headers.message_id, "<abc123@example.com>");
    assert_eq!(headers.subject, "Test Subject");
}

#[test]
fn test_missing_required_header_date() {
    let headers_text = "\
From: user@example.com
Message-ID: <abc123@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com
Subject: Test";

    let result = parse_headers(headers_text);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Date"));
}

#[test]
fn test_missing_required_header_message_id() {
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Newsgroups: comp.lang.rust
Path: news.example.com
Subject: Test";

    let result = parse_headers(headers_text);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Message-ID"));
}
#[test]
fn test_header_folding_with_space() {
    // RFC 5322: Continuation lines start with whitespace
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc123@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com
Subject: This is a very long subject line
 that continues on the next line";

    let headers = parse_headers(headers_text).unwrap();

    assert_eq!(
        headers.subject,
        "This is a very long subject line that continues on the next line"
    );
}

#[test]
fn test_header_folding_with_tab() {
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc123@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com
Subject: This is a subject
\twith tab continuation";

    let headers = parse_headers(headers_text).unwrap();

    assert_eq!(headers.subject, "This is a subject with tab continuation");
}

#[test]
fn test_header_folding_multiple_lines() {
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc123@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com
Subject: Line one
 line two
 line three
 line four";

    let headers = parse_headers(headers_text).unwrap();

    assert_eq!(headers.subject, "Line one line two line three line four");
}

#[test]
fn test_header_folding_with_crlf() {
    // Test with CRLF line endings
    let headers_text = "Date: Mon, 20 Jan 2025 12:00:00 +0000\r\nFrom: user@example.com\r\nMessage-ID: <abc123@example.com>\r\nNewsgroups: comp.lang.rust\r\nPath: news.example.com\r\nSubject: Folded line\r\n continues here";

    let headers = parse_headers(headers_text).unwrap();

    assert_eq!(headers.subject, "Folded line continues here");
}

// Multi-Value Header Tests

#[test]
fn test_multiple_newsgroups() {
    // RFC 5536: Newsgroups are comma-separated
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc123@example.com>
Newsgroups: comp.lang.rust,comp.lang.c,comp.lang.python
Path: news.example.com
Subject: Test";

    let headers = parse_headers(headers_text).unwrap();

    assert_eq!(headers.newsgroups.len(), 3);
    assert_eq!(headers.newsgroups[0], "comp.lang.rust");
    assert_eq!(headers.newsgroups[1], "comp.lang.c");
    assert_eq!(headers.newsgroups[2], "comp.lang.python");
}

#[test]
fn test_newsgroups_with_whitespace() {
    // RFC 5536: Optional FWS around commas MUST be accepted
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc123@example.com>
Newsgroups: comp.lang.rust, comp.lang.c  ,  comp.lang.python
Path: news.example.com
Subject: Test";

    let headers = parse_headers(headers_text).unwrap();

    assert_eq!(headers.newsgroups.len(), 3);
    assert_eq!(headers.newsgroups[0], "comp.lang.rust");
    assert_eq!(headers.newsgroups[1], "comp.lang.c");
    assert_eq!(headers.newsgroups[2], "comp.lang.python");
}

#[test]
fn test_references_multiple_message_ids() {
    // RFC 5536: References are space-separated message-IDs
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <reply@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com
Subject: Re: Test
References: <msg1@example.com> <msg2@example.com> <msg3@example.com>";

    let headers = parse_headers(headers_text).unwrap();

    let refs = headers.references.unwrap();
    assert_eq!(refs.len(), 3);
    assert_eq!(refs[0], "<msg1@example.com>");
    assert_eq!(refs[1], "<msg2@example.com>");
    assert_eq!(refs[2], "<msg3@example.com>");
}

#[test]
fn test_followup_to_comma_separated() {
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com
Subject: Test
Followup-To: comp.lang.c,alt.test";

    let headers = parse_headers(headers_text).unwrap();

    let followup = headers.followup_to.unwrap();
    assert_eq!(followup.len(), 2);
    assert_eq!(followup[0], "comp.lang.c");
    assert_eq!(followup[1], "alt.test");
}
#[test]
fn test_optional_headers_present() {
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com
Subject: Test
Organization: Test Org
Reply-To: reply@example.com
User-Agent: test-client/1.0
Lines: 42";

    let headers = parse_headers(headers_text).unwrap();

    assert_eq!(headers.organization.unwrap(), "Test Org");
    assert_eq!(headers.reply_to.unwrap(), "reply@example.com");
    assert_eq!(headers.user_agent.unwrap(), "test-client/1.0");
    assert_eq!(headers.lines.unwrap(), 42);
}

#[test]
fn test_optional_headers_absent() {
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com
Subject: Test";

    let headers = parse_headers(headers_text).unwrap();

    assert!(headers.organization.is_none());
    assert!(headers.reply_to.is_none());
    assert!(headers.user_agent.is_none());
    assert!(headers.lines.is_none());
    assert!(headers.references.is_none());
}

#[test]
fn test_control_header() {
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <cancel@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com
Subject: cancel <original@example.com>
Control: cancel <original@example.com>";

    let headers = parse_headers(headers_text).unwrap();

    assert_eq!(headers.control.unwrap(), "cancel <original@example.com>");
}

#[test]
fn test_approved_header() {
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc@example.com>
Newsgroups: comp.lang.rust.moderated
Path: news.example.com
Subject: Approved Post
Approved: moderator@example.com";

    let headers = parse_headers(headers_text).unwrap();

    assert_eq!(headers.approved.unwrap(), "moderator@example.com");
}

#[test]
fn test_expires_header() {
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com
Subject: Test
Expires: Mon, 27 Jan 2025 12:00:00 +0000";

    let headers = parse_headers(headers_text).unwrap();

    assert_eq!(headers.expires.unwrap(), "Mon, 27 Jan 2025 12:00:00 +0000");
}

#[test]
fn test_distribution_header() {
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com
Subject: Test
Distribution: local";

    let headers = parse_headers(headers_text).unwrap();

    assert_eq!(headers.distribution.unwrap(), "local");
}

#[test]
fn test_keywords_header() {
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com
Subject: Test
Keywords: rust, programming, test";

    let headers = parse_headers(headers_text).unwrap();

    assert_eq!(headers.keywords.unwrap(), "rust, programming, test");
}

#[test]
fn test_summary_header() {
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com
Subject: Test
Summary: A brief summary of the article";

    let headers = parse_headers(headers_text).unwrap();

    assert_eq!(headers.summary.unwrap(), "A brief summary of the article");
}

#[test]
fn test_xref_header() {
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com
Subject: Test
Xref: news.example.com comp.lang.rust:12345";

    let headers = parse_headers(headers_text).unwrap();

    assert_eq!(
        headers.xref.unwrap(),
        "news.example.com comp.lang.rust:12345"
    );
}
#[test]
fn test_extra_headers() {
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com
Subject: Test
X-Custom-Header: custom value
X-Another: another value
X-Trace: trace information";

    let headers = parse_headers(headers_text).unwrap();

    assert_eq!(headers.extra.len(), 3);
    assert_eq!(
        headers.extra.get("x-custom-header").unwrap(),
        "custom value"
    );
    assert_eq!(headers.extra.get("x-another").unwrap(), "another value");
    assert_eq!(headers.extra.get("x-trace").unwrap(), "trace information");
}

#[test]
fn test_mixed_standard_and_extra_headers() {
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
X-Priority: high
Message-ID: <abc@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com
X-Mailer: TestMailer/1.0
Subject: Test
Organization: Test Org";

    let headers = parse_headers(headers_text).unwrap();

    // Standard headers
    assert_eq!(headers.organization.unwrap(), "Test Org");
    assert_eq!(headers.subject, "Test");

    // Extra headers
    assert_eq!(headers.extra.get("x-priority").unwrap(), "high");
    assert_eq!(headers.extra.get("x-mailer").unwrap(), "TestMailer/1.0");
}
#[test]
fn test_parse_complete_article_with_lf() {
    let article_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com
Subject: Test Article

This is the article body.
It has multiple lines.
End of body.";

    let article = parse_article(article_text).unwrap();

    assert_eq!(article.headers.subject, "Test Article");
    assert_eq!(article.headers.from, "user@example.com");
    assert_eq!(
        article.body,
        "This is the article body.\nIt has multiple lines.\nEnd of body."
    );
    assert!(article.raw().is_some());
}

#[test]
fn test_parse_complete_article_with_crlf() {
    let article_text = "Date: Mon, 20 Jan 2025 12:00:00 +0000\r\nFrom: user@example.com\r\nMessage-ID: <abc@example.com>\r\nNewsgroups: comp.lang.rust\r\nPath: news.example.com\r\nSubject: Test\r\n\r\nBody text here.";

    let article = parse_article(article_text).unwrap();

    assert_eq!(article.headers.subject, "Test");
    assert_eq!(article.body, "Body text here.");
}

#[test]
fn test_parse_article_no_body() {
    let article_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com
Subject: Test

";

    let article = parse_article(article_text).unwrap();

    assert_eq!(article.headers.subject, "Test");
    assert_eq!(article.body, "");
}

#[test]
fn test_parse_article_with_empty_lines_in_body() {
    let article_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com
Subject: Test

First paragraph.

Second paragraph.

Third paragraph.";

    let article = parse_article(article_text).unwrap();

    assert_eq!(
        article.body,
        "First paragraph.\n\nSecond paragraph.\n\nThird paragraph."
    );
}


#[test]
fn test_empty_header_value() {
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com
Subject:
Organization:";

    let headers = parse_headers(headers_text).unwrap();

    assert_eq!(headers.subject, "");
    assert_eq!(headers.organization.unwrap(), "");
}

#[test]
fn test_lines_header_invalid_number() {
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com
Subject: Test
Lines: not-a-number";

    let headers = parse_headers(headers_text).unwrap();

    // Invalid Lines value should be ignored (None)
    assert!(headers.lines.is_none());
}

#[test]
fn test_header_with_colon_in_value() {
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com
Subject: RFC 5536: Article Format";

    let headers = parse_headers(headers_text).unwrap();

    assert_eq!(headers.subject, "RFC 5536: Article Format");
}

#[test]
fn test_very_long_subject() {
    let long_subject = "A".repeat(500);
    let headers_text = format!(
        "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com
Subject: {}",
        long_subject
    );

    let headers = parse_headers(&headers_text).unwrap();

    assert_eq!(headers.subject, long_subject);
}

// Real-World Examples

#[test]
fn test_real_world_usenet_article() {
    // Typical Usenet article structure
    let article_text = "\
Path: news.example.com!feed.example.org!not-for-mail
From: John Doe <john@example.com>
Newsgroups: comp.lang.rust
Subject: Question about error handling
Date: Mon, 20 Jan 2025 14:30:00 +0000
Organization: Example Organization
Message-ID: <20250120143000.12345@example.com>
User-Agent: NewsReader/2.0
Lines: 15

Hi everyone,

I have a question about error handling in Rust.
What's the best way to handle multiple error types?

Thanks in advance!

--
John Doe
john@example.com";

    let article = parse_article(article_text).unwrap();

    assert_eq!(article.headers.from, "John Doe <john@example.com>");
    assert_eq!(article.headers.newsgroups, vec!["comp.lang.rust"]);
    assert_eq!(article.headers.subject, "Question about error handling");
    assert_eq!(
        article.headers.organization.unwrap(),
        "Example Organization"
    );
    assert_eq!(article.headers.user_agent.unwrap(), "NewsReader/2.0");
    assert_eq!(article.headers.lines.unwrap(), 15);
    assert!(article.body.contains("question about error handling"));
}

#[test]
fn test_reply_with_references() {
    let article_text = "\
Date: Mon, 20 Jan 2025 15:00:00 +0000
From: Jane Smith <jane@example.com>
Message-ID: <reply123@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com
Subject: Re: Question about error handling
References: <20250120143000.12345@example.com>

Original poster wrote:
> What's the best way to handle multiple error types?

You can use the thiserror crate...";

    let article = parse_article(article_text).unwrap();

    assert_eq!(article.headers.subject, "Re: Question about error handling");
    let refs = article.headers.references.unwrap();
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0], "<20250120143000.12345@example.com>");
}

#[test]
fn test_crossposted_article() {
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc@example.com>
Newsgroups: comp.lang.rust,comp.programming,alt.test
Path: news.example.com
Subject: Cross-posted Article
Followup-To: comp.lang.rust";

    let headers = parse_headers(headers_text).unwrap();

    assert_eq!(headers.newsgroups.len(), 3);
    assert!(headers.newsgroups.contains(&"comp.lang.rust".to_string()));
    assert!(headers.newsgroups.contains(&"comp.programming".to_string()));
    assert!(headers.newsgroups.contains(&"alt.test".to_string()));

    let followup = headers.followup_to.unwrap();
    assert_eq!(followup.len(), 1);
    assert_eq!(followup[0], "comp.lang.rust");
}
#[test]
fn test_parse_headers_with_encoded_from_base64() {
    // Test Base64-encoded From header
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: =?UTF-8?B?QW5kcsOp?= <andre@example.com>
Message-ID: <abc123@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com!not-for-mail
Subject: Test Article";

    let headers = parse_headers(headers_text).unwrap();

    // Should decode to "André <andre@example.com>"
    assert_eq!(headers.from, "André <andre@example.com>");
}

#[test]
fn test_parse_headers_with_encoded_subject_quoted_printable() {
    // Test Quoted-Printable encoded Subject header
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc123@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com!not-for-mail
Subject: =?ISO-8859-1?Q?Caf=E9?=";

    let headers = parse_headers(headers_text).unwrap();

    // Should decode to "Café"
    assert_eq!(headers.subject, "Café");
}

#[test]
fn test_parse_headers_with_multiple_encoded_words() {
    // Test multiple consecutive encoded words in Subject
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc123@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com!not-for-mail
Subject: =?UTF-8?B?SGVsbG8=?= =?UTF-8?B?IFdvcmxk?=";

    let headers = parse_headers(headers_text).unwrap();

    // Should decode to "Hello World" (whitespace between encoded words removed)
    assert_eq!(headers.subject, "Hello World");
}

#[test]
fn test_parse_headers_with_mixed_encoded_and_plain() {
    // Test Subject with both encoded and plain text
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc123@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com!not-for-mail
Subject: Re: =?UTF-8?B?SGVsbG8=?= World";

    let headers = parse_headers(headers_text).unwrap();

    // Should decode to "Re: Hello World"
    assert_eq!(headers.subject, "Re: Hello World");
}

#[test]
fn test_parse_headers_with_encoded_organization() {
    // Test Organization header with encoding
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc123@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com!not-for-mail
Subject: Test
Organization: =?UTF-8?B?VW5pdmVyc2l0w6k=?= de Paris";

    let headers = parse_headers(headers_text).unwrap();

    // Should decode to "Université de Paris"
    assert_eq!(
        headers.organization.as_ref().unwrap(),
        "Université de Paris"
    );
}

#[test]
fn test_parse_headers_with_encoded_keywords() {
    // Test Keywords header with encoding
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc123@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com!not-for-mail
Subject: Test
Keywords: =?UTF-8?Q?pr=C3=A9sentation?=, documentation";

    let headers = parse_headers(headers_text).unwrap();

    // Should decode to "présentation, documentation"
    assert_eq!(
        headers.keywords.as_ref().unwrap(),
        "présentation, documentation"
    );
}

#[test]
fn test_parse_headers_with_invalid_encoded_word_passthrough() {
    // Test that invalid encoded words are passed through unchanged
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc123@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com!not-for-mail
Subject: =?INVALID Test";

    let headers = parse_headers(headers_text).unwrap();

    // Invalid encoded word should be passed through as-is
    assert_eq!(headers.subject, "=?INVALID Test");
}

#[test]
fn test_parse_headers_message_id_not_decoded() {
    // Message-ID should NOT be decoded (should be ASCII-only)
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc123@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com!not-for-mail
Subject: Test";

    let headers = parse_headers(headers_text).unwrap();

    // Message-ID should remain unchanged
    assert_eq!(headers.message_id, "<abc123@example.com>");
}

#[test]
fn test_parse_headers_references_not_decoded() {
    // References should NOT be decoded (should be ASCII-only)
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc123@example.com>
Newsgroups: comp.lang.rust
Path: news.example.com!not-for-mail
Subject: Test
References: <ref1@example.com> <ref2@example.com>";

    let headers = parse_headers(headers_text).unwrap();

    // References should remain unchanged
    let refs = headers.references.unwrap();
    assert_eq!(refs.len(), 2);
    assert_eq!(refs[0], "<ref1@example.com>");
    assert_eq!(refs[1], "<ref2@example.com>");
}
#[test]
fn test_parse_path_multiple_servers() {
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc123@example.com>
Newsgroups: comp.lang.rust
Path: news1.example.com!news2.example.net!feed.example.org!not-for-mail
Subject: Test";

    let headers = parse_headers(headers_text).unwrap();

    let path = headers.parse_path();
    assert_eq!(path.len(), 4);
    assert_eq!(path[0], "news1.example.com");
    assert_eq!(path[1], "news2.example.net");
    assert_eq!(path[2], "feed.example.org");
    assert_eq!(path[3], "not-for-mail");
}

#[test]
fn test_parse_path_single_server() {
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc123@example.com>
Newsgroups: comp.lang.rust
Path: localhost!not-for-mail
Subject: Test";

    let headers = parse_headers(headers_text).unwrap();

    let path = headers.parse_path();
    assert_eq!(path.len(), 2);
    assert_eq!(path[0], "localhost");
    assert_eq!(path[1], "not-for-mail");
}

#[test]
fn test_parse_path_with_whitespace() {
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc123@example.com>
Newsgroups: comp.lang.rust
Path: news1.example.com ! news2.example.net ! not-for-mail
Subject: Test";

    let headers = parse_headers(headers_text).unwrap();

    let path = headers.parse_path();
    assert_eq!(path.len(), 3);
    assert_eq!(path[0], "news1.example.com");
    assert_eq!(path[1], "news2.example.net");
    assert_eq!(path[2], "not-for-mail");
}

#[test]
fn test_originating_server() {
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc123@example.com>
Newsgroups: comp.lang.rust
Path: news1.example.com!news2.example.net!not-for-mail
Subject: Test";

    let headers = parse_headers(headers_text).unwrap();

    assert_eq!(headers.originating_server(), Some("news1.example.com"));
}

#[test]
fn test_originating_server_single() {
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc123@example.com>
Newsgroups: comp.lang.rust
Path: localhost
Subject: Test";

    let headers = parse_headers(headers_text).unwrap();

    assert_eq!(headers.originating_server(), Some("localhost"));
}

#[test]
fn test_path_length() {
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc123@example.com>
Newsgroups: comp.lang.rust
Path: news1.example.com!news2.example.net!feed.example.org!not-for-mail
Subject: Test";

    let headers = parse_headers(headers_text).unwrap();

    assert_eq!(headers.path_length(), 4);
}

#[test]
fn test_path_length_single() {
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc123@example.com>
Newsgroups: comp.lang.rust
Path: localhost
Subject: Test";

    let headers = parse_headers(headers_text).unwrap();

    assert_eq!(headers.path_length(), 1);
}

#[test]
fn test_parse_path_very_long() {
    // Test with 50+ servers (long path through many relays)
    let long_path = (1..=55)
        .map(|i| format!("server{}.example.com", i))
        .collect::<Vec<_>>()
        .join("!");

    let headers_text = format!(
        "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc123@example.com>
Newsgroups: comp.lang.rust
Path: {}
Subject: Test",
        long_path
    );

    let headers = parse_headers(&headers_text).unwrap();

    let path = headers.parse_path();
    assert_eq!(path.len(), 55);
    assert_eq!(path[0], "server1.example.com");
    assert_eq!(path[54], "server55.example.com");
    assert_eq!(headers.path_length(), 55);
}

#[test]
fn test_parse_path_with_special_chars() {
    // Some server names might contain dots, dashes, underscores
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc123@example.com>
Newsgroups: comp.lang.rust
Path: news-server-1.example.com!relay_2.example.net!feed.3.example.org!not-for-mail
Subject: Test";

    let headers = parse_headers(headers_text).unwrap();

    let path = headers.parse_path();
    assert_eq!(path.len(), 4);
    assert_eq!(path[0], "news-server-1.example.com");
    assert_eq!(path[1], "relay_2.example.net");
    assert_eq!(path[2], "feed.3.example.org");
    assert_eq!(path[3], "not-for-mail");
}

#[test]
fn test_parse_path_empty_components() {
    // Malformed path with empty components (consecutive !!)
    let headers_text = "\
Date: Mon, 20 Jan 2025 12:00:00 +0000
From: user@example.com
Message-ID: <abc123@example.com>
Newsgroups: comp.lang.rust
Path: news1.example.com!!news2.example.net!not-for-mail
Subject: Test";

    let headers = parse_headers(headers_text).unwrap();

    // Empty components should be filtered out
    let path = headers.parse_path();
    assert_eq!(path.len(), 3);
    assert_eq!(path[0], "news1.example.com");
    assert_eq!(path[1], "news2.example.net");
    assert_eq!(path[2], "not-for-mail");
}

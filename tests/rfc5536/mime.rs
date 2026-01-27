//! RFC 5536 Section 4: MIME Detection Tests
//!
//! Tests for detecting and parsing MIME-related headers in Usenet articles.

use nntp_rs::article::{Article, Headers};

/// Helper to create basic headers for testing
fn create_basic_headers() -> Headers {
    Headers::new(
        "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
        "user@example.com".to_string(),
        "<msg123@example.com>".to_string(),
        vec!["comp.test".to_string()],
        "news.example.com!not-for-mail".to_string(),
        "Test Article".to_string(),
    )
}

#[test]
fn test_is_mime_with_text_plain() {
    let mut headers = create_basic_headers();
    headers
        .extra
        .insert("Content-Type".to_string(), "text/plain".to_string());

    let article = Article::new(headers, "This is the body".to_string());
    assert!(article.is_mime());
}

#[test]
fn test_is_mime_with_multipart_mixed() {
    let mut headers = create_basic_headers();
    headers.extra.insert(
        "Content-Type".to_string(),
        "multipart/mixed; boundary=\"----boundary123\"".to_string(),
    );

    let article = Article::new(headers, "Multipart body".to_string());
    assert!(article.is_mime());
}

#[test]
fn test_is_mime_without_content_type() {
    let headers = create_basic_headers();
    let article = Article::new(headers, "Plain text body".to_string());
    assert!(!article.is_mime());
}

#[test]
fn test_is_mime_with_charset_utf8() {
    let mut headers = create_basic_headers();
    headers.extra.insert(
        "Content-Type".to_string(),
        "text/plain; charset=utf-8".to_string(),
    );

    let article = Article::new(headers, "UTF-8 body".to_string());
    assert!(article.is_mime());
}

#[test]
fn test_is_mime_with_charset_iso88591() {
    let mut headers = create_basic_headers();
    headers.extra.insert(
        "Content-Type".to_string(),
        "text/plain; charset=iso-8859-1".to_string(),
    );

    let article = Article::new(headers, "Latin-1 body".to_string());
    assert!(article.is_mime());
}

#[test]
fn test_content_type_text_plain() {
    let mut headers = create_basic_headers();
    headers
        .extra
        .insert("Content-Type".to_string(), "text/plain".to_string());

    let article = Article::new(headers, "Body".to_string());
    assert_eq!(article.content_type(), Some("text/plain"));
}

#[test]
fn test_content_type_with_charset_parameter() {
    let mut headers = create_basic_headers();
    headers.extra.insert(
        "Content-Type".to_string(),
        "text/plain; charset=utf-8".to_string(),
    );

    let article = Article::new(headers, "Body".to_string());
    assert_eq!(article.content_type(), Some("text/plain; charset=utf-8"));
}

#[test]
fn test_content_type_none_when_missing() {
    let headers = create_basic_headers();
    let article = Article::new(headers, "Body".to_string());
    assert_eq!(article.content_type(), None);
}

#[test]
fn test_content_type_multipart_with_boundary() {
    let mut headers = create_basic_headers();
    headers.extra.insert(
        "Content-Type".to_string(),
        "multipart/mixed; boundary=\"----boundary123\"".to_string(),
    );

    let article = Article::new(headers, "Body".to_string());
    assert_eq!(
        article.content_type(),
        Some("multipart/mixed; boundary=\"----boundary123\"")
    );
}

#[test]
fn test_charset_utf8() {
    let mut headers = create_basic_headers();
    headers.extra.insert(
        "Content-Type".to_string(),
        "text/plain; charset=utf-8".to_string(),
    );

    let article = Article::new(headers, "Body".to_string());
    assert_eq!(article.charset(), Some("utf-8"));
}

#[test]
fn test_charset_iso88591() {
    let mut headers = create_basic_headers();
    headers.extra.insert(
        "Content-Type".to_string(),
        "text/plain; charset=iso-8859-1".to_string(),
    );

    let article = Article::new(headers, "Body".to_string());
    assert_eq!(article.charset(), Some("iso-8859-1"));
}

#[test]
fn test_charset_with_quotes() {
    let mut headers = create_basic_headers();
    headers.extra.insert(
        "Content-Type".to_string(),
        "text/plain; charset=\"utf-8\"".to_string(),
    );

    let article = Article::new(headers, "Body".to_string());
    assert_eq!(article.charset(), Some("utf-8"));
}

#[test]
fn test_charset_with_single_quotes() {
    let mut headers = create_basic_headers();
    headers.extra.insert(
        "Content-Type".to_string(),
        "text/plain; charset='iso-8859-1'".to_string(),
    );

    let article = Article::new(headers, "Body".to_string());
    assert_eq!(article.charset(), Some("iso-8859-1"));
}

#[test]
fn test_charset_none_when_missing() {
    let mut headers = create_basic_headers();
    headers
        .extra
        .insert("Content-Type".to_string(), "text/plain".to_string());

    let article = Article::new(headers, "Body".to_string());
    assert_eq!(article.charset(), None);
}

#[test]
fn test_charset_none_when_no_content_type() {
    let headers = create_basic_headers();
    let article = Article::new(headers, "Body".to_string());
    assert_eq!(article.charset(), None);
}

#[test]
fn test_charset_uppercase() {
    let mut headers = create_basic_headers();
    headers.extra.insert(
        "Content-Type".to_string(),
        "text/plain; CHARSET=UTF-8".to_string(),
    );

    let article = Article::new(headers, "Body".to_string());
    assert_eq!(article.charset(), Some("UTF-8"));
}

#[test]
fn test_charset_with_extra_whitespace() {
    let mut headers = create_basic_headers();
    headers.extra.insert(
        "Content-Type".to_string(),
        "text/plain;  charset = utf-8 ".to_string(),
    );

    let article = Article::new(headers, "Body".to_string());
    assert_eq!(article.charset(), Some("utf-8"));
}

#[test]
fn test_is_multipart_true() {
    let mut headers = create_basic_headers();
    headers.extra.insert(
        "Content-Type".to_string(),
        "multipart/mixed; boundary=\"----boundary123\"".to_string(),
    );

    let article = Article::new(headers, "Body".to_string());
    assert!(article.is_multipart());
}

#[test]
fn test_is_multipart_alternative() {
    let mut headers = create_basic_headers();
    headers.extra.insert(
        "Content-Type".to_string(),
        "multipart/alternative; boundary=\"boundary456\"".to_string(),
    );

    let article = Article::new(headers, "Body".to_string());
    assert!(article.is_multipart());
}

#[test]
fn test_is_multipart_false_for_text_plain() {
    let mut headers = create_basic_headers();
    headers
        .extra
        .insert("Content-Type".to_string(), "text/plain".to_string());

    let article = Article::new(headers, "Body".to_string());
    assert!(!article.is_multipart());
}

#[test]
fn test_is_multipart_false_when_no_content_type() {
    let headers = create_basic_headers();
    let article = Article::new(headers, "Body".to_string());
    assert!(!article.is_multipart());
}

#[test]
fn test_is_multipart_case_insensitive() {
    let mut headers = create_basic_headers();
    headers.extra.insert(
        "Content-Type".to_string(),
        "MULTIPART/MIXED; boundary=\"test\"".to_string(),
    );

    let article = Article::new(headers, "Body".to_string());
    assert!(article.is_multipart());
}

#[test]
fn test_is_multipart_with_leading_whitespace() {
    let mut headers = create_basic_headers();
    headers.extra.insert(
        "Content-Type".to_string(),
        "  multipart/related; boundary=\"test\"".to_string(),
    );

    let article = Article::new(headers, "Body".to_string());
    assert!(article.is_multipart());
}

#[test]
fn test_yenc_binary_post_with_mime() {
    // Real-world yEnc binary post often includes MIME headers
    let mut headers = create_basic_headers();
    headers.subject = "[1/10] - \"file.bin\" yEnc (1/50)".to_string();
    headers.extra.insert(
        "Content-Type".to_string(),
        "text/plain; charset=ISO-8859-1".to_string(),
    );
    headers
        .extra
        .insert("Content-Transfer-Encoding".to_string(), "8bit".to_string());

    let body = "=ybegin line=128 size=1000000 name=file.bin\n\
                =ypart begin=1 end=20000\n\
                ...binary data...\n\
                =yend size=20000 part=1 crc32=12345678"
        .to_string();

    let article = Article::new(headers, body);

    assert!(article.is_mime());
    assert_eq!(
        article.content_type(),
        Some("text/plain; charset=ISO-8859-1")
    );
    assert_eq!(article.charset(), Some("ISO-8859-1"));
    assert!(!article.is_multipart());
}

#[test]
fn test_multipart_mime_article() {
    let mut headers = create_basic_headers();
    headers.subject = "Multipart Test".to_string();
    headers.extra.insert(
        "Content-Type".to_string(),
        "multipart/mixed; boundary=\"----=_NextPart_000_001\"".to_string(),
    );
    headers
        .extra
        .insert("MIME-Version".to_string(), "1.0".to_string());

    let body = "------=_NextPart_000_001\n\
                Content-Type: text/plain; charset=utf-8\n\
                \n\
                This is the text part.\n\
                \n\
                ------=_NextPart_000_001\n\
                Content-Type: application/octet-stream\n\
                Content-Transfer-Encoding: base64\n\
                \n\
                SGVsbG8gV29ybGQ=\n\
                ------=_NextPart_000_001--"
        .to_string();

    let article = Article::new(headers, body);

    assert!(article.is_mime());
    assert!(article.is_multipart());
    assert!(article.content_type().unwrap().contains("multipart/mixed"));
    assert_eq!(article.charset(), None); // multipart has no charset, only parts do
}

#[test]
fn test_plain_text_article_no_mime() {
    let headers = create_basic_headers();
    let body = "This is a plain text Usenet article.\n\
                No MIME headers, just plain 7-bit ASCII text.\n\
                \n\
                -- \n\
                Signature"
        .to_string();

    let article = Article::new(headers, body);

    assert!(!article.is_mime());
    assert_eq!(article.content_type(), None);
    assert_eq!(article.charset(), None);
    assert!(!article.is_multipart());
}

#[test]
fn test_html_content_type() {
    let mut headers = create_basic_headers();
    headers.extra.insert(
        "Content-Type".to_string(),
        "text/html; charset=utf-8".to_string(),
    );

    let body = "<html><body><h1>Hello</h1></body></html>".to_string();
    let article = Article::new(headers, body);

    assert!(article.is_mime());
    assert_eq!(article.content_type(), Some("text/html; charset=utf-8"));
    assert_eq!(article.charset(), Some("utf-8"));
    assert!(!article.is_multipart());
}

#[test]
fn test_application_octet_stream() {
    let mut headers = create_basic_headers();
    headers.extra.insert(
        "Content-Type".to_string(),
        "application/octet-stream".to_string(),
    );
    headers.extra.insert(
        "Content-Transfer-Encoding".to_string(),
        "base64".to_string(),
    );

    let body = "SGVsbG8gV29ybGQhIFRoaXMgaXMgYmluYXJ5IGRhdGEu".to_string();
    let article = Article::new(headers, body);

    assert!(article.is_mime());
    assert_eq!(article.content_type(), Some("application/octet-stream"));
    assert_eq!(article.charset(), None); // Binary data has no charset
    assert!(!article.is_multipart());
}

#[test]
fn test_multipart_alternative_html_and_text() {
    let mut headers = create_basic_headers();
    headers.extra.insert(
        "Content-Type".to_string(),
        "multipart/alternative; boundary=\"alt-boundary\"".to_string(),
    );

    let body = "--alt-boundary\n\
                Content-Type: text/plain; charset=utf-8\n\
                \n\
                Plain text version\n\
                \n\
                --alt-boundary\n\
                Content-Type: text/html; charset=utf-8\n\
                \n\
                <html><body>HTML version</body></html>\n\
                --alt-boundary--"
        .to_string();

    let article = Article::new(headers, body);

    assert!(article.is_mime());
    assert!(article.is_multipart());
    assert!(article
        .content_type()
        .unwrap()
        .contains("multipart/alternative"));
}

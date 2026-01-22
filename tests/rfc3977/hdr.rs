//! RFC 3977 Section 8.5 - HDR Command Tests
//!
//! Tests for the HDR command which retrieves specific header field values from articles.
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc3977#section-8.5

use nntp_rs::{codes, commands, NntpResponse};
#[test]
fn test_hdr_command_format_basic() {
    let cmd = commands::hdr("Subject", "1-100");
    assert_eq!(cmd, "HDR Subject 1-100\r\n");
}

#[test]
fn test_hdr_command_format_single_article() {
    let cmd = commands::hdr("From", "12345");
    assert_eq!(cmd, "HDR From 12345\r\n");
}

#[test]
fn test_hdr_command_format_message_id() {
    let cmd = commands::hdr("Subject", "<abc@example.com>");
    assert_eq!(cmd, "HDR Subject <abc@example.com>\r\n");
}

#[test]
fn test_hdr_current_command_format() {
    let cmd = commands::hdr_current("Date");
    assert_eq!(cmd, "HDR Date\r\n");
}

#[test]
fn test_hdr_command_ends_with_crlf() {
    let cmd = commands::hdr("Subject", "1-10");
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_hdr_command_uppercase() {
    let cmd = commands::hdr("subject", "100");
    assert!(cmd.starts_with("HDR"));
}

#[test]
fn test_hdr_various_header_fields() {
    // Standard headers
    assert!(commands::hdr("Subject", "1").contains("Subject"));
    assert!(commands::hdr("From", "1").contains("From"));
    assert!(commands::hdr("Date", "1").contains("Date"));
    assert!(commands::hdr("Message-ID", "1").contains("Message-ID"));
    assert!(commands::hdr("References", "1").contains("References"));
    assert!(commands::hdr("Lines", "1").contains("Lines"));
    assert!(commands::hdr("Xref", "1").contains("Xref"));

    // Custom headers
    assert!(commands::hdr("X-Newsreader", "1").contains("X-Newsreader"));
}

#[test]
fn test_hdr_range_variants() {
    // Range formats
    assert_eq!(
        commands::hdr("Subject", "100-200"),
        "HDR Subject 100-200\r\n"
    );
    assert_eq!(commands::hdr("Subject", "100-"), "HDR Subject 100-\r\n");
    assert_eq!(commands::hdr("Subject", "-200"), "HDR Subject -200\r\n");
}
#[test]
fn test_parse_hdr_line_basic() {
    let entry = commands::parse_hdr_line("12345 Test Subject").unwrap();
    assert_eq!(entry.article_number, 12345);
    assert_eq!(entry.value, "Test Subject");
}

#[test]
fn test_parse_hdr_line_with_spaces() {
    let entry = commands::parse_hdr_line("100 Re: This is a long subject").unwrap();
    assert_eq!(entry.article_number, 100);
    assert_eq!(entry.value, "Re: This is a long subject");
}

#[test]
fn test_parse_hdr_line_empty_value() {
    let entry = commands::parse_hdr_line("999 ").unwrap();
    assert_eq!(entry.article_number, 999);
    assert_eq!(entry.value, "");
}

#[test]
fn test_parse_hdr_line_zero_article_number() {
    // When queried by message-id, article number might be 0
    let entry = commands::parse_hdr_line("0 Test Subject").unwrap();
    assert_eq!(entry.article_number, 0);
    assert_eq!(entry.value, "Test Subject");
}

#[test]
fn test_parse_hdr_line_large_article_number() {
    let entry = commands::parse_hdr_line("18446744073709551615 Test").unwrap();
    assert_eq!(entry.article_number, u64::MAX);
    assert_eq!(entry.value, "Test");
}

#[test]
fn test_parse_hdr_line_special_characters() {
    let entry = commands::parse_hdr_line("123 [SPAM] Re: Test (was: Other)").unwrap();
    assert_eq!(entry.article_number, 123);
    assert_eq!(entry.value, "[SPAM] Re: Test (was: Other)");
}

#[test]
fn test_parse_hdr_line_unicode() {
    let entry = commands::parse_hdr_line("456 测试主题 Тест").unwrap();
    assert_eq!(entry.article_number, 456);
    assert_eq!(entry.value, "测试主题 Тест");
}

#[test]
fn test_parse_hdr_line_email_address() {
    let entry = commands::parse_hdr_line("789 user@example.com (John Doe)").unwrap();
    assert_eq!(entry.article_number, 789);
    assert_eq!(entry.value, "user@example.com (John Doe)");
}

#[test]
fn test_parse_hdr_line_date() {
    let entry = commands::parse_hdr_line("111 Mon, 01 Jan 2024 12:00:00 +0000").unwrap();
    assert_eq!(entry.article_number, 111);
    assert_eq!(entry.value, "Mon, 01 Jan 2024 12:00:00 +0000");
}

#[test]
fn test_parse_hdr_line_message_id() {
    let entry = commands::parse_hdr_line("222 <abc123@news.example.com>").unwrap();
    assert_eq!(entry.article_number, 222);
    assert_eq!(entry.value, "<abc123@news.example.com>");
}
#[test]
fn test_parse_hdr_line_invalid_no_space() {
    assert!(commands::parse_hdr_line("12345").is_err());
}

#[test]
fn test_parse_hdr_line_invalid_article_number() {
    assert!(commands::parse_hdr_line("abc Test").is_err());
}

#[test]
fn test_parse_hdr_line_empty() {
    assert!(commands::parse_hdr_line("").is_err());
}

#[test]
fn test_parse_hdr_line_only_spaces() {
    assert!(commands::parse_hdr_line("   ").is_err());
}

#[test]
fn test_parse_hdr_line_negative_number() {
    assert!(commands::parse_hdr_line("-100 Test").is_err());
}
#[test]
fn test_parse_hdr_response_success_single() {
    let response = NntpResponse {
        code: codes::HEADERS_FOLLOW,
        message: "Headers follow".to_string(),
        lines: vec!["12345 Test Subject".to_string()],
    };

    let entries = commands::parse_hdr_response(&response).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].article_number, 12345);
    assert_eq!(entries[0].value, "Test Subject");
}

#[test]
fn test_parse_hdr_response_success_multiple() {
    let response = NntpResponse {
        code: codes::HEADERS_FOLLOW,
        message: "Headers follow".to_string(),
        lines: vec![
            "100 First Subject".to_string(),
            "101 Second Subject".to_string(),
            "102 Third Subject".to_string(),
        ],
    };

    let entries = commands::parse_hdr_response(&response).unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].value, "First Subject");
    assert_eq!(entries[1].value, "Second Subject");
    assert_eq!(entries[2].value, "Third Subject");
}

#[test]
fn test_parse_hdr_response_empty() {
    let response = NntpResponse {
        code: codes::HEADERS_FOLLOW,
        message: "No headers available".to_string(),
        lines: vec![],
    };

    let entries = commands::parse_hdr_response(&response).unwrap();
    assert_eq!(entries.len(), 0);
}

#[test]
fn test_parse_hdr_response_skips_malformed() {
    let response = NntpResponse {
        code: codes::HEADERS_FOLLOW,
        message: "Headers follow".to_string(),
        lines: vec![
            "100 Valid Entry".to_string(),
            "invalid line".to_string(),
            "101 Another Valid".to_string(),
        ],
    };

    let entries = commands::parse_hdr_response(&response).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].article_number, 100);
    assert_eq!(entries[1].article_number, 101);
}

#[test]
fn test_parse_hdr_response_protocol_error() {
    let response = NntpResponse {
        code: 500,
        message: "Command not understood".to_string(),
        lines: vec![],
    };

    assert!(commands::parse_hdr_response(&response).is_err());
}

// RFC 3977 Section 8.5 Example Tests

#[test]
fn test_rfc_3977_example_subject_range() {
    // RFC 3977 Section 8.5 example
    let response = NntpResponse {
        code: 225,
        message: "Headers follow".to_string(),
        lines: vec![
            "3000234 I am just a test article".to_string(),
            "3000235 Another test article".to_string(),
            "3000236 Re: I am just a test article".to_string(),
        ],
    };

    let entries = commands::parse_hdr_response(&response).unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].article_number, 3000234);
    assert_eq!(entries[0].value, "I am just a test article");
    assert_eq!(entries[2].value, "Re: I am just a test article");
}

#[test]
fn test_rfc_3977_example_message_id_query() {
    // HDR queried by message-id might return article number 0
    let response = NntpResponse {
        code: 225,
        message: "Headers follow".to_string(),
        lines: vec!["0 test@example.com".to_string()],
    };

    let entries = commands::parse_hdr_response(&response).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].article_number, 0);
    assert_eq!(entries[0].value, "test@example.com");
}
#[test]
fn test_hdr_response_code_225() {
    let response = NntpResponse {
        code: codes::HEADERS_FOLLOW,
        message: "Headers follow".to_string(),
        lines: vec!["100 Test".to_string()],
    };

    assert!(response.is_success());
    assert_eq!(response.code, 225);
}

#[test]
fn test_hdr_error_code_412_no_group() {
    let response = NntpResponse {
        code: codes::NO_GROUP_SELECTED,
        message: "No newsgroup selected".to_string(),
        lines: vec![],
    };

    assert!(response.is_error());
    assert_eq!(response.code, 412);
}

#[test]
fn test_hdr_error_code_420_no_current() {
    let response = NntpResponse {
        code: codes::NO_CURRENT_ARTICLE,
        message: "No current article selected".to_string(),
        lines: vec![],
    };

    assert!(response.is_error());
    assert_eq!(response.code, 420);
}

#[test]
fn test_hdr_error_code_423_no_article() {
    let response = NntpResponse {
        code: codes::NO_SUCH_ARTICLE_NUMBER,
        message: "No article with that number".to_string(),
        lines: vec![],
    };

    assert!(response.is_error());
    assert_eq!(response.code, 423);
}

#[test]
fn test_hdr_error_code_430_no_message_id() {
    let response = NntpResponse {
        code: codes::NO_SUCH_ARTICLE_ID,
        message: "No article with that message-id".to_string(),
        lines: vec![],
    };

    assert!(response.is_error());
    assert_eq!(response.code, 430);
}

#[test]
fn test_hdr_error_code_502_not_supported() {
    let response = NntpResponse {
        code: 502,
        message: "Command not supported".to_string(),
        lines: vec![],
    };

    assert!(response.is_error());
    assert_eq!(response.code, 502);
}


#[test]
fn test_hdr_very_large_response() {
    let mut lines = Vec::new();
    for i in 1..=10000 {
        lines.push(format!("{} Subject {}", i, i));
    }

    let response = NntpResponse {
        code: 225,
        message: "Headers follow".to_string(),
        lines,
    };

    let entries = commands::parse_hdr_response(&response).unwrap();
    assert_eq!(entries.len(), 10000);
    assert_eq!(entries[0].article_number, 1);
    assert_eq!(entries[9999].article_number, 10000);
}

#[test]
fn test_hdr_sparse_article_numbers() {
    let response = NntpResponse {
        code: 225,
        message: "Headers follow".to_string(),
        lines: vec![
            "1 First".to_string(),
            "1000 Second".to_string(),
            "500000 Third".to_string(),
        ],
    };

    let entries = commands::parse_hdr_response(&response).unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].article_number, 1);
    assert_eq!(entries[1].article_number, 1000);
    assert_eq!(entries[2].article_number, 500000);
}

#[test]
fn test_hdr_empty_header_values() {
    let response = NntpResponse {
        code: 225,
        message: "Headers follow".to_string(),
        lines: vec![
            "100 ".to_string(), // Empty value
            "101 Normal Value".to_string(),
            "102 ".to_string(), // Another empty
        ],
    };

    let entries = commands::parse_hdr_response(&response).unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].value, "");
    assert_eq!(entries[1].value, "Normal Value");
    assert_eq!(entries[2].value, "");
}

#[test]
fn test_hdr_all_malformed_lines() {
    let response = NntpResponse {
        code: 225,
        message: "Headers follow".to_string(),
        lines: vec!["malformed".to_string(), "also bad".to_string()],
    };

    let entries = commands::parse_hdr_response(&response).unwrap();
    assert_eq!(entries.len(), 0);
}

// Real-World Scenarios

#[test]
fn test_hdr_binary_newsgroup_subjects() {
    // Binary newsgroups often have yEnc-style subjects
    let response = NntpResponse {
        code: 225,
        message: "Headers follow".to_string(),
        lines: vec![
            "1500000 [1/10] - \"file.rar\" yEnc (1/100)".to_string(),
            "1500001 [2/10] - \"file.rar\" yEnc (2/100)".to_string(),
            "1500002 [3/10] - \"file.rar\" yEnc (3/100)".to_string(),
        ],
    };

    let entries = commands::parse_hdr_response(&response).unwrap();
    assert_eq!(entries.len(), 3);
    assert!(entries[0].value.contains("yEnc"));
    assert!(entries[1].value.contains("[2/10]"));
}

#[test]
fn test_hdr_from_field_various_formats() {
    let response = NntpResponse {
        code: 225,
        message: "Headers follow".to_string(),
        lines: vec![
            "100 user@example.com".to_string(),
            "101 user@example.com (John Doe)".to_string(),
            "102 \"John Doe\" <user@example.com>".to_string(),
            "103 John Doe <user@example.com>".to_string(),
        ],
    };

    let entries = commands::parse_hdr_response(&response).unwrap();
    assert_eq!(entries.len(), 4);
    assert_eq!(entries[0].value, "user@example.com");
    assert!(entries[1].value.contains("(John Doe)"));
    assert!(entries[2].value.contains("\"John Doe\""));
}

#[test]
fn test_hdr_references_field() {
    let response = NntpResponse {
        code: 225,
        message: "Headers follow".to_string(),
        lines: vec![
            "100 <msg1@host> <msg2@host> <msg3@host>".to_string(),
            "101 ".to_string(), // No references (new thread)
        ],
    };

    let entries = commands::parse_hdr_response(&response).unwrap();
    assert_eq!(entries.len(), 2);
    assert!(entries[0].value.contains("<msg1@host>"));
    assert_eq!(entries[1].value, "");
}

#[test]
fn test_hdr_lines_and_bytes_metadata() {
    let response = NntpResponse {
        code: 225,
        message: "Headers follow".to_string(),
        lines: vec![
            "100 42".to_string(),
            "101 100".to_string(),
            "102 999".to_string(),
        ],
    };

    let entries = commands::parse_hdr_response(&response).unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].value, "42");
    assert_eq!(entries[1].value, "100");
    assert_eq!(entries[2].value, "999");
}

#[test]
fn test_hdr_xref_field() {
    let response = NntpResponse {
        code: 225,
        message: "Headers follow".to_string(),
        lines: vec!["100 news.example.com alt.test:100 comp.lang.rust:200".to_string()],
    };

    let entries = commands::parse_hdr_response(&response).unwrap();
    assert_eq!(entries.len(), 1);
    assert!(entries[0].value.contains("news.example.com"));
    assert!(entries[0].value.contains("alt.test:100"));
}

#[test]
fn test_hdr_case_sensitive_header_names() {
    // Header names should be case-insensitive, but we preserve what's sent
    assert_eq!(commands::hdr("subject", "1"), "HDR subject 1\r\n");
    assert_eq!(commands::hdr("Subject", "1"), "HDR Subject 1\r\n");
    assert_eq!(commands::hdr("SUBJECT", "1"), "HDR SUBJECT 1\r\n");
}

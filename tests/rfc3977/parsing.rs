//! Tests for NNTP command parsing and response handling

use nntp_rs::NntpResponse;

#[test]
fn test_response_is_success() {
    let response = NntpResponse {
        code: 200,
        message: "Ready".to_string(),
        lines: vec![],
    };
    assert!(response.is_success());
    assert!(!response.is_continuation());
    assert!(!response.is_error());
}

#[test]
fn test_response_is_continuation() {
    let response = NntpResponse {
        code: 381,
        message: "Continue with auth".to_string(),
        lines: vec![],
    };
    assert!(!response.is_success());
    assert!(response.is_continuation());
    assert!(!response.is_error());
}

#[test]
fn test_response_is_error_4xx() {
    let response = NntpResponse {
        code: 411,
        message: "No such group".to_string(),
        lines: vec![],
    };
    assert!(!response.is_success());
    assert!(!response.is_continuation());
    assert!(response.is_error());
}

#[test]
fn test_response_is_error_5xx() {
    let response = NntpResponse {
        code: 502,
        message: "Access denied".to_string(),
        lines: vec![],
    };
    assert!(!response.is_success());
    assert!(!response.is_continuation());
    assert!(response.is_error());
}

#[test]
fn test_response_boundary_codes() {
    // Test exact boundaries
    assert!(!NntpResponse {
        code: 199,
        message: String::new(),
        lines: vec![]
    }
    .is_success());
    assert!(NntpResponse {
        code: 200,
        message: String::new(),
        lines: vec![]
    }
    .is_success());
    assert!(NntpResponse {
        code: 299,
        message: String::new(),
        lines: vec![]
    }
    .is_success());
    assert!(!NntpResponse {
        code: 300,
        message: String::new(),
        lines: vec![]
    }
    .is_success());

    assert!(!NntpResponse {
        code: 299,
        message: String::new(),
        lines: vec![]
    }
    .is_continuation());
    assert!(NntpResponse {
        code: 300,
        message: String::new(),
        lines: vec![]
    }
    .is_continuation());
    assert!(NntpResponse {
        code: 399,
        message: String::new(),
        lines: vec![]
    }
    .is_continuation());
    assert!(!NntpResponse {
        code: 400,
        message: String::new(),
        lines: vec![]
    }
    .is_continuation());

    assert!(!NntpResponse {
        code: 399,
        message: String::new(),
        lines: vec![]
    }
    .is_error());
    assert!(NntpResponse {
        code: 400,
        message: String::new(),
        lines: vec![]
    }
    .is_error());
    assert!(NntpResponse {
        code: 500,
        message: String::new(),
        lines: vec![]
    }
    .is_error());
    assert!(NntpResponse {
        code: 999,
        message: String::new(),
        lines: vec![]
    }
    .is_error());
}

#[test]
fn test_response_1xx_informational() {
    // 1xx codes should not match any category
    let response = NntpResponse {
        code: 100,
        message: "Help follows".to_string(),
        lines: vec![],
    };
    assert!(!response.is_success());
    assert!(!response.is_continuation());
    assert!(!response.is_error());
}

#[test]
fn test_response_with_multiline() {
    let response = NntpResponse {
        code: 220,
        message: "Article follows".to_string(),
        lines: vec![
            "From: user@example.com".to_string(),
            "Subject: Test".to_string(),
            "".to_string(),
            "Body content".to_string(),
        ],
    };
    assert!(response.is_success());
    assert_eq!(response.lines.len(), 4);
    assert_eq!(response.lines[0], "From: user@example.com");
}

#[test]
fn test_response_codes_constants() {
    use nntp_rs::codes;

    // Verify important code constants
    assert_eq!(codes::READY_POSTING_ALLOWED, 200);
    assert_eq!(codes::READY_NO_POSTING, 201);
    assert_eq!(codes::COMPRESSION_ACTIVE, 206);
    assert_eq!(codes::GROUP_SELECTED, 211);
    assert_eq!(codes::ARTICLE_FOLLOWS, 220);
    assert_eq!(codes::HEAD_FOLLOWS, 221);
    assert_eq!(codes::BODY_FOLLOWS, 222);
    assert_eq!(codes::OVERVIEW_INFO_FOLLOWS, 224);
    assert_eq!(codes::AUTH_ACCEPTED, 281);
    assert_eq!(codes::AUTH_CONTINUE, 381);
    assert_eq!(codes::NO_SUCH_GROUP, 411);
    assert_eq!(codes::NO_SUCH_ARTICLE_NUMBER, 423);
    assert_eq!(codes::NO_SUCH_ARTICLE_ID, 430);
    assert_eq!(codes::AUTH_REJECTED, 481);
    assert_eq!(codes::COMMAND_NOT_RECOGNIZED, 500);
    assert_eq!(codes::ACCESS_DENIED, 502);
}

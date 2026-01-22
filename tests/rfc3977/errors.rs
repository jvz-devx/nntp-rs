//! RFC 3977 Section 3.2.1 - NNTP Error Response Tests
//!
//! These tests verify correct handling of all NNTP error codes:
//! - 4xx: Temporary errors
//! - 5xx: Permanent errors

use nntp_rs::{codes, NntpError, NntpResponse};

// 4xx Temporary Error Codes

#[test]
fn test_error_400_service_unavailable() {
    let response = NntpResponse {
        code: codes::SERVICE_UNAVAILABLE,
        message: "Service temporarily unavailable".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 400);
    assert!(response.is_error());
    assert!(!response.is_success());
}

#[test]
fn test_error_411_no_such_group() {
    let response = NntpResponse {
        code: codes::NO_SUCH_GROUP,
        message: "No such newsgroup".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 411);
    assert!(response.is_error());
}

#[test]
fn test_error_412_no_group_selected() {
    let response = NntpResponse {
        code: codes::NO_GROUP_SELECTED,
        message: "No newsgroup selected".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 412);
    assert!(response.is_error());
}

#[test]
fn test_error_420_no_current_article() {
    let response = NntpResponse {
        code: codes::NO_CURRENT_ARTICLE,
        message: "No current article selected".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 420);
    assert!(response.is_error());
}

#[test]
fn test_error_421_no_next_article() {
    let response = NntpResponse {
        code: codes::NO_NEXT_ARTICLE,
        message: "No next article".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 421);
    assert!(response.is_error());
}

#[test]
fn test_error_422_no_previous_article() {
    let response = NntpResponse {
        code: codes::NO_PREV_ARTICLE,
        message: "No previous article".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 422);
    assert!(response.is_error());
}

#[test]
fn test_error_423_no_such_article_number() {
    let response = NntpResponse {
        code: codes::NO_SUCH_ARTICLE_NUMBER,
        message: "No article with that number".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 423);
    assert!(response.is_error());
}

#[test]
fn test_error_430_no_such_article_id() {
    let response = NntpResponse {
        code: codes::NO_SUCH_ARTICLE_ID,
        message: "No article with that message-id".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 430);
    assert!(response.is_error());
}

#[test]
fn test_error_481_auth_rejected() {
    let response = NntpResponse {
        code: codes::AUTH_REJECTED,
        message: "Authentication rejected".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 481);
    assert!(response.is_error());
}

#[test]
fn test_error_482_auth_out_of_sequence() {
    let response = NntpResponse {
        code: codes::AUTH_OUT_OF_SEQUENCE,
        message: "Authentication out of sequence".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 482);
    assert!(response.is_error());
}

// 5xx Permanent Error Codes

#[test]
fn test_error_500_command_not_recognized() {
    let response = NntpResponse {
        code: codes::COMMAND_NOT_RECOGNIZED,
        message: "Command not recognized".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 500);
    assert!(response.is_error());
}

#[test]
fn test_error_501_syntax_error() {
    let response = NntpResponse {
        code: codes::COMMAND_SYNTAX_ERROR,
        message: "Syntax error in command".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 501);
    assert!(response.is_error());
}

#[test]
fn test_error_502_access_denied() {
    let response = NntpResponse {
        code: codes::ACCESS_DENIED,
        message: "Access denied".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 502);
    assert!(response.is_error());
}

#[test]
fn test_error_503_feature_not_supported() {
    // RFC 3977: 503 = "Feature not supported" (optional functionality absent)
    let response = NntpResponse {
        code: codes::FEATURE_NOT_SUPPORTED,
        message: "Feature not supported".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 503);
    assert!(response.is_error());
}

#[test]
fn test_error_403_internal_fault() {
    // RFC 3977: 403 = "Internal fault" (server resource problem)
    // RFC 8054: Also used for "unable to activate compression"
    let response = NntpResponse {
        code: codes::INTERNAL_FAULT,
        message: "Internal server fault".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 403);
    assert!(response.is_error());
}

#[test]
fn test_error_483_encryption_required() {
    // RFC 4643: 483 = "Encryption or authentication required"
    let response = NntpResponse {
        code: codes::ENCRYPTION_REQUIRED,
        message: "Encryption required".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 483);
    assert!(response.is_error());
}

// Error Code Constants Verification

#[test]
fn test_all_error_code_constants() {
    // 4xx codes
    assert_eq!(codes::SERVICE_UNAVAILABLE, 400);
    assert_eq!(codes::INTERNAL_FAULT, 403);
    assert_eq!(codes::NO_SUCH_GROUP, 411);
    assert_eq!(codes::NO_GROUP_SELECTED, 412);
    assert_eq!(codes::NO_CURRENT_ARTICLE, 420);
    assert_eq!(codes::NO_NEXT_ARTICLE, 421);
    assert_eq!(codes::NO_PREV_ARTICLE, 422);
    assert_eq!(codes::NO_SUCH_ARTICLE_NUMBER, 423);
    assert_eq!(codes::NO_SUCH_ARTICLE_ID, 430);
    assert_eq!(codes::AUTH_REJECTED, 481);
    assert_eq!(codes::AUTH_OUT_OF_SEQUENCE, 482);
    assert_eq!(codes::ENCRYPTION_REQUIRED, 483);

    // 5xx codes
    assert_eq!(codes::COMMAND_NOT_RECOGNIZED, 500);
    assert_eq!(codes::COMMAND_SYNTAX_ERROR, 501);
    assert_eq!(codes::ACCESS_DENIED, 502);
    assert_eq!(codes::FEATURE_NOT_SUPPORTED, 503);
}

// NntpError Type Integration

#[test]
fn test_nntp_error_no_such_group() {
    let err = NntpError::NoSuchGroup("alt.nonexistent".to_string());
    let msg = format!("{}", err);

    assert!(msg.contains("No such newsgroup"));
    assert!(msg.contains("alt.nonexistent"));
}

#[test]
fn test_nntp_error_no_such_article() {
    let err = NntpError::NoSuchArticle("<missing@example>".to_string());
    let msg = format!("{}", err);

    assert!(msg.contains("No such article"));
    assert!(msg.contains("<missing@example>"));
}

#[test]
fn test_nntp_error_protocol() {
    let err = NntpError::Protocol {
        code: 411,
        message: "No such group".to_string(),
    };
    let msg = format!("{}", err);

    assert!(msg.contains("411"));
    assert!(msg.contains("No such group"));
}

#[test]
fn test_nntp_error_timeout() {
    let err = NntpError::Timeout;
    let msg = format!("{}", err);

    assert!(msg.contains("timeout") || msg.contains("Timeout"));
}

#[test]
fn test_nntp_error_connection_closed() {
    let err = NntpError::ConnectionClosed;
    let msg = format!("{}", err);

    assert!(msg.contains("closed") || msg.contains("Connection"));
}

#[test]
fn test_nntp_error_invalid_response() {
    let err = NntpError::InvalidResponse("garbage data".to_string());
    let msg = format!("{}", err);

    assert!(msg.contains("Invalid response"));
}

#[test]
fn test_nntp_error_auth_failed() {
    let err = NntpError::AuthFailed("bad password".to_string());
    let msg = format!("{}", err);

    assert!(msg.contains("Authentication failed"));
}

// Error Code Classification

#[test]
fn test_4xx_codes_are_errors() {
    let codes_4xx = [400, 403, 411, 412, 420, 421, 422, 423, 430, 481, 482, 483];

    for code in codes_4xx {
        let response = NntpResponse {
            code,
            message: "Error".to_string(),
            lines: vec![],
        };

        assert!(response.is_error(), "Code {} should be an error", code);
        assert!(
            !response.is_success(),
            "Code {} should not be success",
            code
        );
        assert!(
            !response.is_continuation(),
            "Code {} should not be continuation",
            code
        );
    }
}

#[test]
fn test_5xx_codes_are_errors() {
    let codes_5xx = [500, 501, 502, 503];

    for code in codes_5xx {
        let response = NntpResponse {
            code,
            message: "Error".to_string(),
            lines: vec![],
        };

        assert!(response.is_error(), "Code {} should be an error", code);
    }
}

#[test]
fn test_boundary_between_continuation_and_error() {
    // 399 is continuation (3xx)
    let response_399 = NntpResponse {
        code: 399,
        message: "".to_string(),
        lines: vec![],
    };
    assert!(response_399.is_continuation());
    assert!(!response_399.is_error());

    // 400 is error (4xx)
    let response_400 = NntpResponse {
        code: 400,
        message: "".to_string(),
        lines: vec![],
    };
    assert!(!response_400.is_continuation());
    assert!(response_400.is_error());
}

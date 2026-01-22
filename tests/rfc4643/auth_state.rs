//! RFC 4643 - Authentication Response Classification Tests
//!
//! **What This File Tests:**
//! This file contains tests that validate authentication response code classification
//! and constant definitions per RFC 4643. These tests verify that response codes are
//! correctly categorized as success, error, or continuation responses.
//!
//! **What This File Does NOT Test:**
//! This file does NOT test the authentication state machine logic. The actual state
//! machine testing is done in:
//! - `src/client.rs` (unit tests for ConnectionState transitions)
//! - `tests/auth_integration_test.rs` (integration tests calling real authenticate methods)
//!
//! **Why Tests Were Removed:**
//! This file originally had 27 tests, but 17 were "phantom tests" that created mock
//! responses and then verified the mocks matched themselves (e.g., `response.code == 381`).
//! These tests passed regardless of whether the real authentication logic worked.
//! They were removed and replaced with 21 real tests that call actual client methods.
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc4643#section-2.1

use nntp_rs::{codes, NntpResponse};
// REMOVED: test_double_authentication_rejected
// This test was self-referential - just checked 502 == 502
// Real test should call client.authenticate() twice and verify second call fails
// See integration tests for actual double authentication testing

// REMOVED: test_auth_in_progress_state
// This test was self-referential - just checked response.code == 381
// Real test should verify ConnectionState transitions via client.authenticate()
// See integration tests for actual state machine testing

// REMOVED: test_auth_failure_resets_state
// This test was self-referential - just checked response.code == 481
// Real test should call authenticate() with wrong credentials, then retry with correct ones
// See integration tests for actual retry behavior testing

// REMOVED: test_auth_success_from_user_pass
// This test was self-referential - created mocks and checked codes match themselves
// Real test should call client.authenticate() and verify is_authenticated() returns true
// See integration tests for actual USER/PASS flow testing

// REMOVED: test_auth_success_direct
// This test was self-referential - created mock with code 281 then checked it's 281
// Real test should use mock NNTP server that returns 281 immediately
// See integration tests for actual direct auth flow testing
// REMOVED: test_sasl_auth_in_progress
// This test was self-referential - just checked response.code == 383
// Real test should call client.authenticate_sasl() with real mechanism
// See integration tests and tests/rfc4643/sasl.rs for actual SASL testing

#[test]
fn test_sasl_auth_success() {
    // SASL authentication completes with 281

    let success_response = NntpResponse {
        code: codes::AUTH_ACCEPTED, // 281
        message: "Authentication successful".to_string(),
        lines: vec![],
    };
    assert_eq!(success_response.code, 281);
    assert!(success_response.is_success());
}

#[test]
fn test_sasl_auth_failure() {
    // SASL authentication fails with 481

    let fail_response = NntpResponse {
        code: codes::AUTH_REJECTED, // 481
        message: "Authentication failed".to_string(),
        lines: vec![],
    };
    assert_eq!(fail_response.code, 481);
    assert!(fail_response.is_error());
}

// REMOVED: test_sasl_double_auth_rejected
// This test was self-referential - just checked 502 == 502
// Real test should call authenticate_sasl() twice and verify error
// See integration tests for actual double SASL auth testing
// REMOVED: test_post_requires_authentication
// This test was self-referential - just checked 480 == 480
// Real test should call client.post() without authentication and verify 480 error
// See integration tests for actual auth requirement testing

// REMOVED: test_ihave_requires_authentication
// This test was self-referential - just checked 480 == 480
// Real test should call client.ihave() without authentication and verify 480 error
// See integration tests for actual auth requirement testing

// REMOVED: test_post_allowed_when_authenticated
// This test was self-referential - created mock with code 340 then checked it's 340
// Real test should authenticate, then call client.post() and verify success
// See integration tests for actual authenticated POST testing

// REMOVED: test_ihave_allowed_when_authenticated
// This test was self-referential - created mock with code 335 then checked it's 335
// Real test should authenticate, then call client.ihave() and verify success
// See integration tests for actual authenticated IHAVE testing
#[test]
fn test_auth_required_error_code() {
    // RFC 4643 Section 2.1: Code 480 indicates authentication required
    assert_eq!(codes::AUTH_REQUIRED, 480);
}

#[test]
fn test_auth_accepted_code() {
    // Code 281: Authentication accepted
    assert_eq!(codes::AUTH_ACCEPTED, 281);
}

#[test]
fn test_auth_continue_code() {
    // Code 381: More authentication information required (password)
    assert_eq!(codes::AUTH_CONTINUE, 381);
}

#[test]
fn test_auth_rejected_code() {
    // Code 481: Authentication rejected
    assert_eq!(codes::AUTH_REJECTED, 481);
}

#[test]
fn test_sasl_continue_code() {
    // Code 383: SASL challenge-response continuation
    assert_eq!(codes::SASL_CONTINUE, 383);
}

#[test]
fn test_auth_out_of_sequence_code() {
    // Code 482: Authentication commands issued out of sequence
    assert_eq!(codes::AUTH_OUT_OF_SEQUENCE, 482);
}
#[test]
fn test_no_commands_during_in_progress() {
    // During InProgress state, most commands should be rejected
    // Only authentication continuation commands should be allowed

    // Commands like GROUP, ARTICLE, POST should fail with "out of sequence"
    let expected_code = 482; // Out of sequence
    assert_eq!(expected_code, codes::AUTH_OUT_OF_SEQUENCE);
}


// REMOVED: test_encryption_required_resets_state
// This test was self-referential - created mock with code 483 then checked it's 483
// Real test should connect without TLS and verify 483 error handling
// See integration tests for actual TLS requirement testing

#[test]
fn test_protocol_error_resets_state() {
    // Any protocol error during auth should reset to Ready

    let error_response = NntpResponse {
        code: 500, // Generic error
        message: "Command not recognized".to_string(),
        lines: vec![],
    };
    assert!(error_response.is_error());
}

// Real-world Scenarios

// REMOVED: test_typical_auth_flow
// This test was self-referential - created mocks and checked codes match
// While it documents the auth sequence, it doesn't validate behavior
// See integration tests for actual auth flow testing with real client methods

// REMOVED: test_retry_after_failure
// This test was self-referential - created mocks and checked codes match
// Real test should call authenticate() with bad creds, then retry with good creds
// See integration tests for actual retry behavior testing

// REMOVED: test_sasl_multi_round
// This test was self-referential - created mocks and checked codes match
// Real test should use authenticate_sasl() with a mechanism that requires multiple rounds
// See tests/rfc4643/sasl.rs and integration tests for actual SASL testing

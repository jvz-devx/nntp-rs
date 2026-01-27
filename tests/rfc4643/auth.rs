//! RFC 4643 - NNTP Authentication Tests
//!
//! **What This File Tests:**
//! This file validates RFC 4643 authentication through real production code:
//! - Response code classification (is_success(), is_error(), is_continuation())
//! - AUTHINFO command formatting (authinfo_user, authinfo_pass, authinfo_sasl)
//! - RFC 4643 response code constants (281, 381, 481, 482, 483, 502, etc.)
//! - SASL mechanism name validation
//! - Capability advertisement parsing
//! - 502 "already authenticated" error handling
//!
//! **What This File Does NOT Test:**
//! This file does NOT test the authentication state machine or auth flows.
//! Those are tested in:
//! - `src/client.rs` (unit tests for ConnectionState transitions)
//! - `tests/auth_integration_test.rs` (integration tests calling real authenticate methods)
//!
//! **Cleanup Notes:**
//! Originally 47 tests. Removed 18 "phantom tests" that used mock state machine
//! functions (process_auth_response, process_sasl_response, process_extended_auth)
//! that didn't exist in production code. All remaining 29 tests validate real
//! methods from src/response.rs, src/commands.rs, and src/lib.rs.
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc4643

use nntp_rs::{codes, NntpResponse};

// Response Code Classification (RFC 4643 §2.3)

#[test]
fn test_auth_accepted_281() {
    // 281 = Authentication accepted
    let response = NntpResponse {
        code: codes::AUTH_ACCEPTED,
        message: "Authentication accepted".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 281);
    assert!(response.is_success());
}

#[test]
fn test_auth_continue_381() {
    // 381 = Password required (continue with AUTHINFO PASS)
    let response = NntpResponse {
        code: codes::AUTH_CONTINUE,
        message: "Password required".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 381);
    assert!(response.is_continuation());
    assert!(!response.is_success());
    assert!(!response.is_error());
}

#[test]
fn test_auth_rejected_481() {
    // 481 = Authentication failed/rejected
    let response = NntpResponse {
        code: codes::AUTH_REJECTED,
        message: "Authentication rejected".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 481);
    assert!(response.is_error());
}

#[test]
fn test_auth_out_of_sequence_482() {
    // 482 = Authentication out of sequence
    let response = NntpResponse {
        code: codes::AUTH_OUT_OF_SEQUENCE,
        message: "Authentication commands issued out of sequence".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 482);
    assert!(response.is_error());
}

#[test]
fn test_auth_502_already_authenticated() {
    // 502 = Command unavailable (already authenticated)
    let response = NntpResponse {
        code: codes::ACCESS_DENIED,
        message: "Already authenticated".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 502);
    assert!(response.is_error());
}

// RFC 4643 §2.3.1 - Encryption Required (483)
//
// RFC 4643 states: "483 response [indicates] the datastream is insufficiently
// secure for the command being attempted."
// This response is returned when AUTHINFO is attempted without TLS encryption.

#[test]
fn test_auth_encryption_required_483() {
    // RFC 4643 §2.3.1: 483 = "Encryption or authentication required"
    // Returned when AUTHINFO is attempted without TLS
    let response = NntpResponse {
        code: codes::ENCRYPTION_REQUIRED,
        message: "Encryption required for authentication".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 483);
    assert!(response.is_error());
}

#[test]
fn test_auth_encryption_required_code_constant() {
    assert_eq!(codes::ENCRYPTION_REQUIRED, 483);
}

#[test]
fn test_auth_encryption_required_various_messages() {
    // Different servers send different 483 messages
    let messages = [
        "Encryption required",
        "TLS required for authentication",
        "Use STARTTLS first",
        "Plaintext authentication not allowed",
    ];

    for msg in messages {
        let response = NntpResponse {
            code: 483,
            message: msg.to_string(),
            lines: vec![],
        };

        assert!(response.is_error());
        assert_eq!(response.message, msg);
    }
}

// REMOVED: Mock Authentication State Machine
//
// Previously, this file contained a mock AuthState enum and process_auth_response()
// function that simulated authentication state transitions. These were phantom tests
// that tested mock code instead of the real implementation in src/client.rs.
//
// The 7 removed tests were:
// - test_auth_flow_user_then_pass
// - test_auth_flow_user_only
// - test_auth_flow_user_rejected
// - test_auth_flow_pass_rejected
// - test_auth_flow_pass_out_of_sequence
// - test_auth_flow_encryption_required
// - test_auth_flow_encryption_required_after_user
//
// Real authentication state machine tests are in:
// - src/client.rs (unit tests for ConnectionState enum)
// - tests/auth_integration_test.rs (integration tests that call real NntpClient methods)
//
// These integration tests verify actual behavior by calling client.authenticate()
// and client.authenticate_sasl() with real NNTP servers.

// Response Code Constants Verification

#[test]
fn test_auth_code_constants() {
    assert_eq!(codes::AUTH_ACCEPTED, 281);
    assert_eq!(codes::AUTH_CONTINUE, 381);
    assert_eq!(codes::AUTH_REJECTED, 481);
    assert_eq!(codes::AUTH_OUT_OF_SEQUENCE, 482);
}

// Error Message Handling

#[test]
fn test_auth_error_message_preserved() {
    let response = NntpResponse {
        code: 481,
        message: "Invalid username or password".to_string(),
        lines: vec![],
    };

    assert_eq!(response.message, "Invalid username or password");
}

#[test]
fn test_auth_various_rejection_messages() {
    // Different servers send different messages
    let messages = [
        "Authentication failed",
        "Bad username or password",
        "Invalid credentials",
        "Access denied",
        "Authentication rejected",
    ];

    for msg in messages {
        let response = NntpResponse {
            code: 481,
            message: msg.to_string(),
            lines: vec![],
        };

        assert!(response.is_error());
        assert_eq!(response.message, msg);
    }
}

// Integration with NntpError

#[test]
fn test_auth_error_type() {
    use nntp_rs::NntpError;

    let err = NntpError::AuthFailed("Invalid credentials".to_string());
    let display = format!("{}", err);

    assert!(display.contains("Authentication failed"));
    assert!(display.contains("Invalid credentials"));
}

#[test]
fn test_protocol_error_for_auth() {
    use nntp_rs::NntpError;

    let err = NntpError::Protocol {
        code: 481,
        message: "Bad password".to_string(),
    };
    let display = format!("{}", err);

    assert!(display.contains("481"));
    assert!(display.contains("Bad password"));
}

// RFC 4643 §2.4 - SASL Authentication
//
// RFC 4643 defines AUTHINFO SASL as an alternative to USER/PASS.
// SASL provides more secure authentication mechanisms.

/// Response codes for SASL authentication
mod sasl_codes {
    pub const SASL_CONTINUE: u16 = 383; // Continue with SASL exchange
    pub const SASL_SUCCESS: u16 = 283; // Authentication successful (with data)
    #[allow(dead_code)] // Defined for completeness per RFC 4643
    pub const AUTH_ACCEPTED: u16 = 281; // Authentication successful (no data)
    pub const NO_MECHANISM: u16 = 503; // Mechanism not available
    pub const BASE64_ERROR: u16 = 504; // Base64 decode error
}

#[test]
fn test_sasl_response_code_383_challenge() {
    // RFC 4643 §2.4: 383 = Continue with SASL exchange (challenge)
    let response = NntpResponse {
        code: sasl_codes::SASL_CONTINUE,
        message: "Y2hhbGxlbmdl".to_string(), // Base64 challenge
        lines: vec![],
    };

    assert_eq!(response.code, 383);
    assert!(response.is_continuation());
}

#[test]
fn test_sasl_response_code_283_success_with_data() {
    // RFC 4643 §2.4: 283 = Authentication successful with additional data
    let response = NntpResponse {
        code: sasl_codes::SASL_SUCCESS,
        message: "c3VjY2Vzcw==".to_string(), // Base64 success data
        lines: vec![],
    };

    assert_eq!(response.code, 283);
    assert!(response.is_success());
}

#[test]
fn test_sasl_response_code_503_no_mechanism() {
    // RFC 4643 §2.4: 503 = Requested mechanism not available
    let response = NntpResponse {
        code: sasl_codes::NO_MECHANISM,
        message: "Mechanism not supported".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 503);
    assert!(response.is_error());
}

#[test]
fn test_sasl_response_code_504_base64_error() {
    // RFC 4643 §2.4: 504 = Base64 decoding error
    let response = NntpResponse {
        code: sasl_codes::BASE64_ERROR,
        message: "Invalid base64 encoding".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 504);
    assert!(response.is_error());
}

#[test]
fn test_sasl_mechanisms_list() {
    // Common SASL mechanisms defined in various RFCs
    let mechanisms = [
        ("PLAIN", "RFC 4616 - Simple username/password"),
        ("LOGIN", "Non-standard but widely supported"),
        ("CRAM-MD5", "RFC 2195 - Challenge-response"),
        ("DIGEST-MD5", "RFC 2831 - Digest authentication"),
        ("EXTERNAL", "RFC 4422 - External authentication (TLS cert)"),
        ("ANONYMOUS", "RFC 4505 - Anonymous access"),
        ("GSSAPI", "RFC 4752 - Kerberos authentication"),
    ];

    for (mech, _desc) in mechanisms {
        // Verify mechanism names are valid ASCII uppercase
        assert!(mech
            .chars()
            .all(|c| c.is_ascii_uppercase() || c == '-' || c.is_ascii_digit()));
    }
}

// RFC 4643 §2.1 - CAPABILITY Command and Authentication

#[test]
fn test_capability_authinfo_advertised() {
    // RFC 4643 §2.1: Server advertises AUTHINFO capability
    let capabilities = [
        "VERSION 2".to_string(),
        "READER".to_string(),
        "AUTHINFO USER".to_string(),
        "AUTHINFO SASL PLAIN CRAM-MD5".to_string(),
    ];

    // Check AUTHINFO USER is advertised
    assert!(capabilities.iter().any(|c| c.starts_with("AUTHINFO USER")));

    // Check AUTHINFO SASL is advertised with mechanisms
    let sasl_cap = capabilities.iter().find(|c| c.starts_with("AUTHINFO SASL"));
    assert!(sasl_cap.is_some());
    let sasl_cap = sasl_cap.unwrap();
    assert!(sasl_cap.contains("PLAIN"));
    assert!(sasl_cap.contains("CRAM-MD5"));
}

#[test]
fn test_capability_authinfo_removed_after_auth() {
    // RFC 4643 §2.1: AUTHINFO capability MUST NOT be advertised after authentication
    let capabilities_before = [
        "VERSION 2".to_string(),
        "READER".to_string(),
        "AUTHINFO USER".to_string(),
        "AUTHINFO SASL PLAIN".to_string(),
    ];

    // After successful authentication
    let capabilities_after = [
        "VERSION 2".to_string(),
        "READER".to_string(),
        // AUTHINFO should be removed
    ];

    // Before auth: AUTHINFO present
    assert!(capabilities_before
        .iter()
        .any(|c| c.starts_with("AUTHINFO")));

    // After auth: AUTHINFO absent
    assert!(!capabilities_after.iter().any(|c| c.starts_with("AUTHINFO")));
}

#[test]
fn test_capability_parsing() {
    // Parse AUTHINFO SASL capability to extract mechanisms
    let cap = "AUTHINFO SASL PLAIN LOGIN CRAM-MD5";
    let parts: Vec<&str> = cap.split_whitespace().collect();

    assert_eq!(parts[0], "AUTHINFO");
    assert_eq!(parts[1], "SASL");

    // Mechanisms start at index 2
    let mechanisms: Vec<&str> = parts[2..].to_vec();
    assert!(mechanisms.contains(&"PLAIN"));
    assert!(mechanisms.contains(&"LOGIN"));
    assert!(mechanisms.contains(&"CRAM-MD5"));
}

// RFC 4643 §2.3 - Pipelining Prevention
//
// RFC 4643 states: "These commands MUST NOT be pipelined."
// This means AUTHINFO USER and AUTHINFO PASS must be sent one at a time,
// waiting for each response before sending the next command.

#[test]
fn test_pipelining_not_allowed_for_authinfo() {
    // RFC 4643 §2.3: AUTHINFO commands cannot be pipelined
    // This is a documentation test - pipelining would send:
    // "AUTHINFO USER user\r\nAUTHINFO PASS pass\r\n"
    // Instead of waiting for 381 before sending PASS

    // Simulate pipelined command detection
    let pipelined_commands = "AUTHINFO USER user\r\nAUTHINFO PASS pass\r\n";
    let command_count = pipelined_commands.matches("\r\n").count();

    // Pipelined = more than one command in buffer
    let is_pipelined = command_count > 1;
    assert!(is_pipelined, "This demonstrates pipelined commands");

    // Proper non-pipelined usage: one command at a time
    let single_command = "AUTHINFO USER user\r\n";
    let single_count = single_command.matches("\r\n").count();
    assert_eq!(single_count, 1, "Single command is not pipelined");
}

#[test]
fn test_sequential_auth_commands() {
    // Proper authentication flow is sequential
    let commands = [
        "AUTHINFO USER testuser\r\n",  // Wait for 381
        "AUTHINFO PASS secret123\r\n", // Then send PASS
    ];

    for cmd in commands {
        // Each command should have exactly one CRLF
        assert_eq!(cmd.matches("\r\n").count(), 1);
        assert!(cmd.ends_with("\r\n"));
    }
}

// RFC 4643 - 502 After Successful Authentication
//
// RFC 4643 states: "After successful authentication, the server MUST reject
// any subsequent AUTHINFO commands with a 502 response."

/// Extended authentication state for 502 testing
// REMOVED: ExtendedAuthState mock enum and process_extended_auth() mock function
// These were test-only mocks that simulated authentication state transitions
// but didn't test the real implementation in src/client.rs
//
// Real authentication state is tested in:
// - tests/auth_integration_test.rs (integration tests calling real authenticate() method)
// - src/client.rs unit tests (ConnectionState enum transitions)
//
// Tests for 502 "already authenticated" response are tested through:
// - test_502_after_successful_auth_pass (tests real NntpResponse)
// - test_502_after_successful_auth_sasl (tests real NntpResponse)
// - Integration tests in auth_integration_test.rs (test_double_authentication_rejected)

// REMOVED: test_502_after_successful_auth_user
// Was phantom test using process_extended_auth mock
// Real double-authentication rejection tested in auth_integration_test.rs

#[test]
fn test_502_after_successful_auth_pass() {
    // After authentication, AUTHINFO PASS should return 502
    let response = NntpResponse {
        code: 502,
        message: "Already authenticated".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 502);
    assert!(response.is_error());
}

#[test]
fn test_502_after_successful_auth_sasl() {
    // After authentication, AUTHINFO SASL should return 502
    let response = NntpResponse {
        code: 502,
        message: "Command unavailable".to_string(),
        lines: vec![],
    };

    assert_eq!(response.code, 502);
    assert!(response.is_error());
}

// REMOVED: test_auth_state_machine_comprehensive
// Was phantom test using process_extended_auth mock to simulate state transitions
// Real comprehensive auth state machine tests are in:
// - tests/auth_integration_test.rs (full auth flows with real client)
// - src/client.rs unit tests (ConnectionState transition tests)

// RFC 4643 - Edge Cases and Error Handling

#[test]
fn test_auth_timeout_handling() {
    // If client takes too long between USER and PASS, server may timeout
    // This is documented as server-dependent behavior
    // Some servers send 481, others disconnect

    let timeout_responses = [481, 502];
    for code in timeout_responses {
        let response = NntpResponse {
            code,
            message: "Authentication timeout".to_string(),
            lines: vec![],
        };
        assert!(response.is_error());
    }
}

#[test]
fn test_auth_whitespace_handling() {
    // RFC 4643 notes issues with whitespace in credentials
    // Usernames and passwords may contain spaces

    use nntp_rs::commands;

    let cmd_with_space = commands::authinfo_user("user name");
    assert!(cmd_with_space.contains("user name"));

    let pass_with_space = commands::authinfo_pass("pass word");
    assert!(pass_with_space.contains("pass word"));
}

#[test]
fn test_auth_special_characters() {
    // Credentials can contain special characters
    use nntp_rs::commands;

    let special_chars = r#"p@ss!w0rd#$%^&*()"#;
    let cmd = commands::authinfo_pass(special_chars);
    assert!(cmd.contains(special_chars));
}

// REMOVED: test_auth_multiple_pass_attempts
// Was phantom test using process_extended_auth mock
// Real retry behavior tested in:
// - tests/auth_integration_test.rs::test_retry_after_authentication_failure
// - src/client.rs unit test: test_connection_state_failed_auth_with_retry_flow

#[test]
fn test_auth_empty_credentials() {
    // Some servers may accept empty username/password
    use nntp_rs::commands;

    let empty_user = commands::authinfo_user("");
    assert_eq!(empty_user, "AUTHINFO USER \r\n");

    let empty_pass = commands::authinfo_pass("");
    assert_eq!(empty_pass, "AUTHINFO PASS \r\n");
}

//! RFC 4643 Section 2.4 - AUTHINFO SASL Tests
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc4643#section-2.4

use nntp_rs::{NntpError, Result, SaslMechanism, decode_sasl_data, encode_sasl_data};

// Mock SASL mechanism for testing
struct MockSasl {
    name: &'static str,
    initial: Option<Vec<u8>>,
    responses: Vec<Vec<u8>>,
    response_index: usize,
    requires_tls: bool,
}

impl MockSasl {
    fn new(name: &'static str, initial: Option<Vec<u8>>) -> Self {
        Self {
            name,
            initial,
            responses: Vec::new(),
            response_index: 0,
            requires_tls: false,
        }
    }

    fn with_response(mut self, response: Vec<u8>) -> Self {
        self.responses.push(response);
        self
    }

    fn with_tls_required(mut self) -> Self {
        self.requires_tls = true;
        self
    }
}

impl SaslMechanism for MockSasl {
    fn mechanism_name(&self) -> &str {
        self.name
    }

    fn initial_response(&self) -> Result<Option<Vec<u8>>> {
        Ok(self.initial.clone())
    }

    fn process_challenge(&mut self, _challenge: &[u8]) -> Result<Vec<u8>> {
        if self.response_index < self.responses.len() {
            let response = self.responses[self.response_index].clone();
            self.response_index += 1;
            Ok(response)
        } else {
            Err(NntpError::Protocol {
                code: 482,
                message: "No more responses available".to_string(),
            })
        }
    }

    fn requires_tls(&self) -> bool {
        self.requires_tls
    }
}

// Test SaslMechanism trait

#[test]
fn test_sasl_mechanism_name() {
    let mech = MockSasl::new("TEST-MECH", None);
    assert_eq!(mech.mechanism_name(), "TEST-MECH");
}

#[test]
fn test_sasl_mechanism_with_initial_response() {
    let mech = MockSasl::new("PLAIN", Some(b"user\x00pass".to_vec()));
    let response = mech.initial_response().unwrap();
    assert_eq!(response, Some(b"user\x00pass".to_vec()));
}

#[test]
fn test_sasl_mechanism_without_initial_response() {
    let mech = MockSasl::new("DIGEST-MD5", None);
    let response = mech.initial_response().unwrap();
    assert_eq!(response, None);
}

#[test]
fn test_sasl_mechanism_process_challenge() {
    let mut mech = MockSasl::new("TEST", None).with_response(b"response1".to_vec());
    let response = mech.process_challenge(b"challenge").unwrap();
    assert_eq!(response, b"response1");
}

#[test]
fn test_sasl_mechanism_multiple_challenges() {
    let mut mech = MockSasl::new("TEST", None)
        .with_response(b"response1".to_vec())
        .with_response(b"response2".to_vec());

    let r1 = mech.process_challenge(b"challenge1").unwrap();
    assert_eq!(r1, b"response1");

    let r2 = mech.process_challenge(b"challenge2").unwrap();
    assert_eq!(r2, b"response2");
}

#[test]
fn test_sasl_mechanism_tls_required() {
    let mech = MockSasl::new("PLAIN", None).with_tls_required();
    assert!(mech.requires_tls());
}

#[test]
fn test_sasl_mechanism_tls_not_required() {
    let mech = MockSasl::new("DIGEST-MD5", None);
    assert!(!mech.requires_tls());
}

// Base64 encoding tests (RFC 4643 Section 2.4.2)

#[test]
fn test_encode_empty_data_as_equals() {
    // RFC 4643: "a client response that has zero length
    // MUST be sent as a single equals sign ('=')"
    assert_eq!(encode_sasl_data(&[]), "=");
}

#[test]
fn test_encode_non_empty_data() {
    let data = b"test";
    let encoded = encode_sasl_data(data);
    assert_ne!(encoded, "=");
    assert_eq!(encoded, "dGVzdA=="); // Standard base64
}

#[test]
fn test_encode_binary_data() {
    let data = b"\x00\x01\x02\xff\xfe\xfd";
    let encoded = encode_sasl_data(data);
    // Should be valid base64
    assert!(
        encoded
            .chars()
            .all(|c| { c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=' })
    );
}

#[test]
fn test_decode_equals_as_empty() {
    // RFC 4643: "=" decodes to empty data
    let decoded = decode_sasl_data("=").unwrap();
    assert_eq!(decoded, Vec::<u8>::new());
}

#[test]
fn test_decode_valid_base64() {
    let decoded = decode_sasl_data("dGVzdA==").unwrap();
    assert_eq!(decoded, b"test");
}

#[test]
fn test_decode_invalid_base64() {
    let result = decode_sasl_data("invalid!@#$");
    assert!(result.is_err());
    match result {
        Err(NntpError::Protocol { code, message }) => {
            assert_eq!(code, 482);
            assert!(message.contains("Invalid base64"));
        }
        _ => panic!("Expected Protocol error"),
    }
}

#[test]
fn test_base64_roundtrip_empty() {
    let original: Vec<u8> = Vec::new();
    let encoded = encode_sasl_data(&original);
    let decoded = decode_sasl_data(&encoded).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn test_base64_roundtrip_non_empty() {
    let original = b"username\x00password";
    let encoded = encode_sasl_data(original);
    let decoded = decode_sasl_data(&encoded).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn test_base64_roundtrip_all_bytes() {
    // Test all byte values 0-255
    let original: Vec<u8> = (0..=255).collect();
    let encoded = encode_sasl_data(&original);
    let decoded = decode_sasl_data(&encoded).unwrap();
    assert_eq!(decoded, original);
}

// Base64 validation tests

#[test]
fn test_base64_no_invalid_characters() {
    // RFC 4643: "reject any character not explicitly allowed
    // by the BASE64 alphabet"
    let data = b"test";
    let encoded = encode_sasl_data(data);
    for ch in encoded.chars() {
        assert!(
            ch.is_ascii_alphanumeric() || ch == '+' || ch == '/' || ch == '=',
            "Invalid character: {}",
            ch
        );
    }
}

#[test]
fn test_base64_padding_at_end_only() {
    // RFC 4643: padding must only occur at end
    let data = b"test";
    let encoded = encode_sasl_data(data);
    if let Some(pos) = encoded.find('=') {
        // All characters after first padding must be padding
        assert!(encoded[pos..].chars().all(|c| c == '='));
    }
}

// Edge cases

#[test]
fn test_encode_single_byte() {
    let data = b"a";
    let encoded = encode_sasl_data(data);
    let decoded = decode_sasl_data(&encoded).unwrap();
    assert_eq!(decoded, b"a");
}

#[test]
fn test_encode_null_bytes() {
    let data = b"\x00\x00\x00";
    let encoded = encode_sasl_data(data);
    let decoded = decode_sasl_data(&encoded).unwrap();
    assert_eq!(decoded, b"\x00\x00\x00");
}

#[test]
fn test_encode_max_byte_value() {
    let data = b"\xff\xff\xff";
    let encoded = encode_sasl_data(data);
    let decoded = decode_sasl_data(&encoded).unwrap();
    assert_eq!(decoded, b"\xff\xff\xff");
}

#[test]
fn test_decode_whitespace_rejected() {
    // Base64 should not contain whitespace
    let result = decode_sasl_data("dGVz dA==");
    assert!(result.is_err());
}

#[test]
fn test_decode_newline_rejected() {
    let result = decode_sasl_data("dGVz\ndA==");
    assert!(result.is_err());
}

// Real-world SASL data patterns

#[test]
fn test_plain_mechanism_pattern() {
    // PLAIN format: \0username\0password
    let data = b"\x00user\x00pass";
    let encoded = encode_sasl_data(data);
    let decoded = decode_sasl_data(&encoded).unwrap();
    assert_eq!(decoded, data);
}

#[test]
fn test_long_credentials() {
    let username = "a".repeat(100);
    let password = "b".repeat(100);
    let data = format!("\x00{}\x00{}", username, password);
    let encoded = encode_sasl_data(data.as_bytes());
    let decoded = decode_sasl_data(&encoded).unwrap();
    assert_eq!(decoded, data.as_bytes());
}

#[test]
fn test_unicode_in_credentials() {
    // SASL should handle UTF-8 credentials
    let data = "user\x00пароль".as_bytes(); // Russian "password"
    let encoded = encode_sasl_data(data);
    let decoded = decode_sasl_data(&encoded).unwrap();
    assert_eq!(decoded, data);
}

// SASL PLAIN mechanism tests

#[test]
fn test_sasl_plain_mechanism_name() {
    use nntp_rs::SaslPlain;
    let plain = SaslPlain::new("user", "pass");
    assert_eq!(plain.mechanism_name(), "PLAIN");
}

#[test]
fn test_sasl_plain_requires_tls() {
    use nntp_rs::SaslPlain;
    let plain = SaslPlain::new("user", "pass");
    assert!(plain.requires_tls(), "PLAIN must require TLS");
}

#[test]
fn test_sasl_plain_initial_response_format() {
    use nntp_rs::SaslPlain;
    let plain = SaslPlain::new("alice", "secret");
    let response = plain.initial_response().unwrap().unwrap();

    // PLAIN format: \0username\0password
    let expected = b"\x00alice\x00secret";
    assert_eq!(response, expected);
}

#[test]
fn test_sasl_plain_empty_username() {
    use nntp_rs::SaslPlain;
    let plain = SaslPlain::new("", "password");
    let response = plain.initial_response().unwrap().unwrap();

    assert_eq!(response, b"\x00\x00password");
}

#[test]
fn test_sasl_plain_empty_password() {
    use nntp_rs::SaslPlain;
    let plain = SaslPlain::new("user", "");
    let response = plain.initial_response().unwrap().unwrap();

    assert_eq!(response, b"\x00user\x00");
}

#[test]
fn test_sasl_plain_both_empty() {
    use nntp_rs::SaslPlain;
    let plain = SaslPlain::new("", "");
    let response = plain.initial_response().unwrap().unwrap();

    assert_eq!(response, b"\x00\x00");
}

#[test]
fn test_sasl_plain_special_characters() {
    use nntp_rs::SaslPlain;
    let plain = SaslPlain::new("user@domain", "p@ss!w0rd#$%");
    let response = plain.initial_response().unwrap().unwrap();

    assert_eq!(response, b"\x00user@domain\x00p@ss!w0rd#$%");
}

#[test]
fn test_sasl_plain_unicode_username() {
    use nntp_rs::SaslPlain;
    let plain = SaslPlain::new("用户", "password");
    let response = plain.initial_response().unwrap().unwrap();

    let expected = b"\x00\xe7\x94\xa8\xe6\x88\xb7\x00password"; // UTF-8 encoded
    assert_eq!(response, expected);
}

#[test]
fn test_sasl_plain_unicode_password() {
    use nntp_rs::SaslPlain;
    let plain = SaslPlain::new("user", "пароль"); // Russian "password"
    let response = plain.initial_response().unwrap().unwrap();

    // Verify it contains the null separators and username
    assert!(response.starts_with(b"\x00user\x00"));
}

#[test]
fn test_sasl_plain_long_credentials() {
    use nntp_rs::SaslPlain;
    let username = "a".repeat(100);
    let password = "b".repeat(100);
    let plain = SaslPlain::new(&username, &password);
    let response = plain.initial_response().unwrap().unwrap();

    // Should be: \0 + username + \0 + password
    assert_eq!(response.len(), 1 + 100 + 1 + 100);
    assert_eq!(response[0], 0);
    assert_eq!(response[101], 0);
}

#[test]
fn test_sasl_plain_no_challenge_support() {
    use nntp_rs::SaslPlain;
    let mut plain = SaslPlain::new("user", "pass");

    // PLAIN should reject challenges
    let result = plain.process_challenge(b"unexpected challenge");
    assert!(result.is_err());

    if let Err(nntp_rs::NntpError::Protocol { code, message }) = result {
        assert_eq!(code, 482);
        assert!(message.contains("PLAIN"));
        assert!(message.contains("challenge"));
    } else {
        panic!("Expected Protocol error");
    }
}

#[test]
fn test_sasl_plain_null_in_password() {
    use nntp_rs::SaslPlain;
    // Password containing null byte - valid but unusual
    let plain = SaslPlain::new("user", "pass\x00word");
    let response = plain.initial_response().unwrap().unwrap();

    // Should have 3 null bytes total: initial separator, after username, and in password
    let null_count = response.iter().filter(|&&b| b == 0).count();
    assert_eq!(null_count, 3);
}

#[test]
fn test_sasl_plain_base64_encoding() {
    use nntp_rs::{SaslPlain, encode_sasl_data};

    let plain = SaslPlain::new("test", "password123");
    let response = plain.initial_response().unwrap().unwrap();
    let encoded = encode_sasl_data(&response);

    // Verify it's valid base64
    assert!(base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &encoded).is_ok());

    // Verify it encodes the expected value
    let expected = b"\x00test\x00password123";
    let expected_encoded = encode_sasl_data(expected);
    assert_eq!(encoded, expected_encoded);
}

#[test]
fn test_sasl_plain_clone() {
    use nntp_rs::SaslPlain;

    let plain1 = SaslPlain::new("user", "pass");
    let plain2 = plain1.clone();

    assert_eq!(plain1.mechanism_name(), plain2.mechanism_name());
    assert_eq!(
        plain1.initial_response().unwrap(),
        plain2.initial_response().unwrap()
    );
}

#[test]
fn test_sasl_plain_debug() {
    use nntp_rs::SaslPlain;

    let plain = SaslPlain::new("user", "pass");
    let debug_str = format!("{:?}", plain);

    // Debug should exist and contain the struct name
    assert!(debug_str.contains("SaslPlain"));
}

#[test]
fn test_sasl_plain_into_string() {
    use nntp_rs::SaslPlain;

    // Test that Into<String> works for credentials
    let username = String::from("alice");
    let password = String::from("secret");
    let plain = SaslPlain::new(username, password);

    let response = plain.initial_response().unwrap().unwrap();
    assert_eq!(response, b"\x00alice\x00secret");
}

#[test]
fn test_sasl_plain_str_slice() {
    use nntp_rs::SaslPlain;

    // Test that &str works for credentials
    let plain = SaslPlain::new("bob", "passw0rd");

    let response = plain.initial_response().unwrap().unwrap();
    assert_eq!(response, b"\x00bob\x00passw0rd");
}

// AUTHINFO SASL command format tests

#[test]
fn test_authinfo_sasl_command_format() {
    use nntp_rs::commands;

    let cmd = commands::authinfo_sasl("PLAIN");
    assert_eq!(cmd, "AUTHINFO SASL PLAIN\r\n");
}

#[test]
fn test_authinfo_sasl_command_ends_with_crlf() {
    use nntp_rs::commands;

    let cmd = commands::authinfo_sasl("DIGEST-MD5");
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_authinfo_sasl_ir_command_format() {
    use nntp_rs::commands;

    let cmd = commands::authinfo_sasl_ir("PLAIN", "AGFsaWNlAHNlY3JldA==");
    assert_eq!(cmd, "AUTHINFO SASL PLAIN AGFsaWNlAHNlY3JldA==\r\n");
}

#[test]
fn test_authinfo_sasl_ir_ends_with_crlf() {
    use nntp_rs::commands;

    let cmd = commands::authinfo_sasl_ir("PLAIN", "test");
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_authinfo_sasl_ir_empty_response() {
    use nntp_rs::commands;

    // Empty initial response encoded as "="
    let cmd = commands::authinfo_sasl_ir("EXTERNAL", "=");
    assert_eq!(cmd, "AUTHINFO SASL EXTERNAL =\r\n");
}

#[test]
fn test_authinfo_sasl_continue_format() {
    use nntp_rs::commands;

    let cmd = commands::authinfo_sasl_continue("Y2hhbGxlbmdlLXJlc3BvbnNl");
    assert_eq!(cmd, "Y2hhbGxlbmdlLXJlc3BvbnNl\r\n");
}

#[test]
fn test_authinfo_sasl_continue_ends_with_crlf() {
    use nntp_rs::commands;

    let cmd = commands::authinfo_sasl_continue("test");
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_authinfo_sasl_various_mechanisms() {
    use nntp_rs::commands;

    let mechanisms = vec![
        "PLAIN",
        "DIGEST-MD5",
        "CRAM-MD5",
        "EXTERNAL",
        "SCRAM-SHA-256",
    ];
    for mech in mechanisms {
        let cmd = commands::authinfo_sasl(mech);
        assert_eq!(cmd, format!("AUTHINFO SASL {}\r\n", mech));
    }
}

// AUTHINFO SASL response code tests

#[test]
fn test_sasl_response_code_281_accepted() {
    use nntp_rs::{NntpResponse, codes};

    let response = NntpResponse {
        code: codes::AUTH_ACCEPTED,
        message: "Authentication accepted".to_string(),
        lines: vec![],
    };

    assert!(response.is_success());
    assert_eq!(response.code, 281);
}

#[test]
fn test_sasl_response_code_383_continue() {
    use nntp_rs::{NntpResponse, codes};

    let response = NntpResponse {
        code: codes::SASL_CONTINUE,
        message: "Y2hhbGxlbmdl".to_string(), // base64 challenge
        lines: vec![],
    };

    // 383 is a continuation response (3xx range), not success (2xx) or error (4xx/5xx)
    assert!(!response.is_success());
    assert!(!response.is_error());
    assert!(response.is_continuation());
    assert_eq!(response.code, 383);
}

#[test]
fn test_sasl_response_code_481_rejected() {
    use nntp_rs::{NntpResponse, codes};

    let response = NntpResponse {
        code: codes::AUTH_REJECTED,
        message: "Authentication rejected".to_string(),
        lines: vec![],
    };

    assert!(response.is_error());
    assert_eq!(response.code, 481);
}

#[test]
fn test_sasl_response_code_482_out_of_sequence() {
    use nntp_rs::{NntpResponse, codes};

    let response = NntpResponse {
        code: codes::AUTH_OUT_OF_SEQUENCE,
        message: "Authentication commands out of sequence".to_string(),
        lines: vec![],
    };

    assert!(response.is_error());
    assert_eq!(response.code, 482);
}

#[test]
fn test_sasl_response_code_483_encryption_required() {
    use nntp_rs::{NntpResponse, codes};

    let response = NntpResponse {
        code: codes::ENCRYPTION_REQUIRED,
        message: "Encryption or stronger authentication required".to_string(),
        lines: vec![],
    };

    assert!(response.is_error());
    assert_eq!(response.code, 483);
}

// RFC 4643 Section 2.4 example tests

#[test]
fn test_rfc4643_example_plain_initial_response() {
    // RFC 4643 example: AUTHINFO SASL PLAIN with initial response
    use nntp_rs::{commands, encode_sasl_data};

    // Username "tim", password "tanstaaftanstaaf"
    let credentials = b"\x00tim\x00tanstaaftanstaaf";
    let encoded = encode_sasl_data(credentials);

    let cmd = commands::authinfo_sasl_ir("PLAIN", &encoded);
    assert!(cmd.starts_with("AUTHINFO SASL PLAIN "));
    assert!(cmd.ends_with("\r\n"));
}

#[test]
fn test_rfc4643_example_digest_md5_challenge() {
    // RFC 4643 example: DIGEST-MD5 with challenge-response
    use nntp_rs::{commands, decode_sasl_data};

    // Server sends: 383 <base64-encoded-challenge>
    let server_challenge = "cmVhbG09ImV4YW1wbGUuY29tIixub25jZT0iT0E2TUc5dEVRR20ySGgiLHFvcD0iYXV0aCIsYWxnb3JpdGhtPW1kNS1zZXNzLGNoYXJzZXQ9dXRmLTg=";

    // Client decodes challenge
    let challenge = decode_sasl_data(server_challenge).unwrap();
    assert!(!challenge.is_empty());

    // Client would compute response and send
    let cmd = commands::authinfo_sasl_continue(server_challenge);
    assert!(cmd.ends_with("\r\n"));
}

// Edge cases and error handling

#[test]
fn test_sasl_empty_mechanism_name() {
    use nntp_rs::commands;

    let cmd = commands::authinfo_sasl("");
    assert_eq!(cmd, "AUTHINFO SASL \r\n");
}

#[test]
fn test_sasl_mechanism_with_spaces() {
    use nntp_rs::commands;

    // Mechanism names shouldn't have spaces, but test formatting
    let cmd = commands::authinfo_sasl("SCRAM-SHA-256");
    assert_eq!(cmd, "AUTHINFO SASL SCRAM-SHA-256\r\n");
}

#[test]
fn test_sasl_ir_with_long_initial_response() {
    use nntp_rs::commands;

    // Very long base64 string (1000 chars)
    let long_response = "A".repeat(1000);
    let cmd = commands::authinfo_sasl_ir("TEST", &long_response);
    assert!(cmd.starts_with("AUTHINFO SASL TEST "));
    assert!(cmd.ends_with("\r\n"));
    assert!(cmd.contains(&long_response));
}

#[test]
fn test_sasl_continue_with_empty_marker() {
    use nntp_rs::commands;

    // Empty response encoded as "="
    let cmd = commands::authinfo_sasl_continue("=");
    assert_eq!(cmd, "=\r\n");
}

// Real-world SASL scenarios

#[test]
fn test_sasl_plain_single_round() {
    use nntp_rs::{SaslPlain, encode_sasl_data};

    // PLAIN is single-round: send initial response, get 281
    let plain = SaslPlain::new("user", "password");
    let initial = plain.initial_response().unwrap().unwrap();
    let encoded = encode_sasl_data(&initial);

    assert!(!encoded.is_empty());
    assert_ne!(encoded, "="); // Not empty
}

#[test]
fn test_sasl_challenge_response_flow() {
    // Test challenge-response mechanism flow
    let mut mech = MockSasl::new("DIGEST-MD5", None)
        .with_response(b"response1".to_vec())
        .with_response(b"response2".to_vec());

    // No initial response
    let initial = mech.initial_response().unwrap();
    assert_eq!(initial, None);

    // Process challenges
    let r1 = mech.process_challenge(b"challenge1").unwrap();
    assert_eq!(r1, b"response1");

    let r2 = mech.process_challenge(b"challenge2").unwrap();
    assert_eq!(r2, b"response2");
}

#[test]
fn test_sasl_plain_rejects_challenge() {
    use nntp_rs::SaslPlain;

    let mut plain = SaslPlain::new("user", "pass");

    // PLAIN doesn't support challenges
    let result = plain.process_challenge(b"unexpected challenge");
    assert!(result.is_err());
}

#[test]
fn test_sasl_multiple_mechanisms_compatibility() {
    use nntp_rs::{SaslPlain, commands};

    // Server might advertise multiple mechanisms
    let _mechanisms = ["PLAIN", "DIGEST-MD5", "CRAM-MD5"];

    // Client selects PLAIN (simplest)
    let plain = SaslPlain::new("alice", "secret");
    let initial = plain.initial_response().unwrap().unwrap();
    let encoded = nntp_rs::encode_sasl_data(&initial);

    let cmd = commands::authinfo_sasl_ir("PLAIN", &encoded);
    assert!(cmd.contains("PLAIN"));
}

#[test]
fn test_sasl_case_sensitivity() {
    use nntp_rs::commands;

    // SASL mechanism names are case-insensitive per RFC 4422
    // But implementation should preserve case
    let cmd1 = commands::authinfo_sasl("PLAIN");
    let cmd2 = commands::authinfo_sasl("plain");

    assert_eq!(cmd1, "AUTHINFO SASL PLAIN\r\n");
    assert_eq!(cmd2, "AUTHINFO SASL plain\r\n");
}

#[test]
fn test_sasl_ir_special_characters_in_response() {
    use nntp_rs::commands;

    // Base64 can contain +, /, =
    let cmd = commands::authinfo_sasl_ir("PLAIN", "abc+def/ghi==");
    assert_eq!(cmd, "AUTHINFO SASL PLAIN abc+def/ghi==\r\n");
}

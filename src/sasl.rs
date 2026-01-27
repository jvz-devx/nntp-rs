//! SASL (Simple Authentication and Security Layer) support for NNTP
//!
//! This module implements RFC 4643 AUTHINFO SASL authentication.
//!
//! # SASL Mechanisms
//!
//! The SASL framework supports multiple authentication mechanisms:
//! - PLAIN: Simple username/password authentication (requires TLS)
//! - Others can be implemented by providing a `SaslMechanism` implementation
//!
//! # Example
//!
//! ```no_run
//! # use nntp_rs::{NntpClient, SaslPlain, ServerConfig};
//! # use std::sync::Arc;
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # let config = ServerConfig::plain("news.example.com", "user", "pass");
//! let mut client = NntpClient::connect(Arc::new(config)).await?;
//!
//! // Authenticate using SASL PLAIN
//! let mechanism = SaslPlain::new("username", "password");
//! client.authenticate_sasl(mechanism).await?;
//! # Ok(())
//! # }
//! ```

use crate::{NntpError, Result};
use base64::{Engine, engine::general_purpose::STANDARD};

/// Trait for SASL authentication mechanisms
///
/// Implement this trait to add support for additional SASL mechanisms.
pub trait SaslMechanism: Send + Sync {
    /// Returns the name of the SASL mechanism (e.g., "PLAIN", "DIGEST-MD5")
    fn mechanism_name(&self) -> &str;

    /// Generate the initial client response
    ///
    /// Returns `None` if the mechanism doesn't support initial responses.
    /// Returns `Some(data)` where data will be base64-encoded by the framework.
    fn initial_response(&self) -> Result<Option<Vec<u8>>>;

    /// Process a server challenge and generate a client response
    ///
    /// # Arguments
    ///
    /// * `challenge` - Base64-decoded challenge data from server (383 response)
    ///
    /// # Returns
    ///
    /// The client response data which will be base64-encoded by the framework.
    fn process_challenge(&mut self, challenge: &[u8]) -> Result<Vec<u8>>;

    /// Check if the mechanism requires TLS/encryption
    ///
    /// Returns `true` if the mechanism should only be used over secure connections.
    fn requires_tls(&self) -> bool {
        false
    }
}

/// Base64-encode data for SASL exchange
///
/// Empty data is encoded as "=" per RFC 4643.
pub fn encode_sasl_data(data: &[u8]) -> String {
    if data.is_empty() {
        "=".to_string()
    } else {
        STANDARD.encode(data)
    }
}

/// Base64-decode data from SASL exchange
///
/// "=" is decoded as empty data per RFC 4643.
pub fn decode_sasl_data(encoded: &str) -> Result<Vec<u8>> {
    if encoded == "=" {
        return Ok(Vec::new());
    }

    STANDARD.decode(encoded).map_err(|e| NntpError::Protocol {
        code: 482,
        message: format!("Invalid base64 in SASL response: {}", e),
    })
}

/// SASL PLAIN mechanism implementation
///
/// PLAIN is a simple username/password authentication mechanism.
/// It sends credentials in the format: `\0username\0password`
///
/// # Security Warning
///
/// PLAIN sends credentials in cleartext (albeit base64-encoded). It **must**
/// only be used over TLS-encrypted connections. This implementation enforces
/// that requirement via `requires_tls()`.
///
/// # Example
///
/// ```no_run
/// # use nntp_rs::{NntpClient, SaslPlain, ServerConfig};
/// # use std::sync::Arc;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = ServerConfig::tls("news.example.com", "user", "pass");
/// let mut client = NntpClient::connect(Arc::new(config)).await?;
/// let mechanism = SaslPlain::new("alice", "secret123");
/// client.authenticate_sasl(mechanism).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct SaslPlain {
    username: String,
    password: String,
}

impl SaslPlain {
    /// Create a new SASL PLAIN mechanism with the given credentials
    ///
    /// # Arguments
    ///
    /// * `username` - The username for authentication
    /// * `password` - The password for authentication
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
        }
    }
}

impl SaslMechanism for SaslPlain {
    fn mechanism_name(&self) -> &str {
        "PLAIN"
    }

    fn initial_response(&self) -> Result<Option<Vec<u8>>> {
        // PLAIN format: \0username\0password
        let mut response = Vec::new();
        response.push(0); // authorization identity (empty)
        response.extend_from_slice(self.username.as_bytes());
        response.push(0);
        response.extend_from_slice(self.password.as_bytes());
        Ok(Some(response))
    }

    fn process_challenge(&mut self, _challenge: &[u8]) -> Result<Vec<u8>> {
        // PLAIN doesn't use challenges - this shouldn't be called
        Err(NntpError::Protocol {
            code: 482,
            message: "PLAIN mechanism does not support challenge-response".to_string(),
        })
    }

    fn requires_tls(&self) -> bool {
        true // PLAIN must only be used over TLS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_empty_data() {
        assert_eq!(encode_sasl_data(&[]), "=");
    }

    #[test]
    fn test_encode_non_empty_data() {
        let data = b"test data";
        let encoded = encode_sasl_data(data);
        assert_ne!(encoded, "=");
        // Should be valid base64
        assert!(STANDARD.decode(&encoded).is_ok());
    }

    #[test]
    fn test_decode_empty_marker() {
        let decoded = decode_sasl_data("=").unwrap();
        assert_eq!(decoded, Vec::<u8>::new());
    }

    #[test]
    fn test_decode_valid_base64() {
        let original = b"test data";
        let encoded = STANDARD.encode(original);
        let decoded = decode_sasl_data(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_decode_invalid_base64() {
        let result = decode_sasl_data("not!valid!base64!");
        assert!(result.is_err());
    }

    #[test]
    fn test_roundtrip_encoding() {
        let test_cases = vec![
            vec![],
            vec![0],
            vec![255],
            b"hello".to_vec(),
            b"username\x00password".to_vec(),
            (0..=255).collect::<Vec<u8>>(),
        ];

        for data in test_cases {
            let encoded = encode_sasl_data(&data);
            let decoded = decode_sasl_data(&encoded).unwrap();
            assert_eq!(decoded, data, "Roundtrip failed for data: {:?}", data);
        }
    }

    #[test]
    fn test_base64_alphabet_only() {
        // Valid base64 should only contain A-Z, a-z, 0-9, +, /, =
        let data = b"test";
        let encoded = encode_sasl_data(data);
        for ch in encoded.chars() {
            assert!(
                ch.is_ascii_alphanumeric() || ch == '+' || ch == '/' || ch == '=',
                "Invalid character in base64: {}",
                ch
            );
        }
    }

    #[test]
    fn test_padding_at_end_only() {
        let data = b"test";
        let encoded = encode_sasl_data(data);
        if let Some(pos) = encoded.find('=') {
            // If padding exists, it should be at the end
            assert!(
                encoded[pos..].chars().all(|c| c == '='),
                "Padding not at end: {}",
                encoded
            );
        }
    }
}

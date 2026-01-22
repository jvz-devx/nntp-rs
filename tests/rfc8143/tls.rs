//! RFC 8143 - TLS Configuration Tests
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc8143
//!
//! Tests for TLS security features including certificate validation and
//! insecure mode for self-signed certificates.

use nntp_rs::ServerConfig;

/// Test that ServerConfig defaults to secure certificate validation
#[test]
fn test_default_secure_tls() {
    let config = ServerConfig::tls("news.example.com", "user", "pass");
    assert!(config.tls, "TLS should be enabled");
    assert!(
        !config.allow_insecure_tls,
        "Certificate validation should be enabled by default"
    );
}

/// Test that ServerConfig::new creates secure configuration
#[test]
fn test_new_secure_by_default() {
    let config = ServerConfig::new("news.example.com", 563, true, "user", "pass");
    assert!(config.tls);
    assert!(!config.allow_insecure_tls);
}

/// Test that tls_insecure helper enables insecure mode
#[test]
fn test_tls_insecure_helper() {
    let config = ServerConfig::tls_insecure("localhost", "user", "pass");
    assert!(config.tls, "TLS should be enabled");
    assert!(config.allow_insecure_tls, "Insecure mode should be enabled");
    assert_eq!(config.port, 563, "Should use standard TLS port");
    assert_eq!(config.host, "localhost");
}

/// Test that allow_insecure_tls can be set manually
#[test]
fn test_manual_insecure_tls_flag() {
    let mut config = ServerConfig::tls("news.example.com", "user", "pass");
    assert!(!config.allow_insecure_tls);

    config.allow_insecure_tls = true;
    assert!(config.allow_insecure_tls);
}

/// Test that plain connections don't have insecure TLS flag set
#[test]
fn test_plain_connection_insecure_flag() {
    let config = ServerConfig::plain("news.example.com", "user", "pass");
    assert!(!config.tls);
    assert!(!config.allow_insecure_tls);
}

/// Test that insecure mode is separate from TLS on/off
#[test]
fn test_insecure_mode_separate_from_tls() {
    let mut config = ServerConfig::new("news.example.com", 563, false, "user", "pass");
    assert!(!config.tls);
    assert!(!config.allow_insecure_tls);

    // Setting insecure flag doesn't enable TLS
    config.allow_insecure_tls = true;
    assert!(!config.tls, "TLS should remain disabled");
    assert!(config.allow_insecure_tls);
}

/// Test that we can create various insecure configurations
#[test]
fn test_various_insecure_configs() {
    // Localhost with self-signed cert
    let local = ServerConfig::tls_insecure("localhost", "testuser", "testpass");
    assert_eq!(local.host, "localhost");
    assert!(local.allow_insecure_tls);

    // Development server with self-signed cert
    let dev = ServerConfig::tls_insecure("dev.news.local", "devuser", "devpass");
    assert_eq!(dev.host, "dev.news.local");
    assert!(dev.allow_insecure_tls);

    // IP address with self-signed cert
    let ip = ServerConfig::tls_insecure("192.168.1.100", "user", "pass");
    assert_eq!(ip.host, "192.168.1.100");
    assert!(ip.allow_insecure_tls);
}

/// Test Clone trait works with insecure flag
#[test]
fn test_clone_preserves_insecure_flag() {
    let config1 = ServerConfig::tls_insecure("localhost", "user", "pass");
    let config2 = config1.clone();

    assert_eq!(config1.host, config2.host);
    assert_eq!(config1.port, config2.port);
    assert_eq!(config1.tls, config2.tls);
    assert_eq!(config1.allow_insecure_tls, config2.allow_insecure_tls);
    assert!(config2.allow_insecure_tls);
}

/// Test Debug trait includes insecure flag
#[test]
fn test_debug_shows_insecure_flag() {
    let config = ServerConfig::tls_insecure("localhost", "user", "pass");
    let debug_str = format!("{:?}", config);

    assert!(
        debug_str.contains("allow_insecure_tls"),
        "Debug output should mention allow_insecure_tls field"
    );
}

#[cfg(feature = "serde")]
mod serde_tests {
    use super::*;

    /// Test that allow_insecure_tls serializes and deserializes
    #[test]
    fn test_serde_insecure_flag() {
        let config = ServerConfig::tls_insecure("localhost", "user", "pass");
        let json = serde_json::to_string(&config).expect("Failed to serialize");

        assert!(
            json.contains("allow_insecure_tls"),
            "Serialized JSON should contain allow_insecure_tls field"
        );

        let deserialized: ServerConfig =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert!(deserialized.allow_insecure_tls);
        assert_eq!(deserialized.host, "localhost");
    }

    /// Test that missing allow_insecure_tls defaults to false during deserialization
    #[test]
    fn test_serde_default_insecure_flag() {
        let json = r#"{
            "host": "news.example.com",
            "port": 563,
            "tls": true,
            "username": "user",
            "password": "pass"
        }"#;

        let config: ServerConfig = serde_json::from_str(json).expect("Failed to deserialize");
        assert!(!config.allow_insecure_tls, "Should default to false");
    }
}

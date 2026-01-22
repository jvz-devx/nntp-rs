//! Integration tests for nntp-rs
//!
//! These tests verify the public API works correctly.
//! They do not require a real NNTP server.

use nntp_rs::{NntpError, RetryConfig, ServerConfig};

#[test]
fn test_server_config_creation() {
    let config = ServerConfig::new("news.example.com", 563, true, "user", "pass");
    assert_eq!(config.host, "news.example.com");
    assert_eq!(config.port, 563);
    assert!(config.tls);
    assert_eq!(config.username, "user");
    assert_eq!(config.password, "pass");
}

#[test]
fn test_server_config_tls_helper() {
    let config = ServerConfig::tls("news.example.com", "user", "pass");
    assert_eq!(config.host, "news.example.com");
    assert_eq!(config.port, 563);
    assert!(config.tls);
}

#[test]
fn test_server_config_plain_helper() {
    let config = ServerConfig::plain("news.example.com", "user", "pass");
    assert_eq!(config.host, "news.example.com");
    assert_eq!(config.port, 119);
    assert!(!config.tls);
}

#[test]
fn test_retry_config_default() {
    let config = RetryConfig::default();
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.initial_backoff_ms, 100);
    assert_eq!(config.max_backoff_ms, 10000);
    assert_eq!(config.backoff_multiplier, 2.0);
    assert!(config.jitter);
}

#[test]
fn test_retry_config_no_retry() {
    let config = RetryConfig::no_retry();
    assert_eq!(config.max_retries, 0);
}

#[test]
fn test_retry_config_with_max_retries() {
    let config = RetryConfig::with_max_retries(5);
    assert_eq!(config.max_retries, 5);
    assert_eq!(config.initial_backoff_ms, 100); // Should keep other defaults
}

#[test]
fn test_error_display() {
    let err = NntpError::Timeout;
    assert_eq!(err.to_string(), "Connection timeout");

    let err = NntpError::AuthFailed("invalid credentials".to_string());
    assert_eq!(
        err.to_string(),
        "Authentication failed: invalid credentials"
    );

    let err = NntpError::NoSuchGroup("alt.test".to_string());
    assert_eq!(err.to_string(), "No such newsgroup: alt.test");

    let err = NntpError::NoSuchArticle("<123@example>".to_string());
    assert_eq!(err.to_string(), "No such article: <123@example>");

    let err = NntpError::Protocol {
        code: 411,
        message: "No such group".to_string(),
    };
    assert_eq!(err.to_string(), "NNTP error 411: No such group");
}

#[cfg(feature = "serde")]
#[test]
fn test_server_config_serde() {
    let config = ServerConfig::tls("news.example.com", "user", "pass");

    // Serialize
    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("news.example.com"));
    assert!(json.contains("\"port\":563"));
    assert!(json.contains("\"tls\":true"));

    // Deserialize
    let deserialized: ServerConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.host, config.host);
    assert_eq!(deserialized.port, config.port);
    assert_eq!(deserialized.tls, config.tls);
}

#[cfg(feature = "serde")]
#[test]
fn test_server_config_serde_defaults() {
    // Test that tls defaults to true when deserializing
    let json = r#"{"host":"news.example.com","port":563,"username":"user","password":"pass"}"#;
    let config: ServerConfig = serde_json::from_str(json).unwrap();
    assert!(config.tls); // Should default to true
}

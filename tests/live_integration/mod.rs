//! Live integration tests against a real NNTP server
//!
//! These tests are organized into modules:
//! - rfc_commands: RFC 3977/4643/8054/4642 command testing
//! - binary_yenc: yEnc download and verification
//! - nzb: NZB parsing and download
//! - par2: PAR2 verification
//! - benchmarks: Performance measurements
//!
//! Run with:
//! ```
//! cargo test --features live-tests -- --test-threads=1
//! ```
//!
//! Required environment variables:
//! - NNTP_HOST: NNTP server hostname
//! - NNTP_PORT: NNTP server port (default: 563)
//! - NNTP_USER: Username
//! - NNTP_PASS: Password
//! - NNTP_GROUP: Test newsgroup (default: alt.test)
//! - NNTP_BINARY_GROUP: Binary test newsgroup (default: alt.binaries.test)

#![cfg(feature = "live-tests")]

use nntp_rs::ServerConfig;
use std::sync::Arc;

/// Get server configuration from environment variables
pub fn get_test_config() -> ServerConfig {
    let host = std::env::var("NNTP_HOST").expect("NNTP_HOST not set");
    let port = std::env::var("NNTP_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(563);
    let username = std::env::var("NNTP_USER").expect("NNTP_USER not set");
    let password = std::env::var("NNTP_PASS").expect("NNTP_PASS not set");

    ServerConfig {
        host,
        port,
        tls: true,
        allow_insecure_tls: false,
        username,
        password,
    }
}

/// Get Arc-wrapped server configuration
pub fn get_test_config_arc() -> Arc<ServerConfig> {
    Arc::new(get_test_config())
}

/// Get test newsgroup for text posts (default: alt.test)
pub fn get_test_group() -> String {
    std::env::var("NNTP_GROUP").unwrap_or_else(|_| "alt.test".to_string())
}

/// Get test newsgroup for binary posts (default: alt.binaries.test)
pub fn get_binary_test_group() -> String {
    std::env::var("NNTP_BINARY_GROUP").unwrap_or_else(|_| "alt.binaries.test".to_string())
}

// Module declarations
pub mod benchmarks;
pub mod binary_yenc;
pub mod nzb;
pub mod par2;
pub mod rfc_commands;

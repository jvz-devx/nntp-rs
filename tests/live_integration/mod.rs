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
///
/// Supports both naming conventions with fallback:
/// - `NNTP_USER` or `NNTP_USERNAME`
/// - `NNTP_PASS` or `NNTP_PASSWORD`
/// - `NNTP_PORT` or `NNTP_PORT_SSL` (default: 563)
pub fn get_test_config() -> ServerConfig {
    let host = std::env::var("NNTP_HOST").expect("NNTP_HOST not set");
    let port = std::env::var("NNTP_PORT")
        .or_else(|_| std::env::var("NNTP_PORT_SSL"))
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(563);
    let username = std::env::var("NNTP_USER")
        .or_else(|_| std::env::var("NNTP_USERNAME"))
        .expect("NNTP_USER or NNTP_USERNAME not set");
    let password = std::env::var("NNTP_PASS")
        .or_else(|_| std::env::var("NNTP_PASSWORD"))
        .expect("NNTP_PASS or NNTP_PASSWORD not set");

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
///
/// This test utility is kept for API completeness to support tests that need
/// Arc-wrapped configurations (e.g., for connection pools or shared state).
/// Currently unused but provides a convenient helper for future integration tests.
#[allow(dead_code)] // Kept for test utility completeness
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
pub mod high_throughput;
pub mod listing_extended;
pub mod nzb;
pub mod par2;
pub mod pool;
pub mod rfc_commands;

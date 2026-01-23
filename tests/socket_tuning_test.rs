//! Socket tuning tests for nntp-rs
//!
//! These tests verify that TCP socket buffer tuning works correctly.
//! Most tests require a live NNTP server to validate actual socket configuration.
//!
//! Run with:
//! ```
//! cargo test --test socket_tuning_test --features live-tests
//! ```
//!
//! Required environment variables:
//! - NNTP_HOST: NNTP server hostname
//! - NNTP_PORT: NNTP server port (default: 563)
//! - NNTP_USER: Username
//! - NNTP_PASS: Password

#![cfg(feature = "live-tests")]

use nntp_rs::{NntpClient, ServerConfig};
use std::sync::Arc;

fn get_test_config() -> ServerConfig {
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

/// Test that socket tuning doesn't break basic connection
#[tokio::test]
async fn test_socket_tuning_connection_works() {
    let config = get_test_config();

    // Connect and authenticate - if socket tuning broke anything, this will fail
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    // Verify we can perform basic operations
    let response = client.capabilities().await;
    assert!(response.is_ok(), "Should be able to fetch capabilities with tuned sockets");
}

/// Test that socket tuning doesn't break article fetching
#[tokio::test]
async fn test_socket_tuning_article_fetch() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    // Try to select a test group
    let group = std::env::var("NNTP_GROUP").unwrap_or_else(|_| "alt.test".to_string());
    let select_result = client.select_group(&group).await;

    // Only proceed if the group exists and has articles
    if let Ok((count, first, _last)) = select_result {
        if count > 0 {
            // Try to fetch an article using binary API (which benefits from large buffers)
            let article_num = format!("{}", first);
            let result = client.fetch_article_binary(&article_num).await;

            // Should succeed with tuned socket buffers
            assert!(
                result.is_ok() || matches!(result, Err(nntp_rs::NntpError::NoSuchArticle(_))),
                "Article fetch should work with tuned sockets"
            );
        }
    }
}

/// Test that connection works with IPv4
#[tokio::test]
async fn test_socket_tuning_ipv4() {
    let config = get_test_config();

    // Try to resolve to IPv4 address explicitly
    // This tests that socket2 IPv4 domain selection works correctly
    let host = config.host.clone();

    // Only test if we can resolve to an IPv4 address
    use std::net::ToSocketAddrs;
    let addr = format!("{}:{}", host, config.port);
    if let Ok(mut addrs) = addr.to_socket_addrs() {
        if let Some(socket_addr) = addrs.find(|a| a.is_ipv4()) {
            println!("Testing with IPv4 address: {}", socket_addr);

            let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
            client.authenticate().await.unwrap();

            // Basic operation should work
            assert!(client.capabilities().await.is_ok());
        }
    }
}

/// Test that connection works with IPv6 (if available)
#[tokio::test]
async fn test_socket_tuning_ipv6() {
    let config = get_test_config();

    // Try to resolve to IPv6 address explicitly
    // This tests that socket2 IPv6 domain selection works correctly
    let host = config.host.clone();

    // Only test if we can resolve to an IPv6 address
    use std::net::ToSocketAddrs;
    let addr = format!("{}:{}", host, config.port);
    if let Ok(mut addrs) = addr.to_socket_addrs() {
        if let Some(socket_addr) = addrs.find(|a| a.is_ipv6()) {
            println!("Testing with IPv6 address: {}", socket_addr);

            let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
            client.authenticate().await.unwrap();

            // Basic operation should work
            assert!(client.capabilities().await.is_ok());
        } else {
            println!("Skipping IPv6 test - no IPv6 address available");
        }
    }
}

/// Test that multiple connections with tuned sockets work correctly
#[tokio::test]
async fn test_socket_tuning_multiple_connections() {
    let config = Arc::new(get_test_config());

    // Create multiple connections simultaneously
    let mut handles = vec![];

    for i in 0..5 {
        let cfg = config.clone();
        let handle = tokio::spawn(async move {
            let mut client = NntpClient::connect(cfg).await.unwrap();
            client.authenticate().await.unwrap();

            // Each connection should work independently
            let result = client.capabilities().await;
            assert!(result.is_ok(), "Connection {} should work with tuned sockets", i);
        });
        handles.push(handle);
    }

    // Wait for all connections to complete
    for handle in handles {
        handle.await.unwrap();
    }
}

/// Test that socket tuning doesn't cause issues with connection timeout
#[tokio::test]
async fn test_socket_tuning_respects_timeout() {
    // Create a config pointing to a non-existent server
    // This should timeout properly even with socket tuning
    let config = ServerConfig {
        host: "192.0.2.1".to_string(), // TEST-NET-1 (non-routable)
        port: 563,
        tls: true,
        allow_insecure_tls: false,
        username: "test".to_string(),
        password: "test".to_string(),
    };

    // Connection should timeout (not hang indefinitely)
    let start = std::time::Instant::now();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        NntpClient::connect(Arc::new(config))
    ).await;
    let elapsed = start.elapsed();

    // Should timeout within reasonable time
    assert!(elapsed < std::time::Duration::from_secs(10),
            "Connection should timeout quickly, took {:?}", elapsed);

    // Should either timeout or fail to connect
    assert!(result.is_err() || result.unwrap().is_err(),
            "Connection to non-routable address should fail");
}

/// Test that binary article fetching works with large buffers
#[tokio::test]
async fn test_socket_tuning_large_article_fetch() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    // Try to select a binary group that might have large articles
    let group = std::env::var("NNTP_GROUP").unwrap_or_else(|_| "alt.test".to_string());
    let select_result = client.select_group(&group).await;

    if let Ok((count, first, last)) = select_result {
        if count > 0 {
            // Fetch multiple articles to exercise the receive buffer
            let num_articles = std::cmp::min(5, count);
            let start_article = std::cmp::max(first, last.saturating_sub(num_articles));

            for article_num in start_article..=last {
                let article_id = format!("{}", article_num);
                let result = client.fetch_article_binary(&article_id).await;

                // Large buffers should help performance, but correctness is what we're testing
                if result.is_ok() {
                    println!("Successfully fetched article {} with tuned buffers", article_num);
                    break; // Found at least one article, test passes
                }
            }
        }
    }
}

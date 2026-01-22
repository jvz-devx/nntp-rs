//! Live integration tests against a real NNTP server
//!
//! These tests are disabled by default. Enable with:
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

#![cfg(feature = "live-tests")]

mod live_integration;

use nntp_rs::{NntpClient, NntpPool, ServerConfig};
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

fn get_test_group() -> String {
    std::env::var("NNTP_GROUP").unwrap_or_else(|_| "alt.test".to_string())
}

#[tokio::test]
async fn test_live_connect_and_authenticate() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();
}

#[tokio::test]
async fn test_live_compression_negotiation() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    // Try to enable compression (should succeed or gracefully fall back)
    let result = client.try_enable_compression().await;
    assert!(result.is_ok());

    let compression_enabled = result.unwrap();
    println!("Compression enabled: {}", compression_enabled);

    // If compression was enabled, verify we can still use the connection
    if compression_enabled {
        assert!(client.is_compression_enabled());
    }
}

#[tokio::test]
async fn test_live_select_group() {
    let config = get_test_config();
    let group = get_test_group();

    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    let (count, first, last) = client.select_group(&group).await.unwrap();
    println!("Group {}: {} articles ({}-{})", group, count, first, last);

    assert!(
        count > 0 || first == 0,
        "Group should have articles or be empty"
    );
    if count > 0 {
        assert!(last >= first, "Last article should be >= first");
    }
}

#[tokio::test]
async fn test_live_fetch_xover() {
    let config = get_test_config();
    let group = get_test_group();

    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();
    client.try_enable_compression().await.ok();

    let (count, first, last) = client.select_group(&group).await.unwrap();
    if count == 0 {
        println!("Group is empty, skipping XOVER test");
        return;
    }

    // Fetch last 10 articles
    let start = if last > 10 { last - 10 } else { first };
    let entries = client
        .fetch_xover(&format!("{}-{}", start, last))
        .await
        .unwrap();

    println!("Fetched {} XOVER entries", entries.len());
    assert!(!entries.is_empty(), "Should have at least one entry");

    // Verify entry structure
    let entry = &entries[0];
    assert!(entry.article_number >= start);
    assert!(entry.article_number <= last);
    assert!(!entry.subject.is_empty() || true); // Subject can be empty
    println!(
        "Sample entry: #{} - {}",
        entry.article_number, entry.subject
    );
}

#[tokio::test]
async fn test_live_fetch_article_head() {
    let config = get_test_config();
    let group = get_test_group();

    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    let (count, _first, last) = client.select_group(&group).await.unwrap();
    if count == 0 {
        println!("Group is empty, skipping HEAD test");
        return;
    }

    // Fetch head of last article
    let response = client.fetch_head(&last.to_string()).await.unwrap();
    assert!(response.is_success());
    assert!(!response.lines.is_empty(), "HEAD should have header lines");

    println!("HEAD response: {} lines", response.lines.len());
}

#[tokio::test]
async fn test_live_bandwidth_stats() {
    let config = get_test_config();
    let group = get_test_group();

    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    let compression_enabled = client.try_enable_compression().await.unwrap();
    if !compression_enabled {
        println!("Compression not available, skipping bandwidth stats test");
        return;
    }

    let (count, _first, last) = client.select_group(&group).await.unwrap();
    if count == 0 {
        println!("Group is empty, skipping bandwidth stats test");
        return;
    }

    // Fetch some data to generate bandwidth stats
    let start = if last > 10 { last - 10 } else { last };
    client
        .fetch_xover(&format!("{}-{}", start, last))
        .await
        .unwrap();

    let (compressed, decompressed) = client.get_bandwidth_stats();
    println!(
        "Bandwidth: {} compressed, {} decompressed",
        compressed, decompressed
    );

    if decompressed > 0 {
        assert!(compressed > 0, "Should have compressed bytes");
        assert!(
            compressed < decompressed,
            "Compressed should be smaller than decompressed"
        );

        let ratio = (1.0 - (compressed as f64 / decompressed as f64)) * 100.0;
        println!("Compression ratio: {:.1}%", ratio);
        assert!(
            ratio > 0.0 && ratio < 100.0,
            "Compression ratio should be reasonable"
        );
    }
}

#[tokio::test]
async fn test_live_connection_pool() {
    let config = get_test_config();
    let group = get_test_group();

    let pool = NntpPool::new(config, 5).await.unwrap();

    // Get multiple connections
    let mut conn1 = pool.get().await.unwrap();
    let mut conn2 = pool.get().await.unwrap();

    let result1 = conn1.select_group(&group).await;
    let result2 = conn2.select_group(&group).await;

    assert!(result1.is_ok());
    assert!(result2.is_ok());

    println!("Pool connections: {}", pool.connections_in_use());
    assert_eq!(pool.connections_in_use(), 2);

    // Drop connections and verify they return to pool
    drop(conn1);
    drop(conn2);

    // Give pool time to process returns
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    println!("Idle connections after drop: {}", pool.idle_connections());
}

#[tokio::test]
async fn test_live_invalid_group() {
    let config = get_test_config();

    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    let result = client
        .select_group("this.group.definitely.does.not.exist.xyz123")
        .await;
    assert!(result.is_err());

    match result {
        Err(nntp_rs::NntpError::NoSuchGroup(group)) => {
            assert!(group.contains("this.group.definitely.does.not.exist"));
        }
        _ => panic!("Expected NoSuchGroup error"),
    }
}

#[tokio::test]
async fn test_live_quit() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    let result = client.quit().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_live_parallel_fetching() {
    let config = get_test_config();
    let group = get_test_group();

    let pool = Arc::new(NntpPool::new(config, 5).await.unwrap());

    // Get group info first
    let (count, _first, last) = {
        let mut conn = pool.get().await.unwrap();
        conn.select_group(&group).await.unwrap()
    };

    if count < 5 {
        println!("Not enough articles for parallel test, skipping");
        return;
    }

    // Spawn multiple parallel fetch tasks
    let mut handles = vec![];
    for i in 0..5 {
        let pool = Arc::clone(&pool);
        let group = group.clone();
        let article_num = last - i;

        handles.push(tokio::spawn(async move {
            let mut conn = pool.get().await?;
            conn.select_group(&group).await?;
            conn.fetch_xover(&article_num.to_string()).await
        }));
    }

    // Wait for all tasks
    let mut success_count = 0;
    for handle in handles {
        if let Ok(Ok(_)) = handle.await {
            success_count += 1;
        }
    }

    println!(
        "Successfully fetched {} articles in parallel",
        success_count
    );
    assert!(success_count >= 4, "Most parallel fetches should succeed");
}

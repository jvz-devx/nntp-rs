//! Connection Pool Live Integration Tests
//!
//! Tests for `NntpPool` creation, connection lifecycle, state tracking,
//! and automatic auth/compression negotiation.

#![cfg(feature = "live-tests")]

use nntp_rs::{NntpPool, RetryConfig};

use super::{get_test_config, get_test_group};

#[tokio::test]
async fn test_pool_create() {
    let config = get_test_config();
    let pool = NntpPool::new(config, 2).await.unwrap();

    let state = pool.state();
    println!(
        "Pool state: connections={}, idle={}",
        state.connections, state.idle_connections
    );

    // Pool should have created at least one connection
    assert!(state.connections > 0);
}

#[tokio::test]
async fn test_pool_get_connection() {
    let config = get_test_config();
    let pool = NntpPool::new(config, 2).await.unwrap();

    let mut conn = pool.get().await.unwrap();

    // Connection should already be authenticated and usable
    let group = get_test_group();
    let info = conn.select_group(&group).await.unwrap();
    println!(
        "Pool connection selected group {}: count={}, first={}, last={}",
        group, info.count, info.first, info.last
    );

    assert!(info.last >= info.first);
}

#[tokio::test]
async fn test_pool_connection_reuse() {
    let config = get_test_config();
    let pool = NntpPool::new(config, 2).await.unwrap();

    // Get a connection, use it, drop it
    {
        let mut conn = pool.get().await.unwrap();
        let group = get_test_group();
        conn.select_group(&group).await.unwrap();
    }

    // Get another connection — pool should recycle
    let mut conn = pool.get().await.unwrap();
    let group = get_test_group();
    let info = conn.select_group(&group).await.unwrap();
    println!(
        "Reused connection selected group {}: count={}",
        group, info.count
    );

    assert!(info.last >= info.first);
}

#[tokio::test]
async fn test_pool_state_tracking() {
    let config = get_test_config();
    let pool = NntpPool::new(config, 2).await.unwrap();

    let idle_before = pool.idle_connections();

    // Borrow a connection
    let conn = pool.get().await.unwrap();
    let in_use = pool.connections_in_use();
    println!(
        "With 1 borrowed: in_use={}, idle={}",
        in_use,
        pool.idle_connections()
    );

    assert!(in_use >= 1);

    // Drop the connection back to the pool
    drop(conn);

    // Give the pool a moment to register the return
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let idle_after = pool.idle_connections();
    println!(
        "After return: idle_before={}, idle_after={}",
        idle_before, idle_after
    );

    assert!(idle_after >= 1);
}

#[tokio::test]
async fn test_pool_with_retry_config() {
    let config = get_test_config();
    let pool = NntpPool::with_retry_config(config, 2, RetryConfig::no_retry())
        .await
        .unwrap();

    let mut conn = pool.get().await.unwrap();
    let group = get_test_group();
    let info = conn.select_group(&group).await.unwrap();
    println!(
        "No-retry pool connection: group={}, count={}",
        group, info.count
    );

    assert!(info.last >= info.first);
}

#[tokio::test]
async fn test_pool_auto_auth_and_compress() {
    let config = get_test_config();
    let pool = NntpPool::new(config, 2).await.unwrap();

    let conn = pool.get().await.unwrap();

    // Pool connections are auto-authenticated during creation
    assert!(conn.is_authenticated());
    println!(
        "Pool connection: authenticated={}, compression={}",
        conn.is_authenticated(),
        conn.is_compression_enabled()
    );

    // Compression depends on server support — just log the result
    if conn.is_compression_enabled() {
        println!("Pool connection has compression enabled");
    } else {
        println!("Pool connection has no compression (server may not support it)");
    }
}

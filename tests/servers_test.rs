//! Multi-server support tests
//!
//! Tests for ServerGroup functionality including failover, load balancing,
//! and statistics tracking.

use nntp_rs::{FailoverStrategy, ServerConfig, ServerGroup};

#[tokio::test]
async fn test_server_group_creation() {
    let configs = vec![
        ServerConfig::new("news.example.com", 119, false, "user", "pass"),
        ServerConfig::new("news2.example.com", 119, false, "user", "pass"),
    ];

    let priorities = vec![100, 50];

    let result = ServerGroup::new(
        configs,
        priorities,
        FailoverStrategy::PrimaryWithFallback,
        5,
    )
    .await;

    // Pool creation succeeds even if servers aren't reachable
    // Connection is only tested when get() is called
    assert!(result.is_ok());

    let group = result.unwrap();
    assert_eq!(group.server_count(), 2);
    assert_eq!(group.server_ids().len(), 2);
}

#[tokio::test]
async fn test_server_group_mismatched_priorities() {
    let configs = vec![
        ServerConfig::new("news.example.com", 119, false, "user", "pass"),
        ServerConfig::new("news2.example.com", 119, false, "user", "pass"),
    ];

    let priorities = vec![100]; // Only one priority for two configs

    let result = ServerGroup::new(
        configs,
        priorities,
        FailoverStrategy::PrimaryWithFallback,
        5,
    )
    .await;

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Priorities count must match"));
}

#[tokio::test]
async fn test_server_group_empty_configs() {
    let configs = vec![];
    let priorities = vec![];

    let result = ServerGroup::new(
        configs,
        priorities,
        FailoverStrategy::PrimaryWithFallback,
        5,
    )
    .await;

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("At least one server"));
}

#[test]
fn test_failover_strategy_variants() {
    // Test that all strategy variants work
    let strategies = [
        FailoverStrategy::PrimaryWithFallback,
        FailoverStrategy::RoundRobin,
        FailoverStrategy::RoundRobinHealthy,
    ];

    assert_eq!(strategies.len(), 3);

    // Test Copy
    let copied = strategies[0];
    assert_eq!(copied, FailoverStrategy::PrimaryWithFallback);

    // Test Debug
    let debug_str = format!("{:?}", strategies[0]);
    assert!(debug_str.contains("PrimaryWithFallback"));
}

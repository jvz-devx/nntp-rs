//! Multi-server support for NNTP clients
//!
//! This module provides functionality for managing multiple NNTP servers with
//! automatic failover and load balancing. Supports priority-based server selection,
//! per-server statistics tracking, and intelligent failover on connection errors.
//!
//! # Architecture
//!
//! - `ServerGroup`: Manages multiple NNTP connection pools with failover
//! - `FailoverStrategy`: Defines how servers are selected
//! - `ServerStats`: Tracks per-server performance metrics
//! - `GroupStats`: Aggregates statistics across all servers
//!
//! # Example
//!
//! ```no_run
//! use nntp_rs::{ServerConfig, ServerGroup, FailoverStrategy};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let configs = vec![
//!     ServerConfig::tls("news-primary.com", "user", "pass"),
//!     ServerConfig::tls("news-backup.com", "user", "pass"),
//! ];
//!
//! let priorities = vec![100, 50]; // Primary has higher priority
//!
//! let group = ServerGroup::new(
//!     configs,
//!     priorities,
//!     FailoverStrategy::PrimaryWithFallback,
//!     5, // Max connections per server
//! ).await?;
//!
//! // Get connection - automatically handles failover
//! let mut conn = group.get_connection().await?;
//! # Ok(())
//! # }
//! ```

use crate::pool::NntpConnectionManager;
use crate::{NntpError, NntpPool, Result, ServerConfig};
use bb8::PooledConnection;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Instant;

/// Per-server performance statistics
///
/// Tracks success/failure metrics for individual servers to enable
/// intelligent failover decisions and health monitoring.
#[derive(Debug, Clone)]
pub struct ServerStats {
    /// Unique server identifier (host:port)
    pub server_id: String,
    /// Total number of connection requests
    pub total_requests: u64,
    /// Number of successful requests
    pub successful_requests: u64,
    /// Number of failed requests (connection errors, timeouts)
    pub failed_requests: u64,
    /// Number of 430 (not found) responses
    pub not_found_requests: u64,
    /// Total bytes downloaded from this server
    pub total_bytes_downloaded: u64,
    /// Time of last successful request
    pub last_success_time: Option<Instant>,
    /// Time of last failed request
    pub last_failure_time: Option<Instant>,
    /// Consecutive failures (reset on success)
    pub consecutive_failures: u32,
}

impl ServerStats {
    /// Create new statistics tracker for a server
    pub fn new(server_id: String) -> Self {
        Self {
            server_id,
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            not_found_requests: 0,
            total_bytes_downloaded: 0,
            last_success_time: None,
            last_failure_time: None,
            consecutive_failures: 0,
        }
    }

    /// Record a successful request
    pub fn record_success(&mut self, bytes: u64) {
        self.total_requests += 1;
        self.successful_requests += 1;
        self.total_bytes_downloaded += bytes;
        self.last_success_time = Some(Instant::now());
        self.consecutive_failures = 0;
    }

    /// Record a failed request
    pub fn record_failure(&mut self) {
        self.total_requests += 1;
        self.failed_requests += 1;
        self.last_failure_time = Some(Instant::now());
        self.consecutive_failures += 1;
    }

    /// Record a 430 (article not found) response
    pub fn record_not_found(&mut self) {
        self.total_requests += 1;
        self.not_found_requests += 1;
        // Not counted as failure - article simply doesn't exist
    }

    /// Calculate availability score (0.0 to 1.0)
    ///
    /// Returns the ratio of successful requests to total requests.
    /// Returns 1.0 if no requests have been made yet.
    #[must_use]
    pub fn availability_score(&self) -> f64 {
        if self.total_requests == 0 {
            1.0
        } else {
            self.successful_requests as f64 / self.total_requests as f64
        }
    }

    /// Check if server is degraded
    ///
    /// Returns true if availability is below the threshold or there are
    /// too many consecutive failures.
    pub fn is_degraded(&self, threshold: f64, max_consecutive_failures: u32) -> bool {
        self.availability_score() < threshold
            || self.consecutive_failures >= max_consecutive_failures
    }
}

/// Thread-safe wrapper for ServerStats
#[derive(Debug, Clone)]
struct AtomicServerStats {
    server_id: String,
    total_requests: Arc<AtomicU64>,
    successful_requests: Arc<AtomicU64>,
    failed_requests: Arc<AtomicU64>,
    not_found_requests: Arc<AtomicU64>,
    total_bytes_downloaded: Arc<AtomicU64>,
    last_success_time: Arc<Mutex<Option<Instant>>>,
    last_failure_time: Arc<Mutex<Option<Instant>>>,
    consecutive_failures: Arc<AtomicU32>,
}

impl AtomicServerStats {
    fn new(server_id: String) -> Self {
        Self {
            server_id,
            total_requests: Arc::new(AtomicU64::new(0)),
            successful_requests: Arc::new(AtomicU64::new(0)),
            failed_requests: Arc::new(AtomicU64::new(0)),
            not_found_requests: Arc::new(AtomicU64::new(0)),
            total_bytes_downloaded: Arc::new(AtomicU64::new(0)),
            last_success_time: Arc::new(Mutex::new(None)),
            last_failure_time: Arc::new(Mutex::new(None)),
            consecutive_failures: Arc::new(AtomicU32::new(0)),
        }
    }

    fn record_success(&self, bytes: u64) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.successful_requests.fetch_add(1, Ordering::Relaxed);
        self.total_bytes_downloaded
            .fetch_add(bytes, Ordering::Relaxed);
        *self
            .last_success_time
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(Instant::now());
        self.consecutive_failures.store(0, Ordering::Relaxed);
    }

    fn record_failure(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.failed_requests.fetch_add(1, Ordering::Relaxed);
        *self
            .last_failure_time
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(Instant::now());
        self.consecutive_failures.fetch_add(1, Ordering::Relaxed);
    }

    fn record_not_found(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.not_found_requests.fetch_add(1, Ordering::Relaxed);
    }

    fn snapshot(&self) -> ServerStats {
        ServerStats {
            server_id: self.server_id.clone(),
            total_requests: self.total_requests.load(Ordering::Relaxed),
            successful_requests: self.successful_requests.load(Ordering::Relaxed),
            failed_requests: self.failed_requests.load(Ordering::Relaxed),
            not_found_requests: self.not_found_requests.load(Ordering::Relaxed),
            total_bytes_downloaded: self.total_bytes_downloaded.load(Ordering::Relaxed),
            last_success_time: *self
                .last_success_time
                .lock()
                .unwrap_or_else(|e| e.into_inner()),
            last_failure_time: *self
                .last_failure_time
                .lock()
                .unwrap_or_else(|e| e.into_inner()),
            consecutive_failures: self.consecutive_failures.load(Ordering::Relaxed),
        }
    }
}

/// Strategy for selecting servers from a group
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailoverStrategy {
    /// Always try primary first, fall back on error
    PrimaryWithFallback,

    /// Round-robin through all servers
    RoundRobin,

    /// Round-robin only through healthy servers
    RoundRobinHealthy,
}

/// Aggregate statistics across all servers in a group
#[derive(Debug, Clone)]
pub struct GroupStats {
    /// Total requests across all servers
    pub total_requests: u64,
    /// Total 430 (not found) responses across all servers
    pub total_not_found: u64,
    /// Number of times failover occurred
    pub failover_count: u64,
    /// Per-server statistics
    pub per_server_stats: HashMap<String, ServerStats>,
}

/// Server entry with priority and connection pool
#[derive(Debug)]
struct ServerEntry {
    /// Server identifier
    id: String,
    /// Server configuration
    ///
    /// Currently unused but retained for future reconnection features:
    /// - Manual server pool refresh/reconnection API
    /// - Dynamic credential rotation
    /// - Runtime configuration updates without rebuilding pools
    /// - Server info introspection for monitoring/debugging
    ///
    /// Note: Current failover is handled by ServerGroup's health tracking,
    /// and connection pools manage their own reconnection via NntpConnectionManager.
    ///
    /// Intentionally unused (RFC completeness): This field is reserved for future
    /// API enhancements without breaking changes to the struct layout.
    #[expect(dead_code)]
    config: ServerConfig,
    /// Priority (higher = preferred)
    priority: u32,
    /// Connection pool
    pool: NntpPool,
    /// Statistics tracker
    stats: AtomicServerStats,
}

/// Manages multiple NNTP servers with automatic failover
///
/// `ServerGroup` coordinates multiple NNTP server connection pools,
/// providing automatic failover on connection errors and load balancing
/// based on configurable strategies.
///
/// # Failover Behavior
///
/// - **Connection errors**: Automatically try next server
/// - **430 (Not Found)**: Do NOT failover (article doesn't exist)
/// - **Other 4xx/5xx**: Recorded but connection stays valid
///
/// # Example
///
/// ```no_run
/// use nntp_rs::{ServerConfig, ServerGroup, FailoverStrategy};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let group = ServerGroup::new(
///     vec![
///         ServerConfig::tls("news1.example.com", "user", "pass"),
///         ServerConfig::tls("news2.example.com", "user", "pass"),
///     ],
///     vec![100, 50], // Priorities
///     FailoverStrategy::PrimaryWithFallback,
///     10, // Max connections per server
/// ).await?;
///
/// let mut conn = group.get_connection().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct ServerGroup {
    servers: Vec<ServerEntry>,
    strategy: FailoverStrategy,
    round_robin_index: Arc<Mutex<usize>>,
    failover_count: Arc<AtomicU64>,
    degraded_threshold: f64,
    max_consecutive_failures: u32,
}

impl ServerGroup {
    /// Create a new server group
    ///
    /// # Arguments
    ///
    /// * `configs` - Server configurations
    /// * `priorities` - Priority values (higher = preferred), must match config count
    /// * `strategy` - Server selection strategy
    /// * `max_pool_size` - Maximum connections per server pool
    ///
    /// # Errors
    ///
    /// Returns error if priorities count doesn't match configs count or if
    /// pool creation fails.
    pub async fn new(
        configs: Vec<ServerConfig>,
        priorities: Vec<u32>,
        strategy: FailoverStrategy,
        max_pool_size: u32,
    ) -> Result<Self> {
        if configs.len() != priorities.len() {
            return Err(NntpError::InvalidResponse(
                "Priorities count must match configs count".to_string(),
            ));
        }

        if configs.is_empty() {
            return Err(NntpError::InvalidResponse(
                "At least one server configuration required".to_string(),
            ));
        }

        let mut servers = Vec::new();
        for (config, priority) in configs.into_iter().zip(priorities.into_iter()) {
            let server_id = format!("{}:{}", config.host, config.port);
            let pool = NntpPool::new(config.clone(), max_pool_size).await?;
            servers.push(ServerEntry {
                id: server_id.clone(),
                config,
                priority,
                pool,
                stats: AtomicServerStats::new(server_id),
            });
        }

        // Sort by priority (descending)
        servers.sort_by(|a, b| b.priority.cmp(&a.priority));

        Ok(Self {
            servers,
            strategy,
            round_robin_index: Arc::new(Mutex::new(0)),
            failover_count: Arc::new(AtomicU64::new(0)),
            degraded_threshold: 0.95,
            max_consecutive_failures: 5,
        })
    }

    /// Get a connection from the server group
    ///
    /// Automatically selects a server based on the configured strategy and
    /// handles failover on connection errors.
    ///
    /// # Errors
    ///
    /// Returns error if all servers are unavailable.
    pub async fn get_connection(&self) -> Result<PooledConnection<'_, NntpConnectionManager>> {
        let server_order = self.get_server_order();

        let mut last_error = None;
        for server_idx in server_order {
            let server = &self.servers[server_idx];

            match server.pool.get().await {
                Ok(conn) => {
                    server.stats.record_success(0);
                    return Ok(conn);
                }
                Err(e) => {
                    server.stats.record_failure();
                    last_error = Some(e);
                    self.failover_count.fetch_add(1, Ordering::Relaxed);
                    // Try next server
                }
            }
        }

        // All servers failed
        Err(last_error
            .unwrap_or_else(|| NntpError::InvalidResponse("No servers available".to_string())))
    }

    /// Get a connection from a specific server by ID
    ///
    /// # Arguments
    ///
    /// * `server_id` - Server identifier (host:port format)
    ///
    /// # Errors
    ///
    /// Returns error if server not found or connection fails.
    pub async fn get_connection_from(
        &self,
        server_id: &str,
    ) -> Result<PooledConnection<'_, NntpConnectionManager>> {
        let server = self
            .servers
            .iter()
            .find(|s| s.id == server_id)
            .ok_or_else(|| {
                NntpError::InvalidResponse(format!("Server not found: {}", server_id))
            })?;

        match server.pool.get().await {
            Ok(conn) => {
                server.stats.record_success(0);
                Ok(conn)
            }
            Err(e) => {
                server.stats.record_failure();
                Err(e)
            }
        }
    }

    /// Record that a 430 (not found) error occurred on a server
    ///
    /// This updates statistics but does NOT trigger failover, as 430 errors
    /// indicate the article doesn't exist (not a server problem).
    pub fn record_not_found(&self, server_id: &str) {
        if let Some(server) = self.servers.iter().find(|s| s.id == server_id) {
            server.stats.record_not_found();
        }
    }

    /// Record successful data transfer for statistics
    pub fn record_success(&self, server_id: &str, bytes: u64) {
        if let Some(server) = self.servers.iter().find(|s| s.id == server_id) {
            server.stats.record_success(bytes);
        }
    }

    /// Get aggregate statistics for the server group
    pub fn stats(&self) -> GroupStats {
        let mut per_server_stats = HashMap::new();
        let mut total_requests = 0;
        let mut total_not_found = 0;

        for server in &self.servers {
            let stats = server.stats.snapshot();
            total_requests += stats.total_requests;
            total_not_found += stats.not_found_requests;
            per_server_stats.insert(server.id.clone(), stats);
        }

        GroupStats {
            total_requests,
            total_not_found,
            failover_count: self.failover_count.load(Ordering::Relaxed),
            per_server_stats,
        }
    }

    /// Get statistics for a specific server
    pub fn server_stats(&self, server_id: &str) -> Option<ServerStats> {
        self.servers
            .iter()
            .find(|s| s.id == server_id)
            .map(|s| s.stats.snapshot())
    }

    /// Get list of server IDs in priority order
    pub fn server_ids(&self) -> Vec<String> {
        self.servers.iter().map(|s| s.id.clone()).collect()
    }

    /// Get number of servers in the group
    pub fn server_count(&self) -> usize {
        self.servers.len()
    }

    /// Get the server selection order based on strategy
    fn get_server_order(&self) -> Vec<usize> {
        match self.strategy {
            FailoverStrategy::PrimaryWithFallback => {
                // Try in priority order
                (0..self.servers.len()).collect()
            }
            FailoverStrategy::RoundRobin => {
                // Rotate through all servers
                let mut index = self
                    .round_robin_index
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                let start = *index;
                *index = (*index + 1) % self.servers.len();

                (0..self.servers.len())
                    .map(|i| (start + i) % self.servers.len())
                    .collect()
            }
            FailoverStrategy::RoundRobinHealthy => {
                // Only use healthy servers
                let healthy: Vec<usize> = self
                    .servers
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| {
                        let stats = s.stats.snapshot();
                        !stats.is_degraded(self.degraded_threshold, self.max_consecutive_failures)
                    })
                    .map(|(i, _)| i)
                    .collect();

                if healthy.is_empty() {
                    // No healthy servers, fall back to all
                    (0..self.servers.len()).collect()
                } else {
                    let mut index = self
                        .round_robin_index
                        .lock()
                        .unwrap_or_else(|e| e.into_inner());
                    let start = *index % healthy.len();
                    *index = (*index + 1) % healthy.len();

                    (0..healthy.len())
                        .map(|i| healthy[(start + i) % healthy.len()])
                        .collect()
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_stats_new() {
        let stats = ServerStats::new("test:119".to_string());
        assert_eq!(stats.server_id, "test:119");
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.successful_requests, 0);
        assert_eq!(stats.failed_requests, 0);
        assert_eq!(stats.not_found_requests, 0);
        assert_eq!(stats.total_bytes_downloaded, 0);
        assert_eq!(stats.consecutive_failures, 0);
        assert_eq!(stats.availability_score(), 1.0);
    }

    #[test]
    fn test_server_stats_record_success() {
        let mut stats = ServerStats::new("test:119".to_string());
        stats.record_success(1024);

        assert_eq!(stats.total_requests, 1);
        assert_eq!(stats.successful_requests, 1);
        assert_eq!(stats.total_bytes_downloaded, 1024);
        assert_eq!(stats.consecutive_failures, 0);
        assert_eq!(stats.availability_score(), 1.0);
        assert!(stats.last_success_time.is_some());
    }

    #[test]
    fn test_server_stats_record_failure() {
        let mut stats = ServerStats::new("test:119".to_string());
        stats.record_failure();

        assert_eq!(stats.total_requests, 1);
        assert_eq!(stats.failed_requests, 1);
        assert_eq!(stats.consecutive_failures, 1);
        assert_eq!(stats.availability_score(), 0.0);
        assert!(stats.last_failure_time.is_some());
    }

    #[test]
    fn test_server_stats_record_not_found() {
        let mut stats = ServerStats::new("test:119".to_string());
        stats.record_not_found();

        assert_eq!(stats.total_requests, 1);
        assert_eq!(stats.not_found_requests, 1);
        assert_eq!(stats.failed_requests, 0);
        assert_eq!(stats.consecutive_failures, 0);
    }

    #[test]
    fn test_server_stats_availability_score() {
        let mut stats = ServerStats::new("test:119".to_string());
        stats.record_success(100);
        stats.record_success(200);
        stats.record_failure();

        assert_eq!(stats.total_requests, 3);
        assert_eq!(stats.successful_requests, 2);
        assert_eq!(stats.availability_score(), 2.0 / 3.0);
    }

    #[test]
    fn test_server_stats_consecutive_failures_reset() {
        let mut stats = ServerStats::new("test:119".to_string());
        stats.record_failure();
        stats.record_failure();
        assert_eq!(stats.consecutive_failures, 2);

        stats.record_success(100);
        assert_eq!(stats.consecutive_failures, 0);
    }

    #[test]
    fn test_server_stats_is_degraded() {
        let mut stats = ServerStats::new("test:119".to_string());

        // Not degraded initially
        assert!(!stats.is_degraded(0.95, 5));

        // Degraded due to low availability
        stats.record_success(1);
        stats.record_failure();
        stats.record_failure();
        assert!(stats.is_degraded(0.95, 5));

        // Reset - need enough successes to get above 0.95
        // Current: 1 success, 2 failures (3 total, 0.333 rate)
        // Need: 95% success rate with 2 failures -> need at least 38 successes (38/40 = 0.95)
        for _ in 0..40 {
            stats.record_success(100);
        }
        // Now: 41 successes, 2 failures = 41/43 = 0.953 > 0.95
        assert!(!stats.is_degraded(0.95, 5));

        // Degraded due to consecutive failures
        for _ in 0..5 {
            stats.record_failure();
        }
        assert!(stats.is_degraded(0.95, 5));
    }

    #[test]
    fn test_atomic_server_stats() {
        let stats = AtomicServerStats::new("test:119".to_string());

        stats.record_success(1024);
        stats.record_failure();
        stats.record_not_found();

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.total_requests, 3);
        assert_eq!(snapshot.successful_requests, 1);
        assert_eq!(snapshot.failed_requests, 1);
        assert_eq!(snapshot.not_found_requests, 1);
        assert_eq!(snapshot.total_bytes_downloaded, 1024);
    }

    #[test]
    fn test_failover_strategy_equality() {
        assert_eq!(
            FailoverStrategy::PrimaryWithFallback,
            FailoverStrategy::PrimaryWithFallback
        );
        assert_ne!(
            FailoverStrategy::PrimaryWithFallback,
            FailoverStrategy::RoundRobin
        );
    }

    #[test]
    fn test_group_stats() {
        let mut per_server = HashMap::new();
        per_server.insert(
            "server1:119".to_string(),
            ServerStats::new("server1:119".to_string()),
        );

        let stats = GroupStats {
            total_requests: 10,
            total_not_found: 2,
            failover_count: 1,
            per_server_stats: per_server,
        };

        assert_eq!(stats.total_requests, 10);
        assert_eq!(stats.total_not_found, 2);
        assert_eq!(stats.failover_count, 1);
        assert_eq!(stats.per_server_stats.len(), 1);
    }
}

//! Connection pooling for NNTP clients using bb8

use crate::client::NntpClient;
use crate::config::ServerConfig;
use crate::error::{NntpError, Result};
use bb8::{Pool, PooledConnection};
use rand::Rng;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn};

/// Configuration for connection retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial backoff duration in milliseconds
    pub initial_backoff_ms: u64,
    /// Maximum backoff duration in milliseconds
    pub max_backoff_ms: u64,
    /// Backoff multiplier (exponential factor)
    pub backoff_multiplier: f64,
    /// Whether to add jitter to prevent thundering herd
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 10000,
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryConfig {
    /// Create a retry config with no retries (fail fast)
    pub fn no_retry() -> Self {
        Self {
            max_retries: 0,
            ..Default::default()
        }
    }

    /// Create a retry config with custom max retries
    pub fn with_max_retries(max_retries: u32) -> Self {
        Self {
            max_retries,
            ..Default::default()
        }
    }
}

/// Connection manager for bb8 pool
pub struct NntpConnectionManager {
    config: Arc<ServerConfig>,
}

impl NntpConnectionManager {
    /// Create a new connection manager
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }
}

#[async_trait::async_trait]
impl bb8::ManageConnection for NntpConnectionManager {
    type Connection = NntpClient;
    type Error = NntpError;

    async fn connect(&self) -> Result<Self::Connection> {
        let mut client = NntpClient::connect(self.config.clone()).await?;
        client.authenticate().await?;

        // Try to enable compression (graceful fallback if not supported)
        match client.try_enable_compression().await {
            Ok(true) => debug!("Compression enabled for new connection"),
            Ok(false) => {
                debug!("Compression not supported by server, continuing without compression")
            }
            Err(e) => debug!(
                "Compression negotiation failed: {}, continuing without compression",
                e
            ),
        }

        Ok(client)
    }

    async fn is_valid(&self, _conn: &mut Self::Connection) -> Result<()> {
        // For now, assume connection is valid
        // In the future, we could implement a STAT or similar check
        Ok(())
    }

    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        // Check if connection received invalid/corrupted data
        conn.is_broken()
    }
}

/// NNTP connection pool with retry support
///
/// Provides high-performance connection pooling with:
/// - Automatic connection creation and authentication
/// - Compression negotiation on new connections
/// - Exponential backoff with jitter on failures
/// - Broken connection detection and removal
///
/// # Example
///
/// ```no_run
/// use nntp_rs::{NntpPool, ServerConfig, RetryConfig};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = ServerConfig::tls("news.example.com", "user", "pass");
///
/// // Create pool with 10 connections and default retry config
/// let pool = NntpPool::new(config.clone(), 10).await?;
///
/// // Or with custom retry config
/// let retry_config = RetryConfig {
///     max_retries: 5,
///     initial_backoff_ms: 200,
///     ..Default::default()
/// };
/// let pool = NntpPool::with_retry_config(config, 10, retry_config).await?;
///
/// // Get connection from pool
/// let mut conn = pool.get().await?;
/// conn.select_group("alt.test").await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct NntpPool {
    pool: Pool<NntpConnectionManager>,
    retry_config: RetryConfig,
}

impl NntpPool {
    /// Create a new NNTP connection pool with default retry configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Server configuration
    /// * `max_size` - Maximum number of connections in the pool
    pub async fn new(config: ServerConfig, max_size: u32) -> Result<Self> {
        Self::with_retry_config(config, max_size, RetryConfig::default()).await
    }

    /// Create a new NNTP connection pool with custom retry configuration
    pub async fn with_retry_config(
        config: ServerConfig,
        max_size: u32,
        retry_config: RetryConfig,
    ) -> Result<Self> {
        debug!(
            "Creating NNTP connection pool for {}:{} (max size: {}, max retries: {})",
            config.host, config.port, max_size, retry_config.max_retries
        );

        let manager = NntpConnectionManager::new(config);
        let pool = Pool::builder()
            .max_size(max_size)
            // Set connection timeout to 120 seconds (allows for slow NNTP servers)
            .connection_timeout(Duration::from_secs(120))
            // Set idle connection timeout to 5 minutes
            .idle_timeout(Some(Duration::from_secs(300)))
            .build(manager)
            .await
            .map_err(|e| NntpError::Other(format!("Failed to create pool: {}", e)))?;

        Ok(Self { pool, retry_config })
    }

    /// Get a connection from the pool with automatic retry on failure
    ///
    /// Uses exponential backoff with optional jitter to prevent thundering herd
    /// when multiple clients retry simultaneously.
    ///
    /// # Errors
    ///
    /// Returns [`NntpError::Other`] if all retry attempts fail. The underlying
    /// error may be a connection failure, authentication failure, or pool exhaustion.
    pub async fn get(&self) -> Result<PooledConnection<'_, NntpConnectionManager>> {
        let mut last_error = None;
        let mut backoff_ms = self.retry_config.initial_backoff_ms;

        for attempt in 0..=self.retry_config.max_retries {
            match self.pool.get().await {
                Ok(conn) => return Ok(conn),
                Err(e) => {
                    last_error = Some(e);

                    if attempt < self.retry_config.max_retries {
                        // Calculate backoff with optional jitter
                        let sleep_ms = if self.retry_config.jitter {
                            // Add 0-50% random jitter
                            let jitter = rand::thread_rng().gen_range(0..=(backoff_ms / 2));
                            backoff_ms + jitter
                        } else {
                            backoff_ms
                        };

                        warn!(
                            "Failed to get connection from pool (attempt {}/{}), retrying in {}ms: {}",
                            attempt + 1,
                            self.retry_config.max_retries + 1,
                            sleep_ms,
                            last_error.as_ref().unwrap()
                        );

                        tokio::time::sleep(Duration::from_millis(sleep_ms)).await;

                        // Exponential backoff for next attempt
                        backoff_ms = std::cmp::min(
                            (backoff_ms as f64 * self.retry_config.backoff_multiplier) as u64,
                            self.retry_config.max_backoff_ms,
                        );
                    }
                }
            }
        }

        Err(NntpError::Other(format!(
            "Failed to get connection from pool after {} attempts: {}",
            self.retry_config.max_retries + 1,
            last_error.map(|e| e.to_string()).unwrap_or_default()
        )))
    }

    /// Get a connection without retry (for cases where caller handles retry)
    ///
    /// # Errors
    ///
    /// Returns [`NntpError::Other`] if unable to get a connection from the pool.
    /// The underlying error may be a connection failure, authentication failure,
    /// or pool exhaustion.
    pub async fn get_no_retry(&self) -> Result<PooledConnection<'_, NntpConnectionManager>> {
        self.pool
            .get()
            .await
            .map_err(|e| NntpError::Other(format!("Failed to get connection from pool: {}", e)))
    }

    /// Get current pool state (for monitoring)
    ///
    /// Returns pool statistics including:
    /// - Number of connections in use
    /// - Number of idle connections
    /// - Total connections
    pub fn state(&self) -> bb8::State {
        self.pool.state()
    }

    /// Get the number of connections currently in use
    pub fn connections_in_use(&self) -> u32 {
        let state = self.pool.state();
        state.connections - state.idle_connections
    }

    /// Get the number of idle connections available
    pub fn idle_connections(&self) -> u32 {
        self.pool.state().idle_connections
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_manager_creation() {
        let config = ServerConfig {
            host: "news.example.com".to_string(),
            port: 563,
            tls: true,
            allow_insecure_tls: false,
            username: "testuser".to_string(),
            password: "testpass".to_string(),
        };

        let manager = NntpConnectionManager::new(config);
        assert_eq!(manager.config.host, "news.example.com");
        assert_eq!(manager.config.port, 563);
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
    }
}

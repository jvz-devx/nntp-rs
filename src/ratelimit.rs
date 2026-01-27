//! Rate limiting for bandwidth and connection management
//!
//! This module provides rate limiting capabilities using a token bucket algorithm
//! for bandwidth throttling and connection limiting.

use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use tokio::time::{Duration, Instant};

/// Token bucket rate limiter for bandwidth throttling
///
/// Implements the token bucket algorithm to limit data transfer rates.
/// Tokens are added at a fixed rate (bytes per second), and operations
/// consume tokens. If insufficient tokens are available, operations wait.
#[derive(Debug, Clone)]
pub struct BandwidthLimiter {
    inner: Arc<Mutex<BandwidthLimiterInner>>,
}

#[derive(Debug)]
struct BandwidthLimiterInner {
    /// Maximum bytes per second
    rate: u64,
    /// Current number of tokens (bytes) available
    tokens: f64,
    /// Maximum burst size (bucket capacity)
    capacity: f64,
    /// Last time tokens were added
    last_update: Instant,
}

impl BandwidthLimiter {
    /// Create a new bandwidth limiter
    ///
    /// # Arguments
    ///
    /// * `bytes_per_second` - Maximum transfer rate in bytes per second
    /// * `burst_size` - Optional burst size in bytes. If None, defaults to rate
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nntp_rs::BandwidthLimiter;
    ///
    /// // Limit to 1 MB/s with 2 MB burst
    /// let limiter = BandwidthLimiter::new(1_000_000, Some(2_000_000));
    /// ```
    pub fn new(bytes_per_second: u64, burst_size: Option<u64>) -> Self {
        let capacity = burst_size.unwrap_or(bytes_per_second) as f64;
        Self {
            inner: Arc::new(Mutex::new(BandwidthLimiterInner {
                rate: bytes_per_second,
                tokens: capacity,
                capacity,
                last_update: Instant::now(),
            })),
        }
    }

    /// Wait until the specified number of bytes can be consumed
    ///
    /// This method will block until sufficient tokens are available.
    /// Tokens are replenished at the configured rate.
    ///
    /// # Arguments
    ///
    /// * `bytes` - Number of bytes to consume
    ///
    /// # Example
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use nntp_rs::BandwidthLimiter;
    ///
    /// let limiter = BandwidthLimiter::new(1_000_000, None);
    /// // Wait until we can transfer 4096 bytes
    /// limiter.acquire(4096).await;
    /// // ... perform transfer ...
    /// # Ok(())
    /// # }
    /// ```
    pub async fn acquire(&self, bytes: u64) {
        loop {
            let mut inner = self.inner.lock().await;

            // Refill tokens based on elapsed time
            let now = Instant::now();
            let elapsed = now.duration_since(inner.last_update).as_secs_f64();
            let new_tokens = elapsed * inner.rate as f64;
            inner.tokens = (inner.tokens + new_tokens).min(inner.capacity);
            inner.last_update = now;

            // Check if we have enough tokens
            if inner.tokens >= bytes as f64 {
                inner.tokens -= bytes as f64;
                break;
            }

            // Calculate how long to wait for tokens to refill
            let tokens_needed = bytes as f64 - inner.tokens;
            let wait_seconds = tokens_needed / inner.rate as f64;
            let wait_duration = Duration::from_secs_f64(wait_seconds);

            // Release lock before sleeping
            drop(inner);
            tokio::time::sleep(wait_duration).await;
        }
    }

    /// Get current limiter configuration
    pub async fn config(&self) -> (u64, u64) {
        let inner = self.inner.lock().await;
        (inner.rate, inner.capacity as u64)
    }

    /// Get current available tokens
    pub async fn available_tokens(&self) -> u64 {
        let mut inner = self.inner.lock().await;

        // Update tokens first
        let now = Instant::now();
        let elapsed = now.duration_since(inner.last_update).as_secs_f64();
        let new_tokens = elapsed * inner.rate as f64;
        inner.tokens = (inner.tokens + new_tokens).min(inner.capacity);
        inner.last_update = now;

        inner.tokens as u64
    }
}

/// Connection limiter using semaphores
///
/// Limits the number of concurrent connections to prevent overwhelming
/// servers or exhausting local resources.
#[derive(Debug, Clone)]
pub struct ConnectionLimiter {
    /// Semaphore for limiting connections
    semaphore: Arc<Semaphore>,
    /// Maximum number of connections
    max_connections: usize,
}

impl ConnectionLimiter {
    /// Create a new connection limiter
    ///
    /// # Arguments
    ///
    /// * `max_connections` - Maximum number of concurrent connections
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nntp_rs::ConnectionLimiter;
    ///
    /// // Allow up to 10 concurrent connections
    /// let limiter = ConnectionLimiter::new(10);
    /// ```
    pub fn new(max_connections: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_connections)),
            max_connections,
        }
    }

    /// Acquire a connection permit
    ///
    /// This method will block until a connection slot is available.
    /// The permit is automatically released when dropped.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use nntp_rs::ConnectionLimiter;
    ///
    /// let limiter = ConnectionLimiter::new(10);
    /// let permit = limiter.acquire().await;
    /// // ... use connection ...
    /// // Permit is automatically released when dropped
    /// # Ok(())
    /// # }
    /// ```
    // Semaphore is never closed while ConnectionLimiter holds Arc reference
    #[expect(clippy::expect_used)]
    pub async fn acquire(&self) -> ConnectionPermit {
        let permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("BUG: semaphore closed while ConnectionLimiter holds Arc reference");
        ConnectionPermit { _permit: permit }
    }

    /// Try to acquire a connection permit without blocking
    ///
    /// Returns `Some(permit)` if a slot is available, `None` otherwise.
    pub fn try_acquire(&self) -> Option<ConnectionPermit> {
        self.semaphore
            .clone()
            .try_acquire_owned()
            .ok()
            .map(|permit| ConnectionPermit { _permit: permit })
    }

    /// Get the maximum number of connections
    pub fn max_connections(&self) -> usize {
        self.max_connections
    }

    /// Get the number of available connection slots
    pub fn available(&self) -> usize {
        self.semaphore.available_permits()
    }
}

/// RAII guard for connection permits
///
/// Automatically releases the permit when dropped.
#[derive(Debug)]
pub struct ConnectionPermit {
    _permit: tokio::sync::OwnedSemaphorePermit,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_bandwidth_limiter_basic() {
        let limiter = BandwidthLimiter::new(1000, None);

        // Should be able to acquire immediately (within bucket capacity)
        limiter.acquire(500).await;
        limiter.acquire(500).await;
    }

    #[tokio::test]
    async fn test_bandwidth_limiter_rate() {
        let limiter = BandwidthLimiter::new(1000, Some(1000));

        // Consume all tokens
        limiter.acquire(1000).await;

        // This should wait for tokens to refill
        let start = Instant::now();
        limiter.acquire(500).await;
        let elapsed = start.elapsed();

        // Should have waited approximately 500ms (500 bytes at 1000 bytes/sec)
        assert!(elapsed >= Duration::from_millis(400));
        assert!(elapsed <= Duration::from_millis(700));
    }

    #[tokio::test]
    async fn test_bandwidth_limiter_burst() {
        let limiter = BandwidthLimiter::new(1000, Some(5000));

        // Should be able to burst up to capacity
        limiter.acquire(5000).await;

        // Available tokens should be close to 0
        let available = limiter.available_tokens().await;
        assert!(available < 100);
    }

    #[tokio::test]
    async fn test_bandwidth_limiter_refill() {
        let limiter = BandwidthLimiter::new(1000, Some(1000));

        // Consume all tokens
        limiter.acquire(1000).await;

        // Wait for refill
        sleep(Duration::from_millis(500)).await;

        // Should have approximately 500 tokens available
        let available = limiter.available_tokens().await;
        assert!((400..=600).contains(&available));
    }

    #[tokio::test]
    async fn test_bandwidth_limiter_config() {
        let limiter = BandwidthLimiter::new(1000, Some(2000));
        let (rate, capacity) = limiter.config().await;
        assert_eq!(rate, 1000);
        assert_eq!(capacity, 2000);
    }

    #[tokio::test]
    async fn test_connection_limiter_basic() {
        let limiter = ConnectionLimiter::new(2);

        let _permit1 = limiter.acquire().await;
        let _permit2 = limiter.acquire().await;

        // Third acquire should block, try_acquire should fail
        assert!(limiter.try_acquire().is_none());
    }

    #[tokio::test]
    async fn test_connection_limiter_release() {
        let limiter = ConnectionLimiter::new(1);

        {
            let _permit = limiter.acquire().await;
            assert_eq!(limiter.available(), 0);
        } // permit dropped here

        // Should be available again
        assert_eq!(limiter.available(), 1);
    }

    #[tokio::test]
    async fn test_connection_limiter_max() {
        let limiter = ConnectionLimiter::new(5);
        assert_eq!(limiter.max_connections(), 5);
        assert_eq!(limiter.available(), 5);
    }

    #[tokio::test]
    async fn test_connection_limiter_concurrent() {
        let limiter = ConnectionLimiter::new(3);

        let permit1 = limiter.acquire().await;
        let permit2 = limiter.acquire().await;
        let permit3 = limiter.acquire().await;

        assert_eq!(limiter.available(), 0);

        drop(permit1);
        assert_eq!(limiter.available(), 1);

        drop(permit2);
        drop(permit3);
        assert_eq!(limiter.available(), 3);
    }
}

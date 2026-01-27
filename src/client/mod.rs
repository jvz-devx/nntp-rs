//! NNTP client implementation with TLS and compression support

mod articles;
mod auth;
mod compression;
mod connection;
mod group_ops;
mod high_throughput;
mod io;
mod listing;
mod metadata;
mod posting;
mod server;
mod state;

use crate::config::ServerConfig;
use state::{CompressionMode, ConnectionState};
use std::sync::Arc;
use tokio::io::BufReader;
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tracing::debug;

/// Async NNTP client with TLS and compression support
///
/// # Example
///
/// ```no_run
/// use nntp_rs::{NntpClient, ServerConfig};
/// use std::sync::Arc;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = ServerConfig::tls("news.example.com", "user", "pass");
/// let mut client = NntpClient::connect(Arc::new(config)).await?;
/// client.authenticate().await?;
///
/// // Enable compression for better bandwidth efficiency
/// client.try_enable_compression().await?;
///
/// let info = client.select_group("alt.test").await?;
/// println!("Group has {} articles", info.count);
/// # Ok(())
/// # }
/// ```
#[must_use]
pub struct NntpClient {
    /// TLS stream (both reader and writer)
    stream: BufReader<TlsStream<TcpStream>>,
    /// Connection state
    state: ConnectionState,
    /// Server configuration
    config: Arc<ServerConfig>,
    /// Currently selected newsgroup
    current_group: Option<String>,
    /// Compression mode for this connection
    compression_mode: CompressionMode,
    /// Total compressed bytes received (only when compression enabled)
    bytes_compressed: u64,
    /// Total decompressed bytes (original size)
    bytes_decompressed: u64,
    /// Whether this connection is broken (received garbage/invalid data)
    is_broken: bool,
}

impl NntpClient {
    /// Check if this connection is broken and should be discarded
    pub fn is_broken(&self) -> bool {
        self.is_broken
    }

    /// Mark this connection as broken
    fn mark_broken(&mut self) {
        self.is_broken = true;
    }

    /// Get the currently selected newsgroup, if any
    pub fn current_group(&self) -> Option<&str> {
        self.current_group.as_deref()
    }

    /// Check if the client is currently authenticated
    pub fn is_authenticated(&self) -> bool {
        matches!(self.state, ConnectionState::Authenticated)
    }
}

impl Drop for NntpClient {
    fn drop(&mut self) {
        debug!("NntpClient dropped");
    }
}

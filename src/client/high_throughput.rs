//! High-throughput article fetching operations
//!
//! This module provides optimized methods for fetching articles in bulk:
//! - Binary fetching (avoids UTF-8 parsing overhead)
//! - Command pipelining (reduces network round-trip latency)

use super::NntpClient;
use crate::commands;
use crate::error::{NntpError, Result};
use crate::response::codes;
use tracing::trace;

impl NntpClient {
    /// Fetch article as raw binary data (optimized for high-throughput)
    ///
    /// This is the high-performance version of `fetch_article` that returns
    /// raw binary data instead of parsed lines. Use this for bulk downloads
    /// where you need maximum throughput.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::NoSuchArticle`] - The article does not exist
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn fetch_article_binary(
        &mut self,
        id: &str,
    ) -> Result<crate::response::NntpBinaryResponse> {
        trace!("Fetching article (binary): {}", id);

        let cmd = commands::article(id);
        self.send_command(&cmd).await?;
        let response = self.read_multiline_response_binary().await?;

        if response.code == codes::NO_SUCH_ARTICLE_ID
            || response.code == codes::NO_SUCH_ARTICLE_NUMBER
        {
            return Err(NntpError::NoSuchArticle(id.to_string()));
        }

        if !response.is_success() {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        Ok(response)
    }

    /// Fetch article body as raw binary data (optimized for high-throughput)
    ///
    /// Like `fetch_article_binary` but only fetches the body without headers.
    pub async fn fetch_body_binary(
        &mut self,
        id: &str,
    ) -> Result<crate::response::NntpBinaryResponse> {
        trace!("Fetching body (binary): {}", id);

        let cmd = commands::body(id);
        self.send_command(&cmd).await?;
        let response = self.read_multiline_response_binary().await?;

        if !response.is_success() {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        Ok(response)
    }

    /// Fetch multiple articles with pipelining for improved throughput
    ///
    /// Implements NNTP command pipelining by sending multiple ARTICLE commands
    /// before waiting for responses. This reduces the impact of network latency
    /// by overlapping command transmission with server processing.
    ///
    /// # Arguments
    ///
    /// * `ids` - Slice of message IDs to fetch
    /// * `max_pipeline` - Maximum number of commands to pipeline at once (e.g., 10)
    ///
    /// # Performance
    ///
    /// Pipelining can significantly improve throughput by reducing round-trip latency:
    /// - Without pipelining: send → wait → receive → process (sequential)
    /// - With pipelining: send N → receive N → process N (batched)
    ///
    /// For high-latency connections, this can improve throughput by 30-50%.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::{NntpClient, ServerConfig};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ServerConfig::plain("news.example.com", "user", "pass");
    /// let mut client = NntpClient::connect(Arc::new(config)).await?;
    ///
    /// let message_ids = vec!["<msg1@example.com>", "<msg2@example.com>", "<msg3@example.com>"];
    /// let responses = client.fetch_articles_pipelined(&message_ids, 10).await?;
    ///
    /// for (id, response) in message_ids.iter().zip(responses.iter()) {
    ///     println!("Fetched article {}: {} bytes", id, response.data.len());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::NoSuchArticle`] - One of the articles does not exist (pipeline aborts)
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    /// - Any I/O error occurs during command transmission or response reading
    ///
    /// Note: If an error occurs, the pipeline aborts and returns the error immediately.
    /// Articles fetched before the error are discarded.
    pub async fn fetch_articles_pipelined(
        &mut self,
        ids: &[&str],
        max_pipeline: usize,
    ) -> Result<Vec<crate::response::NntpBinaryResponse>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        // Validate pipeline depth
        let pipeline_depth = max_pipeline.max(1); // Minimum 1
        let mut results = Vec::with_capacity(ids.len());

        trace!(
            "Fetching {} articles with pipeline depth {}",
            ids.len(),
            pipeline_depth
        );

        // Process articles in chunks based on pipeline depth
        for chunk in ids.chunks(pipeline_depth) {
            // Phase 1: Send all commands in the chunk without waiting for responses
            for id in chunk {
                let cmd = commands::article(id);
                self.send_command(&cmd).await?;
            }

            // Phase 2: Read all responses in the same order as commands were sent
            for id in chunk {
                let response = self.read_multiline_response_binary().await?;

                // Check for article not found errors
                if response.code == codes::NO_SUCH_ARTICLE_ID
                    || response.code == codes::NO_SUCH_ARTICLE_NUMBER
                {
                    return Err(NntpError::NoSuchArticle(id.to_string()));
                }

                // Check for other protocol errors
                if !response.is_success() {
                    return Err(NntpError::Protocol {
                        code: response.code,
                        message: response.message,
                    });
                }

                results.push(response);
            }
        }

        Ok(results)
    }
}

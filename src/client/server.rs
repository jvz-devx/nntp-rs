//! Server management and query commands
//!
//! This module implements server-level NNTP commands defined in:
//! - RFC 3977 §5.2 (CAPABILITIES), §5.3 (MODE READER), §7.1 (DATE), §7.2 (HELP)
//! - RFC 4644 §2.3-2.5 (MODE STREAM, CHECK, TAKETHIS)

use super::NntpClient;
use crate::article::Article;
use crate::capabilities::Capabilities;
use crate::commands;
use crate::error::{NntpError, Result};
use crate::response::{codes, NntpResponse};
use tracing::debug;

impl NntpClient {
    /// Request server capabilities (RFC 3977 Section 5.2)
    ///
    /// Returns the list of capabilities supported by the server.
    /// This command can be used to detect which extensions and features
    /// the server supports before attempting to use them.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::{NntpClient, ServerConfig};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ServerConfig::tls("news.example.com", "user", "pass");
    /// # let mut client = NntpClient::connect(Arc::new(config)).await?;
    /// let caps = client.capabilities().await?;
    /// if caps.has("COMPRESS") {
    ///     println!("Server supports compression");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn capabilities(&mut self) -> Result<Capabilities> {
        debug!("Requesting server capabilities");

        let cmd = commands::capabilities();
        self.send_command(cmd).await?;
        let response = self.read_multiline_response().await?;

        if response.code != codes::CAPABILITY_LIST {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        let caps = Capabilities::parse(&response.lines);
        debug!("Received {} capabilities", caps.list().len());
        Ok(caps)
    }

    /// Switch to reader mode (RFC 3977 Section 5.3)
    ///
    /// Instructs the server to switch to reader mode, indicating this is a news
    /// reading client (as opposed to a news transfer agent). Many servers require
    /// this command before accepting most client commands.
    ///
    /// Returns `true` if posting is allowed (code 200), `false` if posting is
    /// not permitted (code 201).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::{NntpClient, ServerConfig};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ServerConfig::tls("news.example.com", "user", "pass");
    /// # let mut client = NntpClient::connect(Arc::new(config)).await?;
    /// let posting_allowed = client.mode_reader().await?;
    /// if posting_allowed {
    ///     println!("Posting is allowed on this server");
    /// } else {
    ///     println!("Read-only access - posting not permitted");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn mode_reader(&mut self) -> Result<bool> {
        debug!("Switching to reader mode");

        let cmd = commands::mode_reader();
        self.send_command(cmd).await?;
        let response = self.read_response().await?;

        match response.code {
            codes::READY_POSTING_ALLOWED => {
                debug!("Reader mode enabled - posting allowed");
                Ok(true)
            }
            codes::READY_NO_POSTING => {
                debug!("Reader mode enabled - posting not allowed");
                Ok(false)
            }
            _ => Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            }),
        }
    }

    /// Switch to streaming mode (RFC 4644 Section 2.3)
    ///
    /// Requests to switch to streaming mode for efficient bulk article transfer.
    /// Streaming mode allows pipelined CHECK and TAKETHIS commands without
    /// waiting for individual responses.
    ///
    /// Before calling this method, check if the server supports streaming by
    /// inspecting the CAPABILITIES response for the STREAMING capability.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::{NntpClient, ServerConfig};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ServerConfig::tls("news.example.com", "user", "pass");
    /// # let mut client = NntpClient::connect(Arc::new(config)).await?;
    /// // Check capabilities first
    /// let caps = client.capabilities().await?;
    /// if caps.has("STREAMING") {
    ///     client.mode_stream().await?;
    ///     println!("Streaming mode enabled");
    ///     // Now you can use CHECK and TAKETHIS commands
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::Protocol`] - Server does not support streaming or returned an error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn mode_stream(&mut self) -> Result<()> {
        debug!("Switching to streaming mode");

        let cmd = commands::mode_stream();
        self.send_command(cmd).await?;
        let response = self.read_response().await?;

        if response.code != codes::STREAMING_OK {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        debug!("Streaming mode enabled");
        Ok(())
    }

    /// Check if server wants an article (RFC 4644 Section 2.4)
    ///
    /// In streaming mode, checks whether the server wants an article by its message-id.
    /// This is the first phase of the streaming article transfer protocol.
    ///
    /// The server responds with:
    /// - 238 (CHECK_SEND) - Server wants the article; use TAKETHIS to send it
    /// - 431 (CHECK_LATER) - Server is temporarily unavailable; retry later
    /// - 438 (CHECK_NOT_WANTED) - Server does not want this article
    ///
    /// CHECK can be pipelined - multiple CHECK commands can be sent without waiting
    /// for responses. The server includes the message-id in each response for matching.
    ///
    /// **Note:** You must call [`mode_stream()`](Self::mode_stream) before using CHECK.
    ///
    /// # Arguments
    ///
    /// * `message_id` - The message-id to check (e.g., "<abc123@example.com>")
    ///
    /// # Returns
    ///
    /// Returns the full response, including the message-id in the response message.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::{NntpClient, ServerConfig, codes};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ServerConfig::tls("news.example.com", "user", "pass");
    /// # let mut client = NntpClient::connect(Arc::new(config)).await?;
    /// // First enable streaming mode
    /// client.mode_stream().await?;
    ///
    /// // Check if server wants an article
    /// let message_id = "<article123@example.com>";
    /// let response = client.check(message_id).await?;
    ///
    /// match response.code {
    ///     codes::CHECK_SEND => {
    ///         println!("Server wants article - send with TAKETHIS");
    ///         // client.takethis(message_id, article_data).await?;
    ///     }
    ///     codes::CHECK_LATER => {
    ///         println!("Server busy - retry later");
    ///     }
    ///     codes::CHECK_NOT_WANTED => {
    ///         println!("Server doesn't want article");
    ///     }
    ///     _ => {
    ///         println!("Unexpected response: {}", response.code);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::Protocol`] - Server returned an error or unexpected response
    /// - [`NntpError::Timeout`] - Server did not respond in time
    /// - Network I/O errors
    pub async fn check(&mut self, message_id: &str) -> Result<NntpResponse> {
        debug!("CHECK: {}", message_id);

        let cmd = commands::check(message_id);
        self.send_command(&cmd).await?;
        let response = self.read_response().await?;

        debug!(
            "CHECK response for {}: {} {}",
            message_id, response.code, response.message
        );
        Ok(response)
    }

    /// Send an article to the server in streaming mode (RFC 4644 Section 2.5)
    ///
    /// In streaming mode, sends an article to the server without waiting for permission.
    /// The article is sent immediately after the command, and the server responds after
    /// receiving the complete article.
    ///
    /// The server responds with:
    /// - 239 (TAKETHIS_RECEIVED) - Article received successfully
    /// - 439 (TAKETHIS_REJECTED) - Article rejected; do not retry
    ///
    /// TAKETHIS can be pipelined - multiple TAKETHIS commands can be sent without waiting
    /// for responses. The server includes the message-id in each response for matching.
    ///
    /// **Note:** You must call [`mode_stream()`](Self::mode_stream) before using TAKETHIS.
    ///
    /// # Arguments
    ///
    /// * `message_id` - The message-id of the article (e.g., "<abc123@example.com>")
    /// * `article` - The article to send
    ///
    /// # Returns
    ///
    /// Returns the full response, including the message-id in the response message.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::{NntpClient, ServerConfig, ArticleBuilder, codes};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ServerConfig::tls("news.example.com", "user", "pass");
    /// # let mut client = NntpClient::connect(Arc::new(config)).await?;
    /// // First enable streaming mode
    /// client.mode_stream().await?;
    ///
    /// // Build an article
    /// let article = ArticleBuilder::new()
    ///     .from("user@example.com")
    ///     .subject("Test post")
    ///     .newsgroups(vec!["test.group"])
    ///     .body("This is a test")
    ///     .build()?;
    ///
    /// let message_id = article.headers.message_id.clone();
    ///
    /// // Send the article without asking first
    /// let response = client.takethis(&message_id, &article).await?;
    ///
    /// match response.code {
    ///     codes::TAKETHIS_RECEIVED => {
    ///         println!("Article received successfully");
    ///     }
    ///     codes::TAKETHIS_REJECTED => {
    ///         println!("Article rejected by server");
    ///     }
    ///     _ => {
    ///         println!("Unexpected response: {}", response.code);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::Protocol`] - Server returned an error or unexpected response
    /// - [`NntpError::Timeout`] - Server did not respond in time
    /// - Network I/O errors
    /// - Article serialization fails
    pub async fn takethis(&mut self, message_id: &str, article: &Article) -> Result<NntpResponse> {
        debug!("TAKETHIS: {}", message_id);

        // Serialize article with CRLF and dot-stuffing
        let article_data = article.serialize_for_posting()?;

        let cmd = commands::takethis(message_id, &article_data);
        self.send_command(&cmd).await?;
        let response = self.read_response().await?;

        debug!(
            "TAKETHIS response for {}: {} {}",
            message_id, response.code, response.message
        );
        Ok(response)
    }

    /// Get server date/time (RFC 3977 Section 7.1)
    ///
    /// Requests the server's current date and time in UTC.
    ///
    /// Returns the server timestamp in the format `YYYYMMDDhhmmss`
    /// (e.g., "20240115123456" represents January 15, 2024 at 12:34:56 UTC).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::{NntpClient, ServerConfig};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ServerConfig::tls("news.example.com", "user", "pass");
    /// # let mut client = NntpClient::connect(Arc::new(config)).await?;
    /// let server_time = client.date().await?;
    /// println!("Server time: {}", server_time);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn date(&mut self) -> Result<String> {
        debug!("Requesting server date/time");

        let cmd = commands::date();
        self.send_command(cmd).await?;
        let response = self.read_response().await?;

        if response.code != codes::SERVER_DATE {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        // Extract timestamp from response message (format: "111 YYYYMMDDhhmmss")
        let timestamp = response
            .message
            .split_whitespace()
            .next()
            .unwrap_or(&response.message)
            .to_string();

        debug!("Server date/time: {}", timestamp);
        Ok(timestamp)
    }

    /// Request help text from the server (RFC 3977 §7.2)
    ///
    /// Returns multi-line help text from the server describing available commands
    /// and server-specific information.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::{NntpClient, ServerConfig};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ServerConfig::tls("news.example.com", "user", "pass");
    /// # let mut client = NntpClient::connect(Arc::new(config)).await?;
    /// let help_response = client.help().await?;
    /// for line in &help_response.lines {
    ///     println!("{}", line);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn help(&mut self) -> Result<NntpResponse> {
        debug!("Requesting help text");

        let cmd = commands::help();
        self.send_command(cmd).await?;
        let response = self.read_multiline_response().await?;

        if response.code != codes::HELP_TEXT_FOLLOWS {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        debug!("Received help text ({} lines)", response.lines.len());
        Ok(response)
    }
}

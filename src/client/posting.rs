use crate::commands;
use crate::response::codes;
use crate::{NntpError, Result};
use tracing::debug;

use super::state::ConnectionState;
use super::NntpClient;

impl NntpClient {
    /// Post a new article to the server (RFC 3977 Section 6.3.1)
    ///
    /// The POST command allows clients to submit articles to the server.
    /// The server may reject articles for various reasons (permissions, content policy, etc.)
    ///
    /// # Authentication
    ///
    /// Most servers require authentication before allowing posting.
    /// Returns `NntpError::Protocol` with code 480 if not authenticated.
    ///
    /// # Two-Phase Protocol
    ///
    /// 1. Client sends POST command
    /// 2. Server responds:
    ///    - 340: Send the article (posting is allowed)
    ///    - 440: Posting not permitted
    /// 3. If 340 received, client sends article text with dot-stuffing
    /// 4. Server responds:
    ///    - 240: Article posted successfully
    ///    - 441: Posting failed
    ///
    /// # Arguments
    ///
    /// * `article` - The article to post (must have valid headers and body)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::{NntpClient, ServerConfig, article::ArticleBuilder};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ServerConfig::tls("news.example.com", "user", "pass");
    /// # let mut client = NntpClient::connect(Arc::new(config)).await?;
    /// let article = ArticleBuilder::new()
    ///     .from("user@example.com")
    ///     .subject("Test Article")
    ///     .newsgroups(vec!["test.group".to_string()])
    ///     .body("This is a test article.")
    ///     .build()?;
    ///
    /// client.post(&article).await?;
    /// println!("Article posted successfully");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::PostingNotPermitted`] - Server does not allow posting (440)
    /// - [`NntpError::PostingFailed`] - Server rejected the article (441)
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn post(&mut self, article: &crate::article::Article) -> Result<()> {
        debug!("Posting article");

        // Verify authenticated - most servers require authentication for posting
        if !matches!(self.state, ConnectionState::Authenticated) {
            return Err(NntpError::Protocol {
                code: 480,
                message: "Authentication required".to_string(),
            });
        }

        // Phase 1: Send POST command
        let cmd = commands::post();
        self.send_command(cmd).await?;
        let response = self.read_response().await?;

        // Check for 340 (send article text) response
        if response.code == codes::POSTING_NOT_PERMITTED {
            return Err(NntpError::PostingNotPermitted);
        }

        if response.code != codes::SEND_ARTICLE {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        // Phase 2: Send article text with dot-stuffing
        let article_text = article.serialize_for_posting()?;

        // Send the article body (already has CRLF and dot-stuffing)
        self.send_command(&article_text).await?;

        // Send terminating dot line
        self.send_command(".\r\n").await?;

        // Wait for final response
        let response = self.read_response().await?;

        // Check result
        if response.code == codes::POSTING_FAILED {
            return Err(NntpError::PostingFailed(response.message));
        }

        if response.code != codes::ARTICLE_POSTED {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        debug!("Article posted successfully");
        Ok(())
    }

    /// Transfer an article to the server using IHAVE (RFC 3977 Section 6.3.2)
    ///
    /// IHAVE is used for server-to-server article transfer. The server decides
    /// whether it wants the article based on the message-id.
    ///
    /// # Two-Phase Protocol
    ///
    /// 1. Client sends IHAVE command with message-id
    /// 2. Server responds:
    ///    - 335: Send the article (server wants it)
    ///    - 435: Article not wanted (server already has it)
    ///    - 436: Transfer not possible; try again later
    /// 3. If 335 received, client sends article text
    /// 4. Server responds:
    ///    - 235: Article transferred successfully
    ///    - 436: Transfer failed; try again later
    ///    - 437: Transfer rejected; do not retry
    ///
    /// # Arguments
    ///
    /// * `message_id` - The message-id of the article (e.g., "<abc@example.com>")
    /// * `article` - The article to transfer
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the article was successfully transferred (code 235).
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - `ArticleNotWanted` (code 435) - Server doesn't want the article
    /// - `TransferNotPossible` (code 436) - Temporary failure; caller should retry
    /// - `TransferRejected` (code 437) - Permanent rejection; do not retry
    /// - `Protocol` - Other protocol errors
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::*;
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<()> {
    /// # let config = ServerConfig::plain("news.example.com", "user", "pass");
    /// # let mut client = NntpClient::connect(Arc::new(config)).await?;
    /// let article = ArticleBuilder::new()
    ///     .from("user@example.com")
    ///     .subject("Test Article")
    ///     .newsgroups(vec!["test.group"])
    ///     .body("Article body text")
    ///     .build()?;
    ///
    /// let message_id = article.headers.message_id.clone();
    /// match client.ihave(&message_id, &article).await {
    ///     Ok(()) => println!("Article transferred"),
    ///     Err(NntpError::ArticleNotWanted) => println!("Server already has it"),
    ///     Err(NntpError::TransferNotPossible(msg)) => println!("Retry later: {}", msg),
    ///     Err(e) => return Err(e),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn ihave(
        &mut self,
        message_id: &str,
        article: &crate::article::Article,
    ) -> Result<()> {
        debug!("IHAVE: offering article {}", message_id);

        // Verify authenticated - IHAVE is for server-to-server transfer
        if !matches!(self.state, ConnectionState::Authenticated) {
            return Err(NntpError::Protocol {
                code: 480,
                message: "Authentication required".to_string(),
            });
        }

        // Phase 1: Send IHAVE command with message-id
        let cmd = commands::ihave(message_id);
        self.send_command(&cmd).await?;
        let response = self.read_response().await?;

        // Handle first-phase responses
        match response.code {
            codes::ARTICLE_NOT_WANTED => {
                debug!("Article not wanted (code 435)");
                return Err(NntpError::ArticleNotWanted);
            }
            codes::TRANSFER_NOT_POSSIBLE => {
                debug!("Transfer not possible (code 436): {}", response.message);
                return Err(NntpError::TransferNotPossible(response.message));
            }
            codes::SEND_ARTICLE_TRANSFER => {
                debug!("Server wants article (code 335), sending...");
                // Continue to phase 2
            }
            _ => {
                return Err(NntpError::Protocol {
                    code: response.code,
                    message: response.message,
                });
            }
        }

        // Phase 2: Send article text with dot-stuffing
        let article_text = article.serialize_for_posting()?;

        // Send the article body (already has CRLF and dot-stuffing)
        self.send_command(&article_text).await?;

        // Send terminating dot line
        self.send_command(".\r\n").await?;

        // Wait for final response
        let response = self.read_response().await?;

        // Handle second-phase responses
        match response.code {
            codes::ARTICLE_TRANSFERRED => {
                debug!("Article transferred successfully (code 235)");
                Ok(())
            }
            codes::TRANSFER_NOT_POSSIBLE => {
                debug!("Transfer failed (code 436): {}", response.message);
                Err(NntpError::TransferNotPossible(response.message))
            }
            codes::TRANSFER_REJECTED => {
                debug!("Transfer rejected (code 437): {}", response.message);
                Err(NntpError::TransferRejected(response.message))
            }
            _ => Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            }),
        }
    }

    /// Close the connection gracefully (RFC 3977 Section 5.4)
    ///
    /// Sends the QUIT command to cleanly terminate the connection.
    /// After calling this method, the client should not be used for further operations.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::{NntpClient, ServerConfig};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ServerConfig::tls("news.example.com", "user", "pass");
    /// # let mut client = NntpClient::connect(Arc::new(config)).await?;
    /// // ... use the client ...
    /// client.quit().await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn quit(&mut self) -> Result<()> {
        debug!("Closing NNTP connection");

        let cmd = commands::quit();
        self.send_command(cmd).await?;
        let _response = self.read_response().await?;

        self.state = ConnectionState::Closed;
        Ok(())
    }
}

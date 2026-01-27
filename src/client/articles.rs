//! Article retrieval and navigation commands (RFC 3977 §6.2)
//!
//! This module implements NNTP commands for fetching articles and navigating
//! within a newsgroup:
//! - ARTICLE - Fetch full article (headers + body)
//! - HEAD - Fetch headers only
//! - BODY - Fetch body only
//! - STAT - Check article status without retrieving content
//! - NEXT - Navigate to next article
//! - LAST - Navigate to previous article

use crate::{NntpError, NntpResponse, Result, commands, response::codes};
use tracing::trace;

use super::NntpClient;

impl NntpClient {
    /// Fetch article by message-ID or number
    ///
    /// Returns the full article (headers and body).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::NoSuchArticle`] - The article does not exist
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn fetch_article(&mut self, id: &str) -> Result<NntpResponse> {
        trace!("Fetching article: {}", id);

        let cmd = commands::article(id);
        self.send_command(&cmd).await?;
        let response = self.read_multiline_response().await?;

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

    /// Fetch article headers only
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::NoSuchArticle`] - The article does not exist
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn fetch_head(&mut self, id: &str) -> Result<NntpResponse> {
        trace!("Fetching head: {}", id);

        let cmd = commands::head(id);
        self.send_command(&cmd).await?;
        let response = self.read_multiline_response().await?;

        if !response.is_success() {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        Ok(response)
    }

    /// Fetch article body only
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::NoSuchArticle`] - The article does not exist
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn fetch_body(&mut self, id: &str) -> Result<NntpResponse> {
        trace!("Fetching body: {}", id);

        let cmd = commands::body(id);
        self.send_command(&cmd).await?;
        let response = self.read_multiline_response().await?;

        if !response.is_success() {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        Ok(response)
    }

    /// Check article status without retrieving content (RFC 3977 §6.2.4)
    ///
    /// The STAT command allows checking whether an article exists and retrieving
    /// its metadata without downloading the full content. This is useful for
    /// checking article existence or getting message-id mapping.
    ///
    /// # Arguments
    ///
    /// * `id` - Either an article number (e.g., "12345") or message-id (e.g., "<abc@example.com>")
    ///
    /// # Returns
    ///
    /// Returns [`ArticleInfo`](crate::commands::ArticleInfo) containing:
    /// - `number`: Article number (0 if message-id was used in request)
    /// - `message_id`: The article's message identifier
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::{NntpClient, ServerConfig};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ServerConfig::tls("news.example.com", "user", "pass");
    /// # let mut client = NntpClient::connect(Arc::new(config)).await?;
    /// # client.select_group("comp.lang.rust").await?;
    /// // Check by article number
    /// let info = client.stat("12345").await?;
    /// println!("Article {} has message-id: {}", info.number, info.message_id);
    ///
    /// // Check by message-id
    /// let info = client.stat("<abc@example.com>").await?;
    /// println!("Message exists at article number: {}", info.number);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::NoSuchArticle`] - Article does not exist (code 430)
    /// - [`NntpError::NoGroupSelected`] - No newsgroup selected (code 412)
    /// - [`NntpError::InvalidArticleNumber`] - Invalid article number (code 423)
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn stat(&mut self, id: &str) -> Result<commands::ArticleInfo> {
        trace!("Checking article status: {}", id);

        let cmd = commands::stat(id);
        self.send_command(&cmd).await?;
        let response = self.read_response().await?;

        // Handle specific error codes
        if response.code == codes::NO_SUCH_ARTICLE_ID
            || response.code == codes::NO_SUCH_ARTICLE_NUMBER
        {
            return Err(NntpError::NoSuchArticle(id.to_string()));
        }

        if response.code == codes::NO_GROUP_SELECTED {
            return Err(NntpError::NoGroupSelected);
        }

        if response.code == codes::NO_CURRENT_ARTICLE {
            return Err(NntpError::InvalidArticleNumber);
        }

        // Parse the response
        commands::parse_stat_response(response)
    }

    /// Navigate to the next article in the current newsgroup
    ///
    /// Moves the server's internal pointer to the next article in the currently selected
    /// newsgroup and returns its article number and message-id.
    ///
    /// Corresponds to the NEXT command in RFC 3977 Section 6.1.4.
    ///
    /// # Returns
    ///
    /// Returns [`ArticleInfo`](crate::commands::ArticleInfo) for the next article.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::NoGroupSelected`] - No newsgroup is currently selected (code 412)
    /// - [`NntpError::InvalidArticleNumber`] - No current article selected (code 420)
    /// - [`NntpError::NoSuchArticle`] - No next article in the group (code 421)
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use nntp_rs::{NntpClient, ServerConfig};
    /// # use std::sync::Arc;
    /// # async fn example() -> nntp_rs::Result<()> {
    /// # let config = ServerConfig::plain("news.example.com", "user", "pass");
    /// # let mut client = NntpClient::connect(Arc::new(config)).await?;
    /// // Select a newsgroup first
    /// client.select_group("comp.lang.rust").await?;
    ///
    /// // Navigate to next article
    /// let info = client.next().await?;
    /// println!("Next article: {} <{}>", info.number, info.message_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn next(&mut self) -> Result<commands::ArticleInfo> {
        trace!("Navigating to next article");

        let cmd = commands::next();
        self.send_command(cmd).await?;
        let response = self.read_response().await?;

        // Handle specific error codes
        if response.code == codes::NO_GROUP_SELECTED {
            return Err(NntpError::NoGroupSelected);
        }

        if response.code == codes::NO_CURRENT_ARTICLE {
            return Err(NntpError::InvalidArticleNumber);
        }

        if response.code == codes::NO_NEXT_ARTICLE {
            return Err(NntpError::NoSuchArticle("no next article".to_string()));
        }

        // Parse the response
        commands::parse_next_response(response)
    }

    /// Navigate to the previous article in the selected newsgroup (RFC 3977 §6.1.3)
    ///
    /// The LAST command moves the currently selected article pointer backwards to the
    /// previous article in the currently selected newsgroup.
    ///
    /// Corresponds to the LAST command in RFC 3977 Section 6.1.3.
    ///
    /// # Returns
    ///
    /// Returns [`ArticleInfo`](crate::commands::ArticleInfo) for the previous article.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::NoGroupSelected`] - No newsgroup is currently selected (code 412)
    /// - [`NntpError::InvalidArticleNumber`] - No current article selected (code 420)
    /// - [`NntpError::NoSuchArticle`] - No previous article in the group (code 422)
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use nntp_rs::{NntpClient, ServerConfig};
    /// # use std::sync::Arc;
    /// # async fn example() -> nntp_rs::Result<()> {
    /// # let config = ServerConfig::plain("news.example.com", "user", "pass");
    /// # let mut client = NntpClient::connect(Arc::new(config)).await?;
    /// // Select a newsgroup first
    /// client.select_group("comp.lang.rust").await?;
    ///
    /// // Navigate to previous article
    /// let info = client.last().await?;
    /// println!("Previous article: {} <{}>", info.number, info.message_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn last(&mut self) -> Result<commands::ArticleInfo> {
        trace!("Navigating to previous article");

        let cmd = commands::last();
        self.send_command(cmd).await?;
        let response = self.read_response().await?;

        // Handle specific error codes
        if response.code == codes::NO_GROUP_SELECTED {
            return Err(NntpError::NoGroupSelected);
        }

        if response.code == codes::NO_CURRENT_ARTICLE {
            return Err(NntpError::InvalidArticleNumber);
        }

        if response.code == codes::NO_PREV_ARTICLE {
            return Err(NntpError::NoSuchArticle("no previous article".to_string()));
        }

        // Parse the response
        commands::parse_last_response(response)
    }
}

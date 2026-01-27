//! Article metadata retrieval commands (XOVER, OVER, HDR)
//!
//! This module contains commands for efficiently retrieving article metadata
//! without downloading full article content. These commands are used for
//! browsing newsgroups and building article lists.

use crate::commands::{self, XoverEntry};
use crate::error::{NntpError, Result};
use crate::response::codes;
use tracing::{trace, warn};

use super::NntpClient;

impl NntpClient {
    /// Fetch article overview data using XOVER command (legacy name)
    ///
    /// XOVER is the legacy name for retrieving article metadata. Modern clients
    /// should prefer the [`over()`](Self::over) method which is the RFC 3977 standard name.
    ///
    /// This retrieves article metadata (subject, author, date, message-id, etc.)
    /// for a range of articles without downloading full content.
    ///
    /// # Arguments
    ///
    /// * `range` - Article range specification (e.g., "100-200", "100-", "-200")
    ///
    /// # Returns
    ///
    /// Returns a [`Vec<XoverEntry>`] containing overview metadata for each article.
    /// Failed parse lines are logged and skipped (not returned as errors).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::Protocol`] - Server returned an error (e.g., no group selected)
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn fetch_xover(&mut self, range: &str) -> Result<Vec<XoverEntry>> {
        trace!("Fetching XOVER: {}", range);

        let cmd = commands::xover(range);
        self.send_command(&cmd).await?;
        let response = self.read_multiline_response().await?;

        if !response.is_success() {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        // Pre-allocate: one entry per response line (minus failed parses)
        let mut entries = Vec::with_capacity(response.lines.len());
        for line in &response.lines {
            match commands::parse_xover_line(line) {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    warn!("Failed to parse XOVER line: {} - {}", line, e);
                    continue;
                }
            }
        }

        Ok(entries)
    }

    /// Fetch article overview data using OVER command (RFC 3977 §8.3)
    ///
    /// OVER is the RFC 3977 standard name for the XOVER command. It retrieves
    /// article metadata (subject, author, date, message-id, etc.) for a range
    /// of articles or a single message-id.
    ///
    /// This is more efficient than fetching full articles when you only need
    /// metadata for browsing or searching.
    ///
    /// # Arguments
    ///
    /// * `range_or_msgid` - Article range, single number, or message-id
    ///
    /// Range format examples:
    /// - "100" - single article
    /// - "100-200" - range from 100 to 200
    /// - "100-" - from 100 to end
    /// - "<abc@example.com>" - specific article by message-id
    /// - Empty string "" - current article
    ///
    /// # Returns
    ///
    /// Returns a [`Vec<XoverEntry>`] containing overview metadata for each article.
    /// Failed parse lines are logged and skipped (not returned as errors).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::Protocol`] - Server returned an error (e.g., no group selected)
    /// - [`NntpError::Timeout`] - Server did not respond in time
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::{NntpClient, ServerConfig};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ServerConfig::plain("news.example.com", "user", "pass");
    /// # let mut client = NntpClient::connect(Arc::new(config)).await?;
    /// // Select a newsgroup
    /// client.select_group("comp.lang.rust").await?;
    ///
    /// // Fetch overview for a range of articles
    /// let entries = client.over("1-100").await?;
    /// for entry in entries {
    ///     println!("{}: {}", entry.article_number, entry.subject);
    /// }
    ///
    /// // Fetch overview for current article
    /// let current = client.over("").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn over(&mut self, range_or_msgid: &str) -> Result<Vec<XoverEntry>> {
        trace!("Fetching OVER: {}", range_or_msgid);

        if range_or_msgid.is_empty() {
            self.send_command(commands::over_current()).await?;
        } else {
            let cmd = commands::over(range_or_msgid);
            self.send_command(&cmd).await?;
        }
        let response = self.read_multiline_response().await?;

        if response.code == codes::NO_GROUP_SELECTED {
            return Err(NntpError::NoGroupSelected);
        }

        if response.code == codes::NO_CURRENT_ARTICLE {
            return Err(NntpError::InvalidArticleNumber);
        }

        if !response.is_success() {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        // Pre-allocate: one entry per response line (minus failed parses)
        let mut entries = Vec::with_capacity(response.lines.len());
        for line in &response.lines {
            match commands::parse_xover_line(line) {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    warn!("Failed to parse OVER line: {} - {}", line, e);
                    continue;
                }
            }
        }

        Ok(entries)
    }

    /// Retrieve specific header field values from articles (HDR command)
    ///
    /// Fetches the value of a specific header field from one or more articles.
    ///
    /// # Arguments
    ///
    /// * `field` - The header field name to retrieve (e.g., "Subject", "From", "Date")
    /// * `range_or_msgid` - Article range ("100-200"), single article ("12345"),
    ///   message-id ("<id@example.com>"), or empty string ("") for current article
    ///
    /// # Returns
    ///
    /// Returns a vector of `HdrEntry` containing article numbers and header values.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use nntp_rs::{NntpClient, ServerConfig};
    /// # use std::sync::Arc;
    /// # async fn example() -> nntp_rs::Result<()> {
    /// # let config = ServerConfig::plain("news.example.com", "user", "pass");
    /// # let mut client = NntpClient::connect(Arc::new(config)).await?;
    /// # client.select_group("misc.test").await?;
    /// // Get Subject header for range of articles
    /// let subjects = client.hdr("Subject", "1-100").await?;
    /// for entry in subjects {
    ///     println!("Article {}: {}", entry.article_number, entry.value);
    /// }
    ///
    /// // Get From header for current article
    /// let authors = client.hdr("From", "").await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// RFC 3977 Section 8.5
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::NoGroupSelected`] - No newsgroup has been selected (code 412)
    /// - [`NntpError::InvalidArticleNumber`] - Current article number is invalid (code 420)
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn hdr(
        &mut self,
        field: &str,
        range_or_msgid: &str,
    ) -> Result<Vec<commands::HdrEntry>> {
        trace!("Fetching HDR {}: {}", field, range_or_msgid);

        let cmd = if range_or_msgid.is_empty() {
            commands::hdr_current(field)
        } else {
            commands::hdr(field, range_or_msgid)
        };
        self.send_command(&cmd).await?;
        let response = self.read_multiline_response().await?;

        if response.code == codes::NO_GROUP_SELECTED {
            return Err(NntpError::NoGroupSelected);
        }

        if response.code == codes::NO_CURRENT_ARTICLE {
            return Err(NntpError::InvalidArticleNumber);
        }

        if !response.is_success() {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        // Pre-allocate: one entry per response line (minus failed parses)
        let mut entries = Vec::with_capacity(response.lines.len());
        for line in &response.lines {
            match commands::parse_hdr_line(line) {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    warn!("Failed to parse HDR line: {} - {}", line, e);
                    continue;
                }
            }
        }

        Ok(entries)
    }
}

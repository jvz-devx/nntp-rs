use crate::{codes, commands, error::NntpError, NntpClient, Result};
use tracing::debug;

impl NntpClient {
    /// Select a newsgroup
    ///
    /// Returns [`GroupInfo`](crate::commands::GroupInfo) with article count and range.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::NoSuchGroup`] - The newsgroup does not exist
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::InvalidResponse`] - Could not parse the server response
    pub async fn select_group(&mut self, newsgroup: &str) -> Result<commands::GroupInfo> {
        debug!("Selecting newsgroup: {}", newsgroup);

        let cmd = commands::group(newsgroup);
        self.send_command(&cmd).await?;
        let response = self.read_response().await?;

        if response.code == codes::NO_SUCH_GROUP {
            return Err(NntpError::NoSuchGroup(newsgroup.to_string()));
        }

        let info = commands::parse_group_response(response)?;
        self.current_group = Some(newsgroup.to_string());

        debug!(
            "Group {} selected: {} articles ({}-{})",
            newsgroup, info.count, info.first, info.last
        );
        Ok(info)
    }

    /// List article numbers in a newsgroup (RFC 3977 Section 6.1.2)
    ///
    /// Returns a list of article numbers currently available in the specified newsgroup.
    /// Optionally accepts a range parameter to limit the returned article numbers.
    ///
    /// This command is useful for:
    /// - Getting all article numbers in a group
    /// - Finding which articles exist in a range
    /// - Checking article availability before downloading
    ///
    /// # Arguments
    ///
    /// * `newsgroup` - The newsgroup to list articles from
    /// * `range` - Optional range specification (e.g., "100-200", "100-", "-200")
    ///
    /// # Returns
    ///
    /// A vector of article numbers available in the group
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::{NntpClient, ServerConfig};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ServerConfig::tls("news.example.com", "user", "pass");
    /// # let mut client = NntpClient::connect(Arc::new(config)).await?;
    /// // List all article numbers
    /// let articles = client.listgroup("alt.binaries.test", None).await?;
    /// println!("Found {} articles", articles.len());
    ///
    /// // List articles in a specific range
    /// let recent = client.listgroup("alt.binaries.test", Some("1000-2000")).await?;
    /// println!("Found {} articles in range 1000-2000", recent.len());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::NoSuchGroup`] - The newsgroup does not exist
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn listgroup(&mut self, newsgroup: &str, range: Option<&str>) -> Result<Vec<u64>> {
        debug!("Listing articles in group: {}", newsgroup);

        let cmd = match range {
            Some(r) => commands::listgroup_range(newsgroup, r),
            None => commands::listgroup(newsgroup),
        };

        self.send_command(&cmd).await?;
        let response = self.read_multiline_response().await?;

        if response.code == codes::NO_SUCH_GROUP {
            return Err(NntpError::NoSuchGroup(newsgroup.to_string()));
        }

        if response.code != codes::GROUP_SELECTED {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        // Parse article numbers from multiline response
        let mut articles = Vec::new();
        for line in &response.lines {
            if let Ok(num) = line.trim().parse::<u64>() {
                articles.push(num);
            }
        }

        debug!("Found {} articles in group {}", articles.len(), newsgroup);
        Ok(articles)
    }
}

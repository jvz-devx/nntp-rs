//! LIST command variants and parsing

use crate::error::{NntpError, Result};
use crate::response::NntpResponse;

/// Build LIST command
///
/// Lists available newsgroups on the server.
///
/// Intentionally unused (RFC 3977 §7.6.1 API completeness): This is the base LIST command
/// without variant specification. Most clients use LIST ACTIVE or other variants instead,
/// but this function is provided for complete RFC 3977 compliance. May be needed for
/// interoperability with legacy servers or strict protocol testing.
pub fn list() -> &'static str {
    "LIST\r\n"
}

/// Build LIST ACTIVE command (RFC 3977 §7.6.3)
///
/// Lists active newsgroups matching the wildmat pattern.
pub fn list_active(wildmat: &str) -> String {
    format!("LIST ACTIVE {}\r\n", wildmat)
}

/// Build LIST ACTIVE.TIMES command (RFC 3977 §7.6.4)
///
/// Lists newsgroup creation times.
pub fn list_active_times(wildmat: &str) -> String {
    format!("LIST ACTIVE.TIMES {}\r\n", wildmat)
}

/// Build LIST NEWSGROUPS command (RFC 3977 §7.6.6)
///
/// Lists newsgroup descriptions.
pub fn list_newsgroups(wildmat: &str) -> String {
    format!("LIST NEWSGROUPS {}\r\n", wildmat)
}

/// Build LIST COUNTS command (RFC 6048 §3)
///
/// Lists newsgroups with estimated article counts.
pub fn list_counts(wildmat: &str) -> String {
    format!("LIST COUNTS {}\r\n", wildmat)
}

/// Build LIST DISTRIBUTIONS command (RFC 6048 §4)
///
/// Lists valid distribution names and descriptions.
/// No wildmat argument is permitted.
pub fn list_distributions() -> &'static str {
    "LIST DISTRIBUTIONS\r\n"
}

/// Build LIST MODERATORS command (RFC 6048 Section 5)
///
/// Lists submission address templates for moderated newsgroups.
/// No arguments are permitted.
pub fn list_moderators() -> &'static str {
    "LIST MODERATORS\r\n"
}

/// Build LIST MOTD command (RFC 6048 Section 6)
///
/// Retrieves the server's message of the day.
/// No arguments are permitted.
pub fn list_motd() -> &'static str {
    "LIST MOTD\r\n"
}

/// Build LIST SUBSCRIPTIONS command (RFC 6048 Section 7)
///
/// Returns a list of newsgroups recommended for new users to subscribe to.
/// This represents the default subscription list for the server.
pub fn list_subscriptions() -> &'static str {
    "LIST SUBSCRIPTIONS\r\n"
}

/// Build LISTGROUP command (RFC 3977 Section 6.1.2)
///
/// Returns a list of article numbers in the specified newsgroup.
pub fn listgroup(newsgroup: &str) -> String {
    format!("LISTGROUP {}\r\n", newsgroup)
}

/// Build LISTGROUP command with range (RFC 3977 Section 6.1.2)
///
/// Returns article numbers in the specified newsgroup within the given range.
pub fn listgroup_range(newsgroup: &str, range: &str) -> String {
    format!("LISTGROUP {} {}\r\n", newsgroup, range)
}

/// Build NEWNEWS command (RFC 3977 §7.4)
///
/// Lists message-IDs of articles posted since the specified date/time.
/// Format: NEWNEWS wildmat yyyymmdd hhmmss
pub fn newnews(wildmat: &str, date: &str, time: &str) -> String {
    format!("NEWNEWS {} {} {}\r\n", wildmat, date, time)
}

/// Build NEWNEWS command with GMT (RFC 3977 §7.4)
pub fn newnews_gmt(wildmat: &str, date: &str, time: &str) -> String {
    format!("NEWNEWS {} {} {} GMT\r\n", wildmat, date, time)
}

/// Parse LIST ACTIVE response into ActiveGroup entries
///
/// Format: "group high low status"
/// Example: "comp.lang.rust 12345 1000 y"
/// Extended example: "alt.binaries.spam 0 0 j" (RFC 6048)
/// Alias example: "comp.lang.c++ 100 1 =comp.lang.cplusplus" (RFC 6048)
pub fn parse_list_active_response(
    response: NntpResponse,
) -> Result<Vec<crate::commands::group::ActiveGroup>> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message,
        });
    }

    let mut groups = Vec::new();
    for line in &response.lines {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue; // Skip malformed lines
        }

        let high = parts[1].parse().unwrap_or(0);
        let low = parts[2].parse().unwrap_or(0);
        // Status can be multi-character (e.g., "=group.name" alias)
        let status = parts[3].to_string();

        groups.push(crate::commands::group::ActiveGroup {
            name: parts[0].to_string(),
            high,
            low,
            status,
        });
    }

    Ok(groups)
}

/// Newsgroup entry from LIST COUNTS (RFC 6048 Section 3)
#[derive(Debug, Clone)]
pub struct CountsGroup {
    /// Newsgroup name
    pub name: String,
    /// Estimated article count
    pub count: u64,
    /// Lowest article number
    pub low: u64,
    /// Highest article number
    pub high: u64,
    /// Posting status:
    /// - "y" = posting allowed
    /// - "n" = posting not allowed
    /// - "m" = moderated
    /// - "j" = junk/spam group (RFC 6048)
    /// - "x" = no local posting (RFC 6048)
    /// - "=group.name" = alias to another group (RFC 6048)
    pub status: String,
}

/// Parse LIST COUNTS response into CountsGroup entries (RFC 6048 Section 3)
///
/// Format: "group count low high status"
/// Example: "comp.lang.rust 1234 1000 12345 y"
pub fn parse_list_counts_response(response: NntpResponse) -> Result<Vec<CountsGroup>> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message,
        });
    }

    let mut groups = Vec::new();
    for line in &response.lines {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            continue; // Skip malformed lines
        }

        let count = parts[1].parse().unwrap_or(0);
        let low = parts[2].parse().unwrap_or(0);
        let high = parts[3].parse().unwrap_or(0);
        // Status can be multi-character (e.g., "=group.name" alias)
        let status = parts[4].to_string();

        groups.push(CountsGroup {
            name: parts[0].to_string(),
            count,
            low,
            high,
            status,
        });
    }

    Ok(groups)
}

/// Distribution information from LIST DISTRIBUTIONS (RFC 6048 Section 4)
#[derive(Debug, Clone)]
pub struct DistributionInfo {
    /// Distribution name (e.g., "local", "usa", "fr")
    pub name: String,
    /// Short description of the distribution area
    pub description: String,
}

/// Parse LIST DISTRIBUTIONS response into DistributionInfo entries (RFC 6048 Section 4)
///
/// Format: "distribution description"
/// Example: "usa Local to the United States of America."
pub fn parse_list_distributions_response(response: NntpResponse) -> Result<Vec<DistributionInfo>> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message,
        });
    }

    let mut distributions = Vec::new();
    for line in &response.lines {
        // Split on first whitespace only - description may contain spaces
        if let Some(space_pos) = line.find(|c: char| c.is_whitespace()) {
            let name = line[..space_pos].to_string();
            let description = line[space_pos..].trim().to_string();

            distributions.push(DistributionInfo { name, description });
        }
        // Skip lines without a space separator
    }

    Ok(distributions)
}

/// Moderator information from LIST MODERATORS (RFC 6048 Section 5)
#[derive(Debug, Clone)]
pub struct ModeratorInfo {
    /// Wildmat pattern or newsgroup name (e.g., "local.*", "foo.bar")
    pub pattern: String,
    /// Submission address template (e.g., "%s@moderators.example.com", "announce@example.com")
    /// %s is replaced with newsgroup name (periods converted to dashes)
    /// %% represents a literal % character
    pub address: String,
}

/// Parse LIST MODERATORS response into ModeratorInfo entries (RFC 6048 Section 5)
///
/// Format: "pattern:address"
/// The pattern and address are separated by a colon with no spaces.
///
/// # Examples
///
/// ```
/// # use nntp_rs::{commands, codes, NntpResponse};
/// let response = NntpResponse {
///     code: codes::LIST_INFORMATION_FOLLOWS,
///     message: "List of submission address templates follows".to_string(),
///     lines: vec![
///         "foo.bar:announce@example.com".to_string(),
///         "local.*:%s@localhost".to_string(),
///         "*:%s@moderators.example.com".to_string(),
///     ],
/// };
///
/// let moderators = commands::parse_list_moderators_response(response).unwrap();
/// assert_eq!(moderators.len(), 3);
/// assert_eq!(moderators[0].pattern, "foo.bar");
/// assert_eq!(moderators[0].address, "announce@example.com");
/// assert_eq!(moderators[1].pattern, "local.*");
/// assert_eq!(moderators[1].address, "%s@localhost");
/// ```
pub fn parse_list_moderators_response(response: NntpResponse) -> Result<Vec<ModeratorInfo>> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message,
        });
    }

    let mut moderators = Vec::new();
    for line in &response.lines {
        // Split on first colon - address may contain colons (e.g., in IPv6)
        if let Some(colon_pos) = line.find(':') {
            let pattern = line[..colon_pos].to_string();
            let address = line[colon_pos + 1..].to_string();

            moderators.push(ModeratorInfo { pattern, address });
        }
        // Skip lines without a colon separator
    }

    Ok(moderators)
}

/// Parse LIST MOTD response into list of text lines (RFC 6048 Section 6)
///
/// Response format: 215 followed by message of the day text (multiline)
/// Returns a vector of text lines representing the server's message of the day.
///
/// # Example
///
/// ```text
/// 215 Message of the day follows
/// Welcome to our NNTP server!
/// Server maintenance scheduled for midnight.
/// Contact admin@example.com for support.
/// .
/// ```
pub fn parse_list_motd_response(response: NntpResponse) -> Result<Vec<String>> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message,
        });
    }

    // Simply return all lines from the multiline response
    // Empty lines are preserved as they may be part of the formatted MOTD
    Ok(response.lines)
}

/// Parse LIST SUBSCRIPTIONS response into list of newsgroup names (RFC 6048 Section 7)
///
/// Response format: 215 followed by list of newsgroup names (one per line)
/// Example:
/// ```text
/// 215 Default subscription list follows
/// comp.lang.rust
/// comp.programming
/// news.announce.newusers
/// .
/// ```
pub fn parse_list_subscriptions_response(response: NntpResponse) -> Result<Vec<String>> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message,
        });
    }

    // Filter out empty lines and return newsgroup names
    Ok(response
        .lines
        .into_iter()
        .filter(|line| !line.is_empty())
        .collect())
}

/// Parse NEWNEWS response into list of message-IDs (RFC 3977 Section 7.4)
///
/// Response format: 230 followed by list of message-IDs (one per line)
/// Example:
/// ```text
/// 230 List of new articles follows
/// <abc@example.com>
/// <def@example.com>
/// .
/// ```
pub fn parse_newnews_response(response: NntpResponse) -> Result<Vec<String>> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message,
        });
    }

    let message_ids: Vec<String> = response
        .lines
        .into_iter()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();

    Ok(message_ids)
}

/// Newsgroup information from LIST NEWSGROUPS (RFC 3977 Section 7.6.6)
#[derive(Debug, Clone)]
pub struct NewsgroupInfo {
    /// Newsgroup name
    pub name: String,
    /// Newsgroup description
    pub description: String,
}

/// Parse LIST NEWSGROUPS response into NewsgroupInfo entries
///
/// Format: "group description text"
/// Example: "comp.lang.rust The Rust programming language"
pub fn parse_list_newsgroups_response(response: NntpResponse) -> Result<Vec<NewsgroupInfo>> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message,
        });
    }

    let mut groups = Vec::new();
    for line in &response.lines {
        // Split on first whitespace only - description may contain spaces
        if let Some(space_pos) = line.find(|c: char| c.is_whitespace()) {
            let name = line[..space_pos].to_string();
            let description = line[space_pos..].trim().to_string();

            groups.push(NewsgroupInfo { name, description });
        }
        // Skip lines without a space separator
    }

    Ok(groups)
}

/// Newsgroup creation time information from LIST ACTIVE.TIMES (RFC 3977 Section 7.6.4)
#[derive(Debug, Clone)]
pub struct GroupTime {
    /// Newsgroup name
    pub name: String,
    /// Creation timestamp (Unix timestamp in seconds)
    pub timestamp: u64,
    /// Creator identifier (typically email or username)
    pub creator: String,
}

/// Parse LIST ACTIVE.TIMES response into GroupTime entries
///
/// Format: "group timestamp creator"
/// Example: "comp.lang.rust 1234567890 user@example.com"
pub fn parse_list_active_times_response(response: NntpResponse) -> Result<Vec<GroupTime>> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message,
        });
    }

    let mut groups = Vec::new();
    for line in &response.lines {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue; // Skip malformed lines
        }

        let timestamp = parts[1].parse().unwrap_or(0);

        groups.push(GroupTime {
            name: parts[0].to_string(),
            timestamp,
            creator: parts[2].to_string(),
        });
    }

    Ok(groups)
}

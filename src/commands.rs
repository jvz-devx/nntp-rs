//! NNTP command builders and response parsers

use crate::error::{NntpError, Result};
use crate::response::NntpResponse;

/// Parse NNTP response line into code and message
pub fn parse_response_line(line: &str) -> Result<(u16, String)> {
    // Check minimum length and that first 3 chars are ASCII digits
    let bytes = line.as_bytes();
    if bytes.len() < 3
        || !bytes[0].is_ascii_digit()
        || !bytes[1].is_ascii_digit()
        || !bytes[2].is_ascii_digit()
    {
        return Err(NntpError::InvalidResponse(line.chars().take(100).collect()));
    }

    // Safe to slice since we verified ASCII
    let code = line[0..3]
        .parse::<u16>()
        .map_err(|_| NntpError::InvalidResponse(line.chars().take(100).collect()))?;

    let message = if line.len() > 4 {
        line[4..].to_string()
    } else {
        String::new()
    };

    Ok((code, message))
}

/// Parse single-line NNTP response
pub fn parse_single_response(line: &str) -> Result<NntpResponse> {
    let (code, message) = parse_response_line(line)?;

    Ok(NntpResponse {
        code,
        message,
        lines: vec![],
    })
}

/// Build AUTHINFO USER command
pub fn authinfo_user(username: &str) -> String {
    format!("AUTHINFO USER {}\r\n", username)
}

/// Build AUTHINFO PASS command
pub fn authinfo_pass(password: &str) -> String {
    format!("AUTHINFO PASS {}\r\n", password)
}

/// Build GROUP command
pub fn group(newsgroup: &str) -> String {
    format!("GROUP {}\r\n", newsgroup)
}

/// Build ARTICLE command
pub fn article(id: &str) -> String {
    format!("ARTICLE {}\r\n", id)
}

/// Build HEAD command
pub fn head(id: &str) -> String {
    format!("HEAD {}\r\n", id)
}

/// Build BODY command
pub fn body(id: &str) -> String {
    format!("BODY {}\r\n", id)
}

/// Build XOVER command for fetching article overview data
pub fn xover(range: &str) -> String {
    format!("XOVER {}\r\n", range)
}

/// Build COMPRESS DEFLATE command (RFC 8054)
///
/// Enables full session compression using deflate algorithm.
/// All data after successful negotiation (response 206) is compressed bidirectionally.
/// This provides the best compression but is not supported by all NNTP servers.
pub fn compress_deflate() -> String {
    "COMPRESS DEFLATE\r\n".to_string()
}

/// Build XFEATURE COMPRESS GZIP command
///
/// Enables headers-only compression using gzip/zlib.
/// Only multiline responses (XOVER, HEAD, ARTICLE) are compressed after
/// successful negotiation (response 290 or 2xx). More widely supported than
/// RFC 8054 COMPRESS DEFLATE. Provides 50-80% bandwidth reduction.
pub fn xfeature_compress_gzip() -> String {
    "XFEATURE COMPRESS GZIP\r\n".to_string()
}

/// Build QUIT command
pub fn quit() -> String {
    "QUIT\r\n".to_string()
}

/// Build LIST command
///
/// Lists available newsgroups on the server.
#[allow(dead_code)] // Part of public API, available for future use
pub fn list() -> String {
    "LIST\r\n".to_string()
}

/// Build STAT command (RFC 3977 §6.2.4)
///
/// Gets article status without retrieving content.
/// Can be used with article number or message-id.
pub fn stat(id: &str) -> String {
    format!("STAT {}\r\n", id)
}

// RFC 3977 Additional Commands

/// Build POST command (RFC 3977 §6.3.1)
///
/// Initiates article posting. Server responds with 340 if ready to accept.
/// After receiving 340, client sends article terminated by ".\r\n".
pub fn post() -> String {
    "POST\r\n".to_string()
}

/// Build IHAVE command (RFC 3977 §6.2.1)
///
/// Offers an article for transfer by message-id.
/// Server responds with 335 if it wants the article, 435/436 if not.
#[allow(dead_code)]
pub fn ihave(message_id: &str) -> String {
    format!("IHAVE {}\r\n", message_id)
}

/// Build MODE READER command (RFC 3977 §5.3)
///
/// Instructs the server to switch to reader mode (for news reading clients).
#[allow(dead_code)]
pub fn mode_reader() -> String {
    "MODE READER\r\n".to_string()
}

/// Build MODE STREAM command (RFC 3977 §5.3)
///
/// Instructs the server to switch to streaming mode (for news transfer).
/// Build MODE STREAM command (RFC 4644 Section 2.3)
///
/// Requests to switch to streaming mode for efficient bulk article transfer.
/// Response is 203 on success.
pub fn mode_stream() -> String {
    "MODE STREAM\r\n".to_string()
}

/// Build CHECK command (RFC 4644 Section 2.4)
///
/// Checks if the server wants an article by message-id in streaming mode.
/// Server responds with:
/// - 238 (CHECK_SEND) - Send the article via TAKETHIS
/// - 431 (CHECK_LATER) - Try again later
/// - 438 (CHECK_NOT_WANTED) - Article not wanted
///
/// The response includes the message-id for matching in pipelined scenarios.
pub fn check(message_id: &str) -> String {
    format!("CHECK {}\r\n", message_id)
}

/// Build TAKETHIS command with article data (RFC 4644 §2.5)
///
/// Sends an article to the server in streaming mode without waiting for permission.
/// The article data is sent immediately after the command line.
///
/// Server responds with:
/// - 239 (TAKETHIS_RECEIVED) - Article received successfully
/// - 439 (TAKETHIS_REJECTED) - Article rejected, do not retry
///
/// The response includes the message-id for matching in pipelined scenarios.
///
/// Note: The article data must be formatted with CRLF line endings and dot-stuffing.
/// Use `Article::serialize_for_posting()` to prepare article data.
pub fn takethis(message_id: &str, article_data: &str) -> String {
    format!("TAKETHIS {}\r\n{}", message_id, article_data)
}

/// Build CAPABILITIES command (RFC 3977 §5.2)
///
/// Requests the list of capabilities supported by the server.
/// Response is multi-line, starting with 101.
#[allow(dead_code)]
pub fn capabilities() -> String {
    "CAPABILITIES\r\n".to_string()
}

/// Build CAPABILITIES command with keyword (RFC 3977 §5.2)
///
/// Requests capabilities with optional keyword for specific capability info.
#[allow(dead_code)]
pub fn capabilities_with_keyword(keyword: &str) -> String {
    format!("CAPABILITIES {}\r\n", keyword)
}

/// Build HELP command (RFC 3977 §7.2)
///
/// Requests help text from the server. Response is multi-line, starting with 100.
#[allow(dead_code)]
pub fn help() -> String {
    "HELP\r\n".to_string()
}

/// Build DATE command (RFC 3977 §7.1)
///
/// Requests the server's current date and time.
/// Response: 111 yyyymmddhhmmss
#[allow(dead_code)]
pub fn date() -> String {
    "DATE\r\n".to_string()
}

/// Build NEWGROUPS command (RFC 3977 §7.3)
///
/// Lists newsgroups created since the specified date/time.
/// Format: NEWGROUPS yyyymmdd hhmmss [GMT]
pub fn newgroups(date: &str, time: &str) -> String {
    format!("NEWGROUPS {} {}\r\n", date, time)
}

/// Build NEWGROUPS command with GMT (RFC 3977 §7.3)
pub fn newgroups_gmt(date: &str, time: &str) -> String {
    format!("NEWGROUPS {} {} GMT\r\n", date, time)
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

/// Build NEXT command (RFC 3977 §6.1.4)
///
/// Moves to the next article in the current group.
pub fn next() -> String {
    "NEXT\r\n".to_string()
}

/// Build LAST command (RFC 3977 §6.1.3)
///
/// Moves to the previous article in the current group.
pub fn last() -> String {
    "LAST\r\n".to_string()
}

/// Build HDR command (RFC 3977 §8.5)
///
/// Retrieves specific header fields from articles.
/// Format: HDR header-name [range|message-id]
pub fn hdr(header: &str, range: &str) -> String {
    format!("HDR {} {}\r\n", header, range)
}

/// Build HDR command for current article (RFC 3977 §8.5)
pub fn hdr_current(header: &str) -> String {
    format!("HDR {}\r\n", header)
}

/// Build OVER command (RFC 3977 §8.3)
///
/// Retrieves overview data (same as XOVER but RFC 3977 standard name).
pub fn over(range: &str) -> String {
    format!("OVER {}\r\n", range)
}

/// Build OVER command for current article
pub fn over_current() -> String {
    "OVER\r\n".to_string()
}

/// Build LIST ACTIVE command (RFC 3977 §7.6.3)
///
/// Lists active newsgroups matching the wildmat pattern.
#[allow(dead_code)]
pub fn list_active(wildmat: &str) -> String {
    format!("LIST ACTIVE {}\r\n", wildmat)
}

/// Build LIST ACTIVE.TIMES command (RFC 3977 §7.6.4)
///
/// Lists newsgroup creation times.
#[allow(dead_code)]
pub fn list_active_times(wildmat: &str) -> String {
    format!("LIST ACTIVE.TIMES {}\r\n", wildmat)
}

/// Build LIST NEWSGROUPS command (RFC 3977 §7.6.6)
///
/// Lists newsgroup descriptions.
#[allow(dead_code)]
pub fn list_newsgroups(wildmat: &str) -> String {
    format!("LIST NEWSGROUPS {}\r\n", wildmat)
}

/// Build LIST HEADERS command (RFC 3977 §8.6)
///
/// Lists header fields available for HDR command.
pub fn list_headers() -> String {
    "LIST HEADERS\r\n".to_string()
}

/// Build LIST HEADERS MSGID command (RFC 3977 §8.6)
///
/// Lists header fields available for HDR with message-id argument.
pub fn list_headers_msgid() -> String {
    "LIST HEADERS MSGID\r\n".to_string()
}

/// Build LIST HEADERS RANGE command (RFC 3977 §8.6)
///
/// Lists header fields available for HDR with range argument.
pub fn list_headers_range() -> String {
    "LIST HEADERS RANGE\r\n".to_string()
}

/// Build LIST OVERVIEW.FMT command (RFC 3977 §8.4)
///
/// Lists the format of overview data.
#[allow(dead_code)]
pub fn list_overview_fmt() -> String {
    "LIST OVERVIEW.FMT\r\n".to_string()
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
pub fn list_distributions() -> String {
    "LIST DISTRIBUTIONS\r\n".to_string()
}

/// Build LIST MODERATORS command (RFC 6048 Section 5)
///
/// Lists submission address templates for moderated newsgroups.
/// No arguments are permitted.
pub fn list_moderators() -> String {
    "LIST MODERATORS\r\n".to_string()
}

/// Build LIST MOTD command (RFC 6048 Section 6)
///
/// Retrieves the server's message of the day.
/// No arguments are permitted.
pub fn list_motd() -> String {
    "LIST MOTD\r\n".to_string()
}

/// Build LIST SUBSCRIPTIONS command (RFC 6048 Section 7)
///
/// Returns a list of newsgroups recommended for new users to subscribe to.
/// This represents the default subscription list for the server.
pub fn list_subscriptions() -> String {
    "LIST SUBSCRIPTIONS\r\n".to_string()
}

/// Build LISTGROUP command (RFC 3977 §6.1.2)
///
/// Lists article numbers in the specified group.
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

/// Build AUTHINFO SASL command (RFC 4643 §2.4)
///
/// Initiates SASL authentication with the specified mechanism.
pub fn authinfo_sasl(mechanism: &str) -> String {
    format!("AUTHINFO SASL {}\r\n", mechanism)
}

/// Build AUTHINFO SASL command with initial response (RFC 4643 §2.4)
pub fn authinfo_sasl_ir(mechanism: &str, initial_response: &str) -> String {
    format!("AUTHINFO SASL {} {}\r\n", mechanism, initial_response)
}

/// Build AUTHINFO SASL continuation response (RFC 4643 §2.4)
pub fn authinfo_sasl_continue(response: &str) -> String {
    format!("{}\r\n", response)
}

/// Build STARTTLS command (RFC 4642)
///
/// Initiates TLS negotiation on the connection.
#[allow(dead_code)]
pub fn starttls() -> String {
    "STARTTLS\r\n".to_string()
}

/// Parse GROUP response to extract article count and range
///
/// Response format: "211 count first last group-name"
pub fn parse_group_response(response: &NntpResponse) -> Result<(u64, u64, u64)> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message.clone(),
        });
    }

    let parts: Vec<&str> = response.message.split_whitespace().collect();
    if parts.len() < 3 {
        return Err(NntpError::InvalidResponse(response.message.clone()));
    }

    let count = parts[0]
        .parse()
        .map_err(|_| NntpError::InvalidResponse(response.message.clone()))?;
    let first = parts[1]
        .parse()
        .map_err(|_| NntpError::InvalidResponse(response.message.clone()))?;
    let last = parts[2]
        .parse()
        .map_err(|_| NntpError::InvalidResponse(response.message.clone()))?;

    Ok((count, first, last))
}

/// Parse STAT response (RFC 3977 §6.2.4)
///
/// Response format: "223 n message-id"
/// - n is the article number (0 if message-id was used in request)
/// - message-id is the article's message identifier
///
/// Returns tuple of (article_number, message_id)
pub fn parse_stat_response(response: &NntpResponse) -> Result<(u64, String)> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message.clone(),
        });
    }

    let parts: Vec<&str> = response.message.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(NntpError::InvalidResponse(response.message.clone()));
    }

    let article_number = parts[0]
        .parse()
        .map_err(|_| NntpError::InvalidResponse(response.message.clone()))?;

    // Message-ID is the rest of the message, joined in case it contains spaces
    // (though it shouldn't per RFC, but we handle it gracefully)
    let message_id = parts[1..].join(" ");

    Ok((article_number, message_id))
}

/// Parse NEXT response (RFC 3977 §6.1.4)
///
/// Response format: "223 n message-id"
/// - n is the article number
/// - message-id is the article's message identifier
///
/// Returns tuple of (article_number, message_id)
pub fn parse_next_response(response: &NntpResponse) -> Result<(u64, String)> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message.clone(),
        });
    }

    let parts: Vec<&str> = response.message.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(NntpError::InvalidResponse(response.message.clone()));
    }

    let article_number = parts[0]
        .parse()
        .map_err(|_| NntpError::InvalidResponse(response.message.clone()))?;

    // Message-ID is the rest of the message, joined in case it contains spaces
    // (though it shouldn't per RFC, but we handle it gracefully)
    let message_id = parts[1..].join(" ");

    Ok((article_number, message_id))
}

/// Parse response to LAST command (RFC 3977 §6.1.3)
///
/// Response format: "223 n message-id"
/// where n is the article number and message-id is the message identifier.
///
/// Returns tuple of (article_number, message_id)
pub fn parse_last_response(response: &NntpResponse) -> Result<(u64, String)> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message.clone(),
        });
    }

    let parts: Vec<&str> = response.message.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(NntpError::InvalidResponse(response.message.clone()));
    }

    let article_number = parts[0]
        .parse()
        .map_err(|_| NntpError::InvalidResponse(response.message.clone()))?;

    // Message-ID is the rest of the message, joined in case it contains spaces
    // (though it shouldn't per RFC, but we handle it gracefully)
    let message_id = parts[1..].join(" ");

    Ok((article_number, message_id))
}

/// HDR entry structure containing article number and header value
///
/// RFC 3977 Section 8.5 - HDR command response format
#[derive(Debug, Clone)]
pub struct HdrEntry {
    /// Article number within the newsgroup (0 if queried by message-id)
    pub article_number: u64,
    /// Header field value for this article
    pub value: String,
}

/// Parse HDR response line into HdrEntry
///
/// Format: "article-number header-value"
/// The article-number and header-value are separated by a space.
/// Header values may contain spaces, so everything after the first space is the value.
///
/// # Examples
///
/// ```
/// # use nntp_rs::commands::parse_hdr_line;
/// let entry = parse_hdr_line("12345 Re: Test Subject").unwrap();
/// assert_eq!(entry.article_number, 12345);
/// assert_eq!(entry.value, "Re: Test Subject");
/// ```
pub fn parse_hdr_line(line: &str) -> Result<HdrEntry> {
    let parts: Vec<&str> = line.splitn(2, ' ').collect();
    if parts.len() < 2 {
        return Err(NntpError::InvalidResponse(line.to_string()));
    }

    let article_number = parts[0]
        .parse()
        .map_err(|_| NntpError::InvalidResponse(line.to_string()))?;

    Ok(HdrEntry {
        article_number,
        value: parts[1].to_string(),
    })
}

/// Parse HDR response into HdrEntry list
///
/// RFC 3977 Section 8.5 - Response code 225 with multiline data
pub fn parse_hdr_response(response: &NntpResponse) -> Result<Vec<HdrEntry>> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message.clone(),
        });
    }

    let mut entries = Vec::new();
    for line in &response.lines {
        match parse_hdr_line(line) {
            Ok(entry) => entries.push(entry),
            Err(_) => {
                // Skip malformed lines
                continue;
            }
        }
    }

    Ok(entries)
}

/// Active newsgroup entry from LIST ACTIVE (RFC 3977 Section 7.6.3, RFC 6048 Section 3)
#[derive(Debug, Clone)]
pub struct ActiveGroup {
    /// Newsgroup name
    pub name: String,
    /// Highest article number
    pub high: u64,
    /// Lowest article number
    pub low: u64,
    /// Posting status:
    /// - "y" = posting allowed
    /// - "n" = posting not allowed
    /// - "m" = moderated
    /// - "j" = junk/spam group (RFC 6048)
    /// - "x" = no local posting (RFC 6048)
    /// - "=group.name" = alias to another group (RFC 6048)
    pub status: String,
}

/// Parse LIST ACTIVE response into ActiveGroup entries
///
/// Format: "group high low status"
/// Example: "comp.lang.rust 12345 1000 y"
/// Extended example: "alt.binaries.spam 0 0 j" (RFC 6048)
/// Alias example: "comp.lang.c++ 100 1 =comp.lang.cplusplus" (RFC 6048)
pub fn parse_list_active_response(response: &NntpResponse) -> Result<Vec<ActiveGroup>> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message.clone(),
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

        groups.push(ActiveGroup {
            name: parts[0].to_string(),
            high,
            low,
            status,
        });
    }

    Ok(groups)
}

/// Parse NEWGROUPS response into ActiveGroup entries (RFC 3977 Section 7.3)
///
/// NEWGROUPS returns the same format as LIST ACTIVE: "group high low status"
/// Example: "comp.lang.rust 12345 1000 y"
pub fn parse_newgroups_response(response: &NntpResponse) -> Result<Vec<ActiveGroup>> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message.clone(),
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

        groups.push(ActiveGroup {
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
pub fn parse_list_counts_response(response: &NntpResponse) -> Result<Vec<CountsGroup>> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message.clone(),
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
pub fn parse_list_distributions_response(response: &NntpResponse) -> Result<Vec<DistributionInfo>> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message.clone(),
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
/// let moderators = commands::parse_list_moderators_response(&response).unwrap();
/// assert_eq!(moderators.len(), 3);
/// assert_eq!(moderators[0].pattern, "foo.bar");
/// assert_eq!(moderators[0].address, "announce@example.com");
/// assert_eq!(moderators[1].pattern, "local.*");
/// assert_eq!(moderators[1].address, "%s@localhost");
/// ```
pub fn parse_list_moderators_response(response: &NntpResponse) -> Result<Vec<ModeratorInfo>> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message.clone(),
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
pub fn parse_list_motd_response(response: &NntpResponse) -> Result<Vec<String>> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message.clone(),
        });
    }

    // Simply return all lines from the multiline response
    // Empty lines are preserved as they may be part of the formatted MOTD
    Ok(response.lines.clone())
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
pub fn parse_list_subscriptions_response(response: &NntpResponse) -> Result<Vec<String>> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message.clone(),
        });
    }

    // Filter out empty lines and return newsgroup names
    Ok(response
        .lines
        .iter()
        .filter(|line| !line.is_empty())
        .cloned()
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
pub fn parse_newnews_response(response: &NntpResponse) -> Result<Vec<String>> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message.clone(),
        });
    }

    let message_ids: Vec<String> = response
        .lines
        .iter()
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
pub fn parse_list_newsgroups_response(response: &NntpResponse) -> Result<Vec<NewsgroupInfo>> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message.clone(),
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
pub fn parse_list_active_times_response(response: &NntpResponse) -> Result<Vec<GroupTime>> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message.clone(),
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

/// Parse LIST OVERVIEW.FMT response into field names
///
/// Format: One field name per line, in order of OVER/XOVER output
/// Example lines: "Subject:", "From:", ":bytes", "Xref:full"
///
/// RFC 3977 Section 8.4
pub fn parse_list_overview_fmt_response(response: &NntpResponse) -> Result<Vec<String>> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message.clone(),
        });
    }

    // Each line is a field name - return as-is for maximum compatibility
    // The caller can parse colons and metadata markers as needed
    Ok(response.lines.clone())
}

/// Parse LIST HEADERS response (RFC 3977 §8.6)
///
/// Returns a list of header field names available for the HDR command.
/// Each line is a field name (e.g., "Subject", "From", ":lines", ":bytes").
/// A special entry ":" means any header may be retrieved.
pub fn parse_list_headers_response(response: &NntpResponse) -> Result<Vec<String>> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message.clone(),
        });
    }

    // Each line is a field name - return as-is
    // Special case: ":" means any header can be retrieved
    Ok(response.lines.clone())
}

/// XOVER entry structure containing article metadata
#[derive(Debug, Clone)]
pub struct XoverEntry {
    /// Article number within the newsgroup
    pub article_number: u64,
    /// Article subject line
    pub subject: String,
    /// Article author (From header)
    pub author: String,
    /// Article date string
    pub date: String,
    /// Unique message ID
    pub message_id: String,
    /// References to parent articles (for threading)
    pub references: String,
    /// Article size in bytes
    pub bytes: usize,
    /// Number of lines in the article
    pub lines: usize,
}

/// Parse XOVER response line into components
///
/// Format: "article-number\tsubject\tauthor\tdate\tmessage-id\treferences\tbytes\tlines\txref"
pub fn parse_xover_line(line: &str) -> Result<XoverEntry> {
    let parts: Vec<&str> = line.split('\t').collect();
    if parts.len() < 8 {
        return Err(NntpError::InvalidResponse(line.to_string()));
    }

    Ok(XoverEntry {
        article_number: parts[0].parse().unwrap_or(0),
        subject: parts[1].to_string(),
        author: parts[2].to_string(),
        date: parts[3].to_string(),
        message_id: parts[4].to_string(),
        references: parts[5].to_string(),
        bytes: parts[6].parse().unwrap_or(0),
        lines: parts[7].parse().unwrap_or(0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_response_line() {
        let (code, msg) = parse_response_line("200 server ready").unwrap();
        assert_eq!(code, 200);
        assert_eq!(msg, "server ready");

        let (code, msg) = parse_response_line("281 Authentication accepted").unwrap();
        assert_eq!(code, 281);
        assert_eq!(msg, "Authentication accepted");
    }

    #[test]
    fn test_parse_response_line_invalid() {
        assert!(parse_response_line("abc").is_err());
        assert!(parse_response_line("").is_err());
        assert!(parse_response_line("12").is_err());
    }

    #[test]
    fn test_parse_group_response() {
        let response = NntpResponse {
            code: 211,
            message: "3000 1 3000 free.pt".to_string(),
            lines: vec![],
        };

        let (count, first, last) = parse_group_response(&response).unwrap();
        assert_eq!(count, 3000);
        assert_eq!(first, 1);
        assert_eq!(last, 3000);
    }

    #[test]
    fn test_command_builders() {
        assert_eq!(authinfo_user("testuser"), "AUTHINFO USER testuser\r\n");
        assert_eq!(authinfo_pass("testpass"), "AUTHINFO PASS testpass\r\n");
        assert_eq!(group("free.pt"), "GROUP free.pt\r\n");
        assert_eq!(article("<123@example>"), "ARTICLE <123@example>\r\n");
        assert_eq!(head("<123@example>"), "HEAD <123@example>\r\n");
        assert_eq!(body("<123@example>"), "BODY <123@example>\r\n");
        assert_eq!(xover("1-100"), "XOVER 1-100\r\n");
        assert_eq!(compress_deflate(), "COMPRESS DEFLATE\r\n");
        assert_eq!(xfeature_compress_gzip(), "XFEATURE COMPRESS GZIP\r\n");
        assert_eq!(quit(), "QUIT\r\n");
    }

    #[test]
    fn test_parse_xover_line() {
        let line = "12345\tTest Subject\tauthor@example.com\tMon, 01 Jan 2024\t<msg@id>\t<ref@id>\t1234\t50";
        let entry = parse_xover_line(line).unwrap();

        assert_eq!(entry.article_number, 12345);
        assert_eq!(entry.subject, "Test Subject");
        assert_eq!(entry.author, "author@example.com");
        assert_eq!(entry.message_id, "<msg@id>");
        assert_eq!(entry.bytes, 1234);
        assert_eq!(entry.lines, 50);
    }
}

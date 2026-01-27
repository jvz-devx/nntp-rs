//! HDR command and header field retrieval

use crate::error::{NntpError, Result};
use crate::response::NntpResponse;

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

/// Build LIST HEADERS command (RFC 3977 §8.6)
///
/// Lists header fields available for HDR command.
pub fn list_headers() -> &'static str {
    "LIST HEADERS\r\n"
}

/// Build LIST HEADERS MSGID command (RFC 3977 §8.6)
///
/// Lists header fields available for HDR with message-id argument.
pub fn list_headers_msgid() -> &'static str {
    "LIST HEADERS MSGID\r\n"
}

/// Build LIST HEADERS RANGE command (RFC 3977 §8.6)
///
/// Lists header fields available for HDR with range argument.
pub fn list_headers_range() -> &'static str {
    "LIST HEADERS RANGE\r\n"
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
pub fn parse_hdr_response(response: NntpResponse) -> Result<Vec<HdrEntry>> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message,
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

/// Parse LIST HEADERS response (RFC 3977 §8.6)
///
/// Returns a list of header field names available for the HDR command.
/// Each line is a field name (e.g., "Subject", "From", ":lines", ":bytes").
/// A special entry ":" means any header may be retrieved.
pub fn parse_list_headers_response(response: NntpResponse) -> Result<Vec<String>> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message,
        });
    }

    // Each line is a field name - return as-is
    // Special case: ":" means any header can be retrieved
    Ok(response.lines)
}

//! OVER/XOVER commands and overview data parsing

use crate::error::{NntpError, Result};
use crate::response::NntpResponse;

/// Build XOVER command for fetching article overview data
pub fn xover(range: &str) -> String {
    format!("XOVER {}\r\n", range)
}

/// Build OVER command (RFC 3977 §8.3)
///
/// Retrieves overview data (same as XOVER but RFC 3977 standard name).
pub fn over(range: &str) -> String {
    format!("OVER {}\r\n", range)
}

/// Build OVER command for current article
pub fn over_current() -> &'static str {
    "OVER\r\n"
}

/// Build LIST OVERVIEW.FMT command (RFC 3977 §8.4)
///
/// Lists the format of overview data.
pub fn list_overview_fmt() -> &'static str {
    "LIST OVERVIEW.FMT\r\n"
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

/// Parse LIST OVERVIEW.FMT response into field names
///
/// Format: One field name per line, in order of OVER/XOVER output
/// Example lines: "Subject:", "From:", ":bytes", "Xref:full"
///
/// RFC 3977 Section 8.4
pub fn parse_list_overview_fmt_response(response: NntpResponse) -> Result<Vec<String>> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message,
        });
    }

    // Each line is a field name - return as-is for maximum compatibility
    // The caller can parse colons and metadata markers as needed
    Ok(response.lines)
}

#[cfg(test)]
mod tests {
    use super::*;

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

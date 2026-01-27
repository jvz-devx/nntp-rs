//! Group selection and newsgroup-related commands

use crate::error::{NntpError, Result};
use crate::response::NntpResponse;

/// Build GROUP command
pub fn group(newsgroup: &str) -> String {
    format!("GROUP {}\r\n", newsgroup)
}

/// Build NEWGROUPS command (RFC 3977 §7.3)
///
/// Lists newsgroups created since the specified date/time.
/// Format: `NEWGROUPS yyyymmdd hhmmss [GMT]`
pub fn newgroups(date: &str, time: &str) -> String {
    format!("NEWGROUPS {} {}\r\n", date, time)
}

/// Build NEWGROUPS command with GMT (RFC 3977 §7.3)
pub fn newgroups_gmt(date: &str, time: &str) -> String {
    format!("NEWGROUPS {} {} GMT\r\n", date, time)
}

/// Group information returned by the GROUP command
///
/// Contains article count and range information for a newsgroup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupInfo {
    /// Number of articles in the group
    pub count: u64,
    /// Number of the first article
    pub first: u64,
    /// Number of the last article
    pub last: u64,
}

/// Parse GROUP response to extract article count and range
///
/// Response format: "211 count first last group-name"
pub fn parse_group_response(response: NntpResponse) -> Result<GroupInfo> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message,
        });
    }

    let parts: Vec<&str> = response.message.split_whitespace().collect();
    if parts.len() < 3 {
        return Err(NntpError::InvalidResponse(response.message));
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

    Ok(GroupInfo { count, first, last })
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

/// Parse NEWGROUPS response into ActiveGroup entries (RFC 3977 Section 7.3)
///
/// NEWGROUPS returns the same format as LIST ACTIVE: "group high low status"
/// Example: "comp.lang.rust 12345 1000 y"
pub fn parse_newgroups_response(response: NntpResponse) -> Result<Vec<ActiveGroup>> {
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

        groups.push(ActiveGroup {
            name: parts[0].to_string(),
            high,
            low,
            status,
        });
    }

    Ok(groups)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_group_response() {
        let response = NntpResponse {
            code: 211,
            message: "3000 1 3000 free.pt".to_string(),
            lines: vec![],
        };

        let info = parse_group_response(response).unwrap();
        assert_eq!(info.count, 3000);
        assert_eq!(info.first, 1);
        assert_eq!(info.last, 3000);
    }
}

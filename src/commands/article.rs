//! Article retrieval and navigation commands

use crate::error::{NntpError, Result};
use crate::response::NntpResponse;

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

/// Build STAT command (RFC 3977 §6.2.4)
///
/// Gets article status without retrieving content.
/// Can be used with article number or message-id.
pub fn stat(id: &str) -> String {
    format!("STAT {}\r\n", id)
}

/// Build NEXT command (RFC 3977 §6.1.4)
///
/// Moves to the next article in the current group.
pub fn next() -> &'static str {
    "NEXT\r\n"
}

/// Build LAST command (RFC 3977 §6.1.3)
///
/// Moves to the previous article in the current group.
pub fn last() -> &'static str {
    "LAST\r\n"
}

/// Article information returned by STAT, NEXT, and LAST commands
///
/// Contains the article number and message-id for an article.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArticleInfo {
    /// Article number (0 if message-id was used in STAT request)
    pub number: u64,
    /// Message identifier (e.g., "<abc@example.com>")
    pub message_id: String,
}

/// Parse STAT response (RFC 3977 §6.2.4)
///
/// Response format: "223 n message-id"
/// - n is the article number (0 if message-id was used in request)
/// - message-id is the article's message identifier
///
/// Returns [`ArticleInfo`] containing the article number and message-id
pub fn parse_stat_response(response: NntpResponse) -> Result<ArticleInfo> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message,
        });
    }

    let parts: Vec<&str> = response.message.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(NntpError::InvalidResponse(response.message));
    }

    let article_number = parts[0]
        .parse()
        .map_err(|_| NntpError::InvalidResponse(response.message.clone()))?;

    // Message-ID is the rest of the message, joined in case it contains spaces
    // (though it shouldn't per RFC, but we handle it gracefully)
    let message_id = parts[1..].join(" ");

    Ok(ArticleInfo {
        number: article_number,
        message_id,
    })
}

/// Parse NEXT response (RFC 3977 §6.1.4)
///
/// Response format: "223 n message-id"
/// - n is the article number
/// - message-id is the article's message identifier
///
/// Returns [`ArticleInfo`] containing the article number and message-id
pub fn parse_next_response(response: NntpResponse) -> Result<ArticleInfo> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message,
        });
    }

    let parts: Vec<&str> = response.message.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(NntpError::InvalidResponse(response.message));
    }

    let article_number = parts[0]
        .parse()
        .map_err(|_| NntpError::InvalidResponse(response.message.clone()))?;

    // Message-ID is the rest of the message, joined in case it contains spaces
    // (though it shouldn't per RFC, but we handle it gracefully)
    let message_id = parts[1..].join(" ");

    Ok(ArticleInfo {
        number: article_number,
        message_id,
    })
}

/// Parse response to LAST command (RFC 3977 §6.1.3)
///
/// Response format: "223 n message-id"
/// where n is the article number and message-id is the message identifier.
///
/// Returns [`ArticleInfo`] containing the article number and message-id
pub fn parse_last_response(response: NntpResponse) -> Result<ArticleInfo> {
    if !response.is_success() {
        return Err(NntpError::Protocol {
            code: response.code,
            message: response.message,
        });
    }

    let parts: Vec<&str> = response.message.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(NntpError::InvalidResponse(response.message));
    }

    let article_number = parts[0]
        .parse()
        .map_err(|_| NntpError::InvalidResponse(response.message.clone()))?;

    // Message-ID is the rest of the message, joined in case it contains spaces
    // (though it shouldn't per RFC, but we handle it gracefully)
    let message_id = parts[1..].join(" ");

    Ok(ArticleInfo {
        number: article_number,
        message_id,
    })
}

//! Article parsing functions
//!
//! This module contains functions for parsing raw article text into structured data.

use std::collections::HashMap;

use crate::encoded_words::decode_header_value;
use crate::{NntpError, Result};

use super::types::{Article, Headers};

/// Parse raw article text into headers and body
///
/// Splits article at the first blank line (CRLF CRLF or LF LF).
/// Returns (headers_text, body_text) tuple.
pub fn split_article(raw: &str) -> (&str, &str) {
    // Try CRLF first (standard)
    if let Some(pos) = raw.find("\r\n\r\n") {
        return (&raw[..pos], &raw[pos + 4..]);
    }

    // Fallback to LF (non-standard but common)
    if let Some(pos) = raw.find("\n\n") {
        return (&raw[..pos], &raw[pos + 2..]);
    }

    // No separator found - entire text is headers
    (raw, "")
}

/// Parse comma-separated list (for Newsgroups, Followup-To, etc.)
///
/// RFC 5536: Values are comma-separated, whitespace around commas is optional
pub fn parse_comma_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// Parse space-separated message-id list (for References)
///
/// RFC 5536: Message-IDs are separated by CFWS (whitespace/comments)
/// We handle basic whitespace separation here
pub fn parse_message_id_list(value: &str) -> Vec<String> {
    value
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// Unfold header value by removing continuation line breaks
///
/// RFC 5536/5322: Continuation lines start with whitespace (space or tab)
/// Replace CRLF or LF followed by whitespace with a single space
pub fn unfold_header(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    let mut prev_was_newline = false;

    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                // Skip CR, wait for LF
                if chars.peek() == Some(&'\n') {
                    prev_was_newline = true;
                }
            }
            '\n' => {
                prev_was_newline = true;
            }
            ' ' | '\t' if prev_was_newline => {
                // This is a continuation line - replace newline+whitespace with space
                if !result.ends_with(' ') {
                    result.push(' ');
                }
                prev_was_newline = false;
            }
            _ => {
                if prev_was_newline {
                    // Newline wasn't followed by whitespace, so it's not a fold
                    // This shouldn't happen in valid headers, but handle it
                    result.push(' ');
                }
                result.push(ch);
                prev_was_newline = false;
            }
        }
    }

    result.trim().to_string()
}

/// Parse headers from raw header text
///
/// RFC 5536 Section 3: Header field format is "name: value"
/// - Header names are case-insensitive
/// - Continuation lines start with whitespace
/// - At least one space should follow the colon
///
/// # Arguments
///
/// * `headers_text` - Raw header section text
///
/// # Returns
///
/// Parsed `Headers` struct or error if required headers are missing
pub fn parse_headers(headers_text: &str) -> Result<Headers> {
    let mut raw_headers: HashMap<String, String> = HashMap::new();
    let mut current_name: Option<String> = None;
    let mut current_value = String::new();

    // Process line by line, handling folding
    for line in headers_text.lines() {
        if line.is_empty() {
            continue;
        }

        // Check if this is a continuation line (starts with whitespace)
        if line.starts_with(' ') || line.starts_with('\t') {
            // Continuation of previous header
            current_value.push('\n');
            current_value.push_str(line);
        } else {
            // New header field
            // Save previous header if any
            if let Some(name) = current_name.take() {
                let unfolded = unfold_header(&current_value);
                raw_headers.insert(name.to_lowercase(), unfolded);
            }

            // Parse new header: "name: value"
            if let Some(colon_pos) = line.find(':') {
                let name = line[..colon_pos].trim();
                let value = line[colon_pos + 1..].trim_start(); // At least one space after colon

                current_name = Some(name.to_string());
                current_value = value.to_string();
            }
        }
    }

    // Save last header
    if let Some(name) = current_name {
        let unfolded = unfold_header(&current_value);
        raw_headers.insert(name.to_lowercase(), unfolded);
    }

    // Extract required headers
    let date = raw_headers
        .get("date")
        .ok_or_else(|| NntpError::InvalidResponse("Missing required header: Date".to_string()))?
        .clone();

    let from = raw_headers
        .get("from")
        .ok_or_else(|| NntpError::InvalidResponse("Missing required header: From".to_string()))
        .map(|s| decode_header_value(s))?;

    let message_id = raw_headers
        .get("message-id")
        .ok_or_else(|| {
            NntpError::InvalidResponse("Missing required header: Message-ID".to_string())
        })?
        .clone();

    let newsgroups_str = raw_headers.get("newsgroups").ok_or_else(|| {
        NntpError::InvalidResponse("Missing required header: Newsgroups".to_string())
    })?;
    let newsgroups = parse_comma_list(newsgroups_str);

    let path = raw_headers
        .get("path")
        .ok_or_else(|| NntpError::InvalidResponse("Missing required header: Path".to_string()))?
        .clone();

    let subject = raw_headers
        .get("subject")
        .ok_or_else(|| NntpError::InvalidResponse("Missing required header: Subject".to_string()))
        .map(|s| decode_header_value(s))?;

    // Extract optional headers
    let references = raw_headers
        .get("references")
        .map(|s| parse_message_id_list(s));

    let reply_to = raw_headers.get("reply-to").map(|s| decode_header_value(s));

    let organization = raw_headers
        .get("organization")
        .map(|s| decode_header_value(s));

    let followup_to = raw_headers.get("followup-to").map(|s| parse_comma_list(s));

    let expires = raw_headers.get("expires").cloned();

    let control = raw_headers.get("control").cloned();

    let distribution = raw_headers.get("distribution").cloned();

    let keywords = raw_headers.get("keywords").map(|s| decode_header_value(s));

    let summary = raw_headers.get("summary").map(|s| decode_header_value(s));

    let supersedes = raw_headers.get("supersedes").cloned();

    let approved = raw_headers.get("approved").cloned();

    let lines = raw_headers.get("lines").and_then(|s| s.parse::<u32>().ok());

    let user_agent = raw_headers.get("user-agent").cloned();

    let xref = raw_headers.get("xref").cloned();

    // Collect non-standard headers (X-* and others)
    let mut extra = HashMap::new();
    let standard_headers = [
        "date",
        "from",
        "message-id",
        "newsgroups",
        "path",
        "subject",
        "references",
        "reply-to",
        "organization",
        "followup-to",
        "expires",
        "control",
        "distribution",
        "keywords",
        "summary",
        "supersedes",
        "approved",
        "lines",
        "user-agent",
        "xref",
    ];

    for (name, value) in raw_headers {
        if !standard_headers.contains(&name.as_str()) {
            extra.insert(name, value);
        }
    }

    Ok(Headers {
        date,
        from,
        message_id,
        newsgroups,
        path,
        subject,
        references,
        reply_to,
        organization,
        followup_to,
        expires,
        control,
        distribution,
        keywords,
        summary,
        supersedes,
        approved,
        lines,
        user_agent,
        xref,
        extra,
    })
}

/// Parse a complete article from raw text
///
/// RFC 5536: Article format is headers, blank line, body
///
/// # Arguments
///
/// * `raw` - Raw article text including headers and body
///
/// # Returns
///
/// Parsed `Article` with headers and body, or error if malformed
pub fn parse_article(raw: &str) -> Result<Article> {
    let (headers_text, body_text) = split_article(raw);
    let headers = parse_headers(headers_text)?;

    Ok(Article {
        headers,
        body: body_text.to_string(),
        raw: Some(raw.to_string()),
    })
}

//! Core NNTP response parsing utilities

use crate::error::{NntpError, Result};
use crate::response::NntpResponse;

/// Parse NNTP response line into code and message
pub fn parse_response_line(line: &str) -> Result<(u16, String)> {
    // Strip UTF-8 BOM if present (some broken servers/proxies add it)
    let line = line.trim_start_matches('\u{FEFF}');

    // Check minimum length and that first 3 chars are ASCII digits
    let bytes = line.as_bytes();
    if bytes.len() < 3
        || !bytes[0].is_ascii_digit()
        || !bytes[1].is_ascii_digit()
        || !bytes[2].is_ascii_digit()
    {
        return Err(NntpError::InvalidResponse(line.chars().take(100).collect()));
    }

    // Validate that position 3 is either space, end-of-string, or start of message
    // This catches malformed codes like "99999" being parsed as "999" with message "9 ..."
    if bytes.len() > 3 && bytes[3].is_ascii_digit() {
        return Err(NntpError::InvalidResponse(line.chars().take(100).collect()));
    }

    // Safe to slice since we verified ASCII
    let code = line[0..3]
        .parse::<u16>()
        .map_err(|_| NntpError::InvalidResponse(line.chars().take(100).collect()))?;

    // Extract message: if char 3 is space, start at 4; otherwise start at 3
    let message = if line.len() > 3 {
        if bytes[3] == b' ' {
            // Normal case: "200 message"
            line[4..].to_string()
        } else {
            // Missing space case: "200message" - start at position 3
            line[3..].to_string()
        }
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
    fn test_parse_response_line_code_overflow() {
        // BUG 2 fix: Code overflow "99999" should be rejected, not parsed as 999
        assert!(parse_response_line("99999 message").is_err());
        assert!(parse_response_line("2000 message").is_err());
        assert!(parse_response_line("1234567 message").is_err());

        // Valid 3-digit codes should still work
        let (code, msg) = parse_response_line("999 message").unwrap();
        assert_eq!(code, 999);
        assert_eq!(msg, "message");
    }

    #[test]
    fn test_parse_response_line_bom() {
        // BUG 3 fix: UTF-8 BOM prefix should be stripped
        let (code, msg) = parse_response_line("\u{FEFF}200 server ready").unwrap();
        assert_eq!(code, 200);
        assert_eq!(msg, "server ready");

        // Multiple BOMs (unlikely but handle gracefully)
        let (code, msg) = parse_response_line("\u{FEFF}\u{FEFF}200 ok").unwrap();
        assert_eq!(code, 200);
        assert_eq!(msg, "ok");
    }

    #[test]
    fn test_parse_response_line_missing_space() {
        // BUG 4 fix: Missing space should preserve full message
        let (code, msg) = parse_response_line("200message").unwrap();
        assert_eq!(code, 200);
        assert_eq!(msg, "message");

        // Code only, no message
        let (code, msg) = parse_response_line("200").unwrap();
        assert_eq!(code, 200);
        assert_eq!(msg, "");

        // Normal case with space still works
        let (code, msg) = parse_response_line("200 message").unwrap();
        assert_eq!(code, 200);
        assert_eq!(msg, "message");
    }
}

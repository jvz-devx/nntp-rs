//! RFC 2047 Encoded Words Support
//!
//! This module provides support for decoding RFC 2047 encoded words, which allow
//! non-ASCII characters in email and Usenet headers.
//!
//! Encoded words have the format: `=?charset?encoding?encoded-text?=`
//!
//! ## Supported Encodings
//! - `B` - Base64 (RFC 2045)
//! - `Q` - Quoted-Printable variant (RFC 2047)
//!
//! ## Supported Charsets
//! - UTF-8 (full support)
//! - ISO-8859-1 (full support via direct byte mapping)
//! - Windows-1252 (lossy conversion)
//! - Other charsets use lossy conversion
//!
//! ## Usage Examples
//!
//! ### Basic Decoding
//!
//! ```
//! use nntp_rs::encoded_words::decode_header_value;
//!
//! // Decode a Base64-encoded UTF-8 subject
//! let subject = decode_header_value("=?UTF-8?B?SGVsbG8gV29ybGQ=?=");
//! assert_eq!(subject, "Hello World");
//!
//! // Decode Quoted-Printable
//! let name = decode_header_value("=?ISO-8859-1?Q?Andr=E9?=");
//! assert_eq!(name, "André");
//!
//! // Mixed encoded and plain text
//! let mixed = decode_header_value("Re: =?UTF-8?B?SGVsbG8=?= World");
//! assert_eq!(mixed, "Re: Hello World");
//! ```
//!
//! ### International Names and Subjects
//!
//! ```
//! use nntp_rs::encoded_words::decode_header_value;
//!
//! // French name with accents
//! let from = decode_header_value("=?UTF-8?Q?Fran=C3=A7ois_Dupr=C3=A9?= <francois@example.com>");
//! assert_eq!(from, "François Dupré <francois@example.com>");
//!
//! // Japanese subject
//! let subject = decode_header_value("=?UTF-8?B?44GT44KT44Gr44Gh44Gv?=");
//! assert_eq!(subject, "こんにちは");
//!
//! // German with umlauts
//! let subject = decode_header_value("=?ISO-8859-1?Q?M=FCnchen?=");
//! assert_eq!(subject, "München");
//! ```
//!
//! ### Multiple Encoded Words
//!
//! RFC 2047 specifies that whitespace between consecutive encoded words should be removed:
//!
//! ```
//! use nntp_rs::encoded_words::decode_header_value;
//!
//! // Multiple encoded words - whitespace removed between them
//! let text = decode_header_value("=?UTF-8?B?SGVsbG8=?= =?UTF-8?B?V29ybGQ=?=");
//! assert_eq!(text, "HelloWorld");
//!
//! // Mixed with plain text - whitespace preserved
//! let text = decode_header_value("Subject: =?UTF-8?B?SGVsbG8=?= World");
//! assert_eq!(text, "Subject: Hello World");
//! ```
//!
//! ### Using with Article Parsing
//!
//! The encoded_words module is automatically integrated into article header parsing:
//!
//! ```
//! use nntp_rs::parse_article;
//!
//! // Parse an article with encoded headers
//! let article_text = "From: =?UTF-8?Q?Andr=C3=A9?= <andre@example.com>\r
//! Subject: =?UTF-8?B?SGVsbG8gV29ybGQ=?=\r
//! Message-ID: <test@example.com>\r
//! Date: Wed, 22 Jan 2026 12:00:00 +0000\r
//! Newsgroups: comp.lang.rust\r
//! Path: news.example.com!not-for-mail\r
//! \r
//! Body text\r
//! ";
//!
//! let article = parse_article(article_text).unwrap();
//! assert_eq!(article.headers.from, "André <andre@example.com>");
//! assert_eq!(article.headers.subject, "Hello World");
//! ```

use crate::error::{NntpError, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

/// Decodes a single RFC 2047 encoded word.
///
/// An encoded word has the format: `=?charset?encoding?encoded-text?=`
///
/// Returns the decoded string, or the original input if decoding fails.
///
/// # Supported Encodings
/// - `B` - Base64 encoding
/// - `Q` - Quoted-Printable encoding (underscores become spaces)
///
/// # Examples
///
/// ```
/// use nntp_rs::encoded_words::decode_encoded_word;
///
/// // Base64 encoding
/// let decoded = decode_encoded_word("=?UTF-8?B?SGVsbG8=?=");
/// assert_eq!(decoded, "Hello");
///
/// // Quoted-Printable encoding
/// let decoded = decode_encoded_word("=?ISO-8859-1?Q?Caf=E9?=");
/// assert_eq!(decoded, "Café");
///
/// // Invalid encoded words are returned unchanged
/// let invalid = decode_encoded_word("not an encoded word");
/// assert_eq!(invalid, "not an encoded word");
/// ```
pub fn decode_encoded_word(encoded: &str) -> String {
    // Check for valid encoded word format: =?charset?encoding?text?=
    if !encoded.starts_with("=?") || !encoded.ends_with("?=") {
        return encoded.to_string();
    }

    // Remove the =? and ?= markers
    let inner = &encoded[2..encoded.len() - 2];

    // Split into parts: charset?encoding?text
    let parts: Vec<&str> = inner.splitn(3, '?').collect();
    if parts.len() != 3 {
        return encoded.to_string();
    }

    let charset = parts[0];
    let encoding = parts[1].to_uppercase();
    let encoded_text = parts[2];

    // Decode based on encoding type
    let decoded_bytes = match encoding.as_str() {
        "B" => decode_base64(encoded_text),
        "Q" => decode_quoted_printable(encoded_text),
        _ => return encoded.to_string(), // Unknown encoding
    };

    let decoded_bytes = match decoded_bytes {
        Ok(bytes) => bytes,
        Err(_) => return encoded.to_string(), // Decoding failed
    };

    // Convert bytes to string based on charset
    charset_to_string(&decoded_bytes, charset)
}

/// Decodes a header value that may contain one or more encoded words.
///
/// This is the main function you should use for decoding header values.
/// It handles complex real-world scenarios automatically.
///
/// # Behavior
/// - Multiple consecutive encoded words: whitespace between them is removed per RFC 2047
/// - Mixed encoded and plain text: both are handled correctly
/// - Invalid encoded words: passed through unchanged
/// - Plain text: returned as-is
///
/// # Examples
///
/// ```
/// use nntp_rs::encoded_words::decode_header_value;
///
/// // Multiple encoded words - whitespace between encoded words is removed
/// let text = decode_header_value("=?UTF-8?B?SGVsbG8=?= =?UTF-8?B?V29ybGQ=?=");
/// assert_eq!(text, "HelloWorld");
///
/// // Mixed with plain text - whitespace preserved around plain text
/// let text = decode_header_value("Subject: =?UTF-8?B?SGVsbG8=?= World");
/// assert_eq!(text, "Subject: Hello World");
///
/// // Real-world From header with international name
/// let from = decode_header_value("=?UTF-8?Q?Mar=C3=ADa_Garc=C3=ADa?= <maria@example.com>");
/// assert_eq!(from, "María García <maria@example.com>");
///
/// // Plain text (no encoding) - returned unchanged
/// let plain = decode_header_value("Plain ASCII subject");
/// assert_eq!(plain, "Plain ASCII subject");
///
/// // Invalid encoded words are passed through
/// let invalid = decode_header_value("=?invalid encoding");
/// assert_eq!(invalid, "=?invalid encoding");
/// ```
pub fn decode_header_value(value: &str) -> String {
    let mut result = String::new();
    let bytes = value.as_bytes();
    let mut i = 0;
    let mut last_was_encoded = false;

    while i < bytes.len() {
        // Check for encoded word start
        if i + 1 < bytes.len() && bytes[i] == b'=' && bytes[i + 1] == b'?' {
            let remaining = &value[i..];

            // Find the end of the encoded word
            if let Some(end_idx) = find_encoded_word_end(remaining) {
                let encoded_word = &remaining[..end_idx];
                let decoded = decode_encoded_word(encoded_word);

                // Remove trailing whitespace if last segment was also encoded
                // (RFC 2047: whitespace between encoded words is ignored)
                if last_was_encoded {
                    while result.ends_with(' ') || result.ends_with('\t') {
                        result.pop();
                    }
                }

                result.push_str(&decoded);
                last_was_encoded = true;
                i += end_idx;
                continue;
            }
        }

        // Regular character
        result.push(bytes[i] as char);
        if bytes[i] != b' ' && bytes[i] != b'\t' {
            last_was_encoded = false;
        }
        i += 1;
    }

    result
}

/// Finds the end position of an encoded word starting at the beginning of the input.
///
/// Returns the byte index after the closing `?=`, or None if no valid encoded word is found.
fn find_encoded_word_end(input: &str) -> Option<usize> {
    if !input.starts_with("=?") {
        return None;
    }

    // Look for the closing ?=
    let mut question_count = 0;
    let bytes = input.as_bytes();

    for i in 2..bytes.len() {
        let ch = bytes[i] as char;
        if ch == '?' {
            question_count += 1;
            // Need to find at least 3 question marks: charset?encoding?text?=
            if question_count >= 3 && i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                return Some(i + 2);
            }
        } else if ch == ' ' || ch == '\t' {
            // Whitespace not allowed inside encoded word
            return None;
        }
    }

    None
}

/// Decodes Base64 encoded text (B encoding).
fn decode_base64(encoded: &str) -> Result<Vec<u8>> {
    BASE64
        .decode(encoded)
        .map_err(|e| NntpError::InvalidResponse(format!("Base64 decode error: {}", e)))
}

/// Decodes Quoted-Printable encoded text (Q encoding).
///
/// Q encoding is similar to quoted-printable but:
/// - Underscores represent spaces
/// - Any 8-bit value is represented as =XX (hex)
fn decode_quoted_printable(encoded: &str) -> Result<Vec<u8>> {
    let mut result = Vec::new();
    let mut chars = encoded.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '_' => result.push(b' '), // Underscore represents space
            '=' => {
                // Hex escape: =XX
                let hex1 = chars.next();
                let hex2 = chars.next();
                if let (Some(h1), Some(h2)) = (hex1, hex2) {
                    let hex_str = format!("{}{}", h1, h2);
                    if let Ok(byte) = u8::from_str_radix(&hex_str, 16) {
                        result.push(byte);
                    } else {
                        // Invalid hex, pass through
                        result.push(b'=');
                        result.push(h1 as u8);
                        result.push(h2 as u8);
                    }
                } else {
                    // Incomplete escape, pass through
                    result.push(b'=');
                }
            }
            _ => {
                // Regular character
                if ch.is_ascii() {
                    result.push(ch as u8);
                } else {
                    // Non-ASCII in Q encoding is invalid, but pass through
                    for byte in ch.to_string().as_bytes() {
                        result.push(*byte);
                    }
                }
            }
        }
    }

    Ok(result)
}

/// Converts bytes to a String based on the specified charset.
///
/// Currently supports:
/// - UTF-8 (full support)
/// - ISO-8859-1, Windows-1252, and other charsets (lossy conversion)
fn charset_to_string(bytes: &[u8], charset: &str) -> String {
    let charset_lower = charset.to_lowercase();

    match charset_lower.as_str() {
        "utf-8" | "utf8" => {
            // Try UTF-8 decoding, fall back to lossy if invalid
            String::from_utf8(bytes.to_vec())
                .unwrap_or_else(|_| String::from_utf8_lossy(bytes).to_string())
        }
        "iso-8859-1" | "latin1" => {
            // ISO-8859-1: each byte maps directly to a Unicode code point
            bytes.iter().map(|&b| b as char).collect()
        }
        "windows-1252" | "cp1252" => {
            // Windows-1252: similar to ISO-8859-1 but with different mappings for 0x80-0x9F
            // For now, use lossy conversion (full support would require encoding_rs crate)
            String::from_utf8_lossy(bytes).to_string()
        }
        _ => {
            // Unknown charset: use lossy conversion
            String::from_utf8_lossy(bytes).to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for decode_base64
    #[test]
    fn test_decode_base64_valid() {
        let result = decode_base64("SGVsbG8gV29ybGQ=").unwrap();
        assert_eq!(result, b"Hello World");
    }

    #[test]
    fn test_decode_base64_empty() {
        let result = decode_base64("").unwrap();
        assert_eq!(result, b"");
    }

    #[test]
    fn test_decode_base64_invalid() {
        let result = decode_base64("!!!invalid!!!");
        assert!(result.is_err());
    }

    // Tests for decode_quoted_printable
    #[test]
    fn test_decode_quoted_printable_underscore() {
        let result = decode_quoted_printable("Hello_World").unwrap();
        assert_eq!(result, b"Hello World");
    }

    #[test]
    fn test_decode_quoted_printable_hex_escape() {
        let result = decode_quoted_printable("Caf=E9").unwrap();
        assert_eq!(result, &[b'C', b'a', b'f', 0xE9]);
    }

    #[test]
    fn test_decode_quoted_printable_mixed() {
        let result = decode_quoted_printable("Hello=20World=21").unwrap();
        assert_eq!(result, b"Hello World!");
    }

    #[test]
    fn test_decode_quoted_printable_invalid_hex() {
        let result = decode_quoted_printable("Hello=ZZ").unwrap();
        // Invalid hex should pass through
        assert_eq!(result, b"Hello=ZZ");
    }

    // Tests for charset_to_string
    #[test]
    fn test_charset_to_string_utf8() {
        let bytes = "Hello 世界".as_bytes();
        let result = charset_to_string(bytes, "UTF-8");
        assert_eq!(result, "Hello 世界");
    }

    #[test]
    fn test_charset_to_string_iso_8859_1() {
        let bytes = &[b'C', b'a', b'f', 0xE9]; // "Café" in ISO-8859-1
        let result = charset_to_string(bytes, "ISO-8859-1");
        assert_eq!(result, "Café");
    }

    #[test]
    fn test_charset_to_string_unknown() {
        let bytes = b"Hello";
        let result = charset_to_string(bytes, "unknown-charset");
        assert_eq!(result, "Hello");
    }

    // Tests for decode_encoded_word
    #[test]
    fn test_decode_encoded_word_base64_utf8() {
        let result = decode_encoded_word("=?UTF-8?B?SGVsbG8gV29ybGQ=?=");
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_decode_encoded_word_quoted_printable() {
        let result = decode_encoded_word("=?ISO-8859-1?Q?Caf=E9?=");
        assert_eq!(result, "Café");
    }

    #[test]
    fn test_decode_encoded_word_invalid_format() {
        let result = decode_encoded_word("not an encoded word");
        assert_eq!(result, "not an encoded word");
    }

    #[test]
    fn test_decode_encoded_word_missing_parts() {
        let result = decode_encoded_word("=?UTF-8?B?=");
        assert_eq!(result, "=?UTF-8?B?=");
    }

    #[test]
    fn test_decode_encoded_word_unknown_encoding() {
        let result = decode_encoded_word("=?UTF-8?X?test?=");
        assert_eq!(result, "=?UTF-8?X?test?=");
    }

    // Tests for decode_header_value
    #[test]
    fn test_decode_header_value_single_encoded() {
        let result = decode_header_value("=?UTF-8?B?SGVsbG8=?=");
        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_decode_header_value_multiple_encoded() {
        let result = decode_header_value("=?UTF-8?B?SGVsbG8=?= =?UTF-8?B?V29ybGQ=?=");
        assert_eq!(result, "HelloWorld");
    }

    #[test]
    fn test_decode_header_value_mixed_plain_and_encoded() {
        let result = decode_header_value("Re: =?UTF-8?B?SGVsbG8=?= World");
        assert_eq!(result, "Re: Hello World");
    }

    #[test]
    fn test_decode_header_value_plain_text() {
        let result = decode_header_value("Plain text subject");
        assert_eq!(result, "Plain text subject");
    }

    #[test]
    fn test_decode_header_value_invalid_encoded_word() {
        let result = decode_header_value("=?invalid");
        assert_eq!(result, "=?invalid");
    }

    #[test]
    fn test_decode_header_value_empty() {
        let result = decode_header_value("");
        assert_eq!(result, "");
    }

    // Tests for find_encoded_word_end
    #[test]
    fn test_find_encoded_word_end_valid() {
        let result = find_encoded_word_end("=?UTF-8?B?test?=");
        assert_eq!(result, Some(16));
    }

    #[test]
    fn test_find_encoded_word_end_with_whitespace() {
        let result = find_encoded_word_end("=?UTF-8?B?te st?=");
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_encoded_word_end_incomplete() {
        let result = find_encoded_word_end("=?UTF-8?B?test");
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_encoded_word_end_not_encoded() {
        let result = find_encoded_word_end("not encoded");
        assert_eq!(result, None);
    }
}

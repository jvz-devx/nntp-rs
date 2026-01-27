//! RFC 2047 Encoded Words Tests
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc2047
//!
//! Tests for decoding RFC 2047 encoded words in article headers.

use nntp_rs::encoded_words::{decode_encoded_word, decode_header_value};

#[test]
fn test_decode_base64_utf8_simple() {
    // "Hello World" in UTF-8 Base64
    let encoded = "=?UTF-8?B?SGVsbG8gV29ybGQ=?=";
    let decoded = decode_encoded_word(encoded);
    assert_eq!(decoded, "Hello World");
}

#[test]
fn test_decode_base64_utf8_subject() {
    // Full subject header with Base64 encoding
    let subject = decode_header_value("=?UTF-8?B?SGVsbG8gV29ybGQ=?=");
    assert_eq!(subject, "Hello World");
}

#[test]
fn test_decode_base64_utf8_unicode() {
    // "こんにちは" (Japanese "Hello") in UTF-8 Base64
    let encoded = "=?UTF-8?B?44GT44KT44Gr44Gh44Gv?=";
    let decoded = decode_encoded_word(encoded);
    assert_eq!(decoded, "こんにちは");
}

#[test]
fn test_decode_multiple_base64_words_in_header() {
    // Multiple consecutive encoded words (whitespace between should be removed)
    let value = "=?UTF-8?B?SGVsbG8=?= =?UTF-8?B?V29ybGQ=?=";
    let decoded = decode_header_value(value);
    assert_eq!(decoded, "HelloWorld");
}

#[test]
fn test_decode_base64_with_special_chars() {
    // Test with emojis and special characters
    let encoded = "=?UTF-8?B?8J+YgCBIZWxsbyDwn5iA?=";
    let decoded = decode_encoded_word(encoded);
    assert_eq!(decoded, "😀 Hello 😀");
}

#[test]
fn test_decode_base64_mixed_with_plain_text() {
    // Mixed encoded word and plain text
    let value = "Re: =?UTF-8?B?SGVsbG8=?= World";
    let decoded = decode_header_value(value);
    assert_eq!(decoded, "Re: Hello World");
}

#[test]
fn test_decode_quoted_printable_simple() {
    // "Café" in ISO-8859-1 Quoted-Printable
    let encoded = "=?ISO-8859-1?Q?Caf=E9?=";
    let decoded = decode_encoded_word(encoded);
    assert_eq!(decoded, "Café");
}

#[test]
fn test_decode_quoted_printable_underscores() {
    // Underscores represent spaces in Q encoding
    let encoded = "=?UTF-8?Q?Hello_World?=";
    let decoded = decode_encoded_word(encoded);
    assert_eq!(decoded, "Hello World");
}

#[test]
fn test_decode_quoted_printable_from_header() {
    // Real-world From header with encoded name
    let from = "=?UTF-8?Q?Andr=C3=A9?= <andre@example.com>";
    let decoded = decode_header_value(from);
    assert_eq!(decoded, "André <andre@example.com>");
}

#[test]
fn test_decode_quoted_printable_german_umlauts() {
    // German umlauts in ISO-8859-1
    let encoded = "=?ISO-8859-1?Q?M=FCnchen?=";
    let decoded = decode_encoded_word(encoded);
    assert_eq!(decoded, "München");
}

#[test]
fn test_decode_quoted_printable_mixed_chars() {
    // Mix of encoded and non-encoded characters
    let encoded = "=?UTF-8?Q?Caf=C3=A9_de_la_Pa=C3=AFx?=";
    let decoded = decode_encoded_word(encoded);
    assert_eq!(decoded, "Café de la Païx");
}

#[test]
fn test_decode_quoted_printable_accented_chars() {
    // Various accented characters
    let encoded = "=?UTF-8?Q?=C3=A0=C3=A9=C3=AE=C3=B4=C3=BB?=";
    let decoded = decode_encoded_word(encoded);
    assert_eq!(decoded, "àéîôû");
}

#[test]
fn test_invalid_encoded_word_passthrough() {
    // Invalid encoded words should be returned as-is
    let invalid = "=?UTF-8?X?Invalid?="; // X is not a valid encoding
    let decoded = decode_encoded_word(invalid);
    assert_eq!(decoded, invalid);
}

#[test]
fn test_malformed_encoded_word_missing_closing() {
    // Missing closing ?=
    let malformed = "=?UTF-8?B?SGVsbG8";
    let decoded = decode_encoded_word(malformed);
    assert_eq!(decoded, malformed);
}

#[test]
fn test_malformed_encoded_word_missing_opening() {
    // Missing opening =?
    let malformed = "UTF-8?B?SGVsbG8?=";
    let decoded = decode_encoded_word(malformed);
    assert_eq!(decoded, malformed);
}

#[test]
fn test_empty_encoded_word() {
    // Empty encoded text
    let encoded = "=?UTF-8?B??=";
    let decoded = decode_encoded_word(encoded);
    // Should decode to empty string or pass through
    assert!(decoded.is_empty() || decoded == encoded);
}

#[test]
fn test_nested_encoded_markers() {
    // Nested/malformed encoding markers
    let nested = "=?UTF-8?B?=?UTF-8?B?SGVsbG8=?=?=";
    let decoded = decode_encoded_word(nested);
    // Should handle gracefully (either decode or pass through)
    assert!(!decoded.is_empty());
}

#[test]
fn test_very_long_encoded_word() {
    // Very long Base64 encoded string (75+ characters is RFC limit, but we should handle longer)
    let long_text = "A".repeat(100);
    let long_base64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        long_text.as_bytes(),
    );
    let encoded = format!("=?UTF-8?B?{}?=", long_base64);
    let decoded = decode_encoded_word(&encoded);
    assert_eq!(decoded, long_text);
}

#[test]
fn test_plain_text_passthrough() {
    // Plain text without encoding should pass through unchanged
    let plain = "Hello World";
    let decoded = decode_header_value(plain);
    assert_eq!(decoded, plain);
}

#[test]
fn test_incomplete_encoded_word_in_value() {
    // Encoded word marker in middle of text
    let value = "Hello =?UTF-8 World";
    let decoded = decode_header_value(value);
    assert_eq!(decoded, value);
}

#[test]
fn test_multiple_encodings_same_header() {
    // Multiple different encodings in one header
    let value = "=?UTF-8?B?SGVsbG8=?= and =?ISO-8859-1?Q?Caf=E9?=";
    let decoded = decode_header_value(value);
    assert_eq!(decoded, "Hello and Café");
}

#[test]
fn test_whitespace_preservation() {
    // Whitespace between non-encoded words should be preserved
    let value = "Hello   World   Test";
    let decoded = decode_header_value(value);
    assert_eq!(decoded, "Hello   World   Test");
}

#[test]
fn test_real_world_international_from_header() {
    // Real-world example: International name in From header
    let from = "=?UTF-8?Q?Fran=C3=A7ois_Dupr=C3=A9?= <francois@example.com>";
    let decoded = decode_header_value(from);
    assert_eq!(decoded, "François Dupré <francois@example.com>");
}

#[test]
fn test_real_world_japanese_subject() {
    // Real-world example: Japanese subject
    let subject = "=?UTF-8?B?44CMUmVcOuOAgei2s+WRs+OBruOBiumhjuOAjeOBq+OBpOOBhOOBpg==?=";
    let decoded = decode_header_value(subject);
    // Should contain Japanese characters
    assert!(decoded.contains("Re"));
}

#[test]
fn test_real_world_german_subject() {
    // Real-world example: German subject with umlauts
    let subject = "=?ISO-8859-1?Q?Gr=FC=DFe_aus_M=FCnchen?=";
    let decoded = decode_header_value(subject);
    assert_eq!(decoded, "Grüße aus München");
}

#[test]
fn test_real_world_russian_subject() {
    // Real-world example: Russian/Cyrillic subject
    let subject = "=?UTF-8?B?0JTQvtCx0YDRi9C5INC00LXQvdGM?=";
    let decoded = decode_header_value(subject);
    assert_eq!(decoded, "Добрый день");
}

#[test]
fn test_real_world_mixed_encoding_reply() {
    // Real-world example: Reply prefix with encoded subject
    let subject = "Re: =?UTF-8?Q?Conna=C3=AEtre_les_d=C3=A9tails?=";
    let decoded = decode_header_value(subject);
    assert_eq!(decoded, "Re: Connaître les détails");
}

#[test]
fn test_real_world_organization_header() {
    // Real-world example: Organization header with encoding
    let org = "=?UTF-8?Q?Universit=C3=A9_de_Paris?=";
    let decoded = decode_header_value(org);
    assert_eq!(decoded, "Université de Paris");
}

#[test]
fn test_real_world_complex_from_with_quotes() {
    // Real-world example: Complex From header with quotes and encoding
    let from = "\"=?UTF-8?Q?Jos=C3=A9_Mart=C3=ADnez?=\" <jose@example.com>";
    let decoded = decode_header_value(from);
    assert_eq!(decoded, "\"José Martínez\" <jose@example.com>");
}

#[test]
fn test_real_world_multiple_words_subject() {
    // Real-world example: Long subject encoded in a single word
    // In practice, subjects longer than 75 chars should be split with folding
    let subject = "=?UTF-8?B?TG9uZyBzdWJqZWN0IHdpdGggbXVsdGlwbGUgcGFydHM=?=";
    let decoded = decode_header_value(subject);
    assert_eq!(decoded, "Long subject with multiple parts");
}

#[test]
fn test_real_world_reply_to_encoded() {
    // Real-world example: Reply-To header with encoding
    let reply_to = "=?UTF-8?Q?Mar=C3=ADa_Garc=C3=ADa?= <maria@example.es>";
    let decoded = decode_header_value(reply_to);
    assert_eq!(decoded, "María García <maria@example.es>");
}

#[test]
fn test_real_world_chinese_subject() {
    // Real-world example: Chinese subject
    let subject = "=?UTF-8?B?5Lit5paH5rWL6K+V?=";
    let decoded = decode_header_value(subject);
    assert_eq!(decoded, "中文测试");
}

#[test]
fn test_decode_header_value_preserves_structure() {
    // Ensure structure of email addresses is preserved
    let from = "=?UTF-8?Q?Test_User?= <test@example.com>";
    let decoded = decode_header_value(from);
    assert_eq!(decoded, "Test User <test@example.com>");
    assert!(decoded.contains("<"));
    assert!(decoded.contains(">"));
}

#[test]
fn test_windows_1252_charset() {
    // Test Windows-1252 charset support
    let encoded = "=?Windows-1252?Q?Smart_=93quotes=94?=";
    let decoded = decode_encoded_word(encoded);
    // Should decode without panic (lossy conversion is acceptable)
    assert!(!decoded.is_empty());
}

#[test]
fn test_unknown_charset_lossy_conversion() {
    // Unknown charset should use lossy conversion
    let encoded = "=?UNKNOWN-CHARSET?B?SGVsbG8=?=";
    let decoded = decode_encoded_word(encoded);
    // Should decode to "Hello" despite unknown charset
    assert_eq!(decoded, "Hello");
}

#[test]
fn test_case_insensitive_charset() {
    // Charset names should be case-insensitive
    let encoded_lower = "=?utf-8?B?SGVsbG8=?=";
    let encoded_upper = "=?UTF-8?B?SGVsbG8=?=";
    let decoded_lower = decode_encoded_word(encoded_lower);
    let decoded_upper = decode_encoded_word(encoded_upper);
    assert_eq!(decoded_lower, decoded_upper);
    assert_eq!(decoded_lower, "Hello");
}

#[test]
fn test_case_insensitive_encoding() {
    // Encoding type should be case-insensitive
    let encoded_lower = "=?UTF-8?b?SGVsbG8=?=";
    let encoded_upper = "=?UTF-8?B?SGVsbG8=?=";
    let decoded_lower = decode_encoded_word(encoded_lower);
    let decoded_upper = decode_encoded_word(encoded_upper);
    assert_eq!(decoded_lower, decoded_upper);
    assert_eq!(decoded_lower, "Hello");
}

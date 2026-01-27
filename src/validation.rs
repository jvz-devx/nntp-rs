//! RFC 5536 Article Validation
//!
//! Provides validation functions for Usenet article headers and components.
//! All validation follows RFC 5536 specifications.

use crate::{NntpError, Result};
use chrono::{DateTime, Duration, Utc};

/// Configuration options for validation behavior
///
/// Controls how strict validation should be and what constraints to apply.
///
/// # Examples
///
/// ```
/// use nntp_rs::validation::ValidationConfig;
///
/// // Strict validation (default)
/// let strict = ValidationConfig::strict();
///
/// // Lenient validation (allows some non-compliant but common practices)
/// let lenient = ValidationConfig::lenient();
///
/// // Custom configuration
/// let custom = ValidationConfig {
///     strict_date_validation: true,
///     allow_future_dates: false,
///     max_date_age_days: Some(365 * 2), // 2 years
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationConfig {
    /// If true, apply strict validation rules.
    /// If false, allow some common non-compliant practices.
    pub strict_date_validation: bool,

    /// If true, allow dates in the future.
    /// If false, reject dates that are after the current time.
    pub allow_future_dates: bool,

    /// Maximum age of articles in days.
    /// If Some(days), reject dates older than this.
    /// If None, no age limit.
    pub max_date_age_days: Option<i64>,
}

impl ValidationConfig {
    /// Creates a strict validation configuration
    ///
    /// - Strict date validation enabled
    /// - Future dates rejected
    /// - No age limit
    pub fn strict() -> Self {
        Self {
            strict_date_validation: true,
            allow_future_dates: false,
            max_date_age_days: None,
        }
    }

    /// Creates a lenient validation configuration
    ///
    /// - Lenient date validation
    /// - Future dates allowed
    /// - No age limit
    pub fn lenient() -> Self {
        Self {
            strict_date_validation: false,
            allow_future_dates: true,
            max_date_age_days: None,
        }
    }
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self::strict()
    }
}

/// Validates a Message-ID header value (RFC 5536 Section 3.1.3)
///
/// Message-IDs must have the format `<local-part@domain>`:
/// - Must start with `<` and end with `>`
/// - Must contain exactly one `@` sign
/// - Must not contain whitespace or control characters
///
/// # Examples
///
/// ```
/// use nntp_rs::validation::validate_message_id;
///
/// assert!(validate_message_id("<abc123@example.com>").is_ok());
/// assert!(validate_message_id("<uuid-v4@localhost>").is_ok());
/// assert!(validate_message_id("abc123@example.com").is_err()); // Missing brackets
/// assert!(validate_message_id("<abc123>").is_err());           // Missing @
/// ```
pub fn validate_message_id(message_id: &str) -> Result<()> {
    // Check minimum length: <a@b>
    if message_id.len() < 5 {
        return Err(NntpError::InvalidResponse(
            "Message-ID too short".to_string(),
        ));
    }

    // Check angle brackets
    if !message_id.starts_with('<') || !message_id.ends_with('>') {
        return Err(NntpError::InvalidResponse(
            "Message-ID must be enclosed in angle brackets: <local-part@domain>".to_string(),
        ));
    }

    // Extract content between brackets
    let content = &message_id[1..message_id.len() - 1];

    // Check for exactly one @ sign
    let at_count = content.matches('@').count();
    if at_count != 1 {
        return Err(NntpError::InvalidResponse(format!(
            "Message-ID must contain exactly one @ sign, found {}",
            at_count
        )));
    }

    // Split at @ and validate both parts are non-empty
    let parts: Vec<&str> = content.split('@').collect();
    if parts[0].is_empty() {
        return Err(NntpError::InvalidResponse(
            "Message-ID local-part cannot be empty".to_string(),
        ));
    }
    if parts[1].is_empty() {
        return Err(NntpError::InvalidResponse(
            "Message-ID domain cannot be empty".to_string(),
        ));
    }

    // Check for whitespace or control characters
    for ch in content.chars() {
        if ch.is_whitespace() || ch.is_control() {
            return Err(NntpError::InvalidResponse(
                "Message-ID cannot contain whitespace or control characters".to_string(),
            ));
        }
    }

    Ok(())
}

/// Validates a newsgroup name (RFC 5536 Section 3.1.4)
///
/// Newsgroup names must have the format `component.component.component`:
/// - Components separated by dots (.)
/// - Each component must be non-empty
/// - Components may contain: lowercase letters, digits, +, -, _
/// - Must not start or end with a dot
///
/// # Examples
///
/// ```
/// use nntp_rs::validation::validate_newsgroup_name;
///
/// assert!(validate_newsgroup_name("comp.lang.rust").is_ok());
/// assert!(validate_newsgroup_name("alt.binaries.test").is_ok());
/// assert!(validate_newsgroup_name("de.comp.lang.c++").is_ok());
/// assert!(validate_newsgroup_name("comp..rust").is_err());     // Empty component
/// assert!(validate_newsgroup_name(".comp.rust").is_err());     // Leading dot
/// assert!(validate_newsgroup_name("comp/lang/rust").is_err()); // Invalid char
/// ```
pub fn validate_newsgroup_name(newsgroup: &str) -> Result<()> {
    // Check not empty
    if newsgroup.is_empty() {
        return Err(NntpError::InvalidResponse(
            "Newsgroup name cannot be empty".to_string(),
        ));
    }

    // Check for leading or trailing dots
    if newsgroup.starts_with('.') || newsgroup.ends_with('.') {
        return Err(NntpError::InvalidResponse(
            "Newsgroup name cannot start or end with a dot".to_string(),
        ));
    }

    // Split by dots and validate each component
    let components: Vec<&str> = newsgroup.split('.').collect();

    if components.is_empty() {
        return Err(NntpError::InvalidResponse(
            "Newsgroup name must have at least one component".to_string(),
        ));
    }

    for component in components {
        // Check component is non-empty
        if component.is_empty() {
            return Err(NntpError::InvalidResponse(
                "Newsgroup name cannot have empty components".to_string(),
            ));
        }

        // Check each character is valid: lowercase letter, digit, +, -, or _
        for ch in component.chars() {
            if !(ch.is_ascii_lowercase()
                || ch.is_ascii_digit()
                || ch == '+'
                || ch == '-'
                || ch == '_')
            {
                return Err(NntpError::InvalidResponse(format!(
                    "Invalid character '{}' in newsgroup name (only lowercase letters, digits, +, -, _ allowed)",
                    ch
                )));
            }
        }
    }

    Ok(())
}

/// Parses an RFC 5322 date-time string into a `DateTime<Utc>`
///
/// Supports RFC 5322 date-time format as specified in RFC 5536 Section 3.1.1.
/// Also supports common variations found in the wild.
///
/// # Examples
///
/// ```
/// use nntp_rs::validation::parse_date;
/// use chrono::Datelike;
///
/// let date = parse_date("Tue, 20 Jan 2026 12:00:00 +0000").unwrap();
/// assert_eq!(date.year(), 2026);
///
/// // Also supports variations
/// parse_date("20 Jan 2026 12:00:00 GMT").unwrap();
/// parse_date("Tue, 20 Jan 2026 12:00:00 GMT").unwrap();
/// ```
pub fn parse_date(date_str: &str) -> Result<DateTime<Utc>> {
    // Try parsing as RFC 2822/RFC 5322 format first
    match DateTime::parse_from_rfc2822(date_str) {
        Ok(dt) => Ok(dt.with_timezone(&Utc)),
        Err(_) => {
            // Try common variations
            // Some servers use "GMT" instead of "+0000"
            if date_str.contains("GMT") {
                let normalized = date_str.replace("GMT", "+0000");
                if let Ok(dt) = DateTime::parse_from_rfc2822(&normalized) {
                    return Ok(dt.with_timezone(&Utc));
                }
            }

            Err(NntpError::InvalidResponse(format!(
                "Invalid date format: {} (expected RFC 5322 format)",
                date_str
            )))
        }
    }
}

/// Validates a date according to the provided configuration
///
/// Checks if a date is valid according to the configured validation rules:
/// - Future dates (if not allowed)
/// - Maximum age (if configured)
///
/// # Examples
///
/// ```
/// use nntp_rs::validation::{parse_date, validate_date, ValidationConfig};
///
/// let date = parse_date("Tue, 20 Jan 2026 12:00:00 +0000").unwrap();
/// let config = ValidationConfig::strict();
///
/// // Validate the parsed date with no age limit
/// validate_date(&date, &config).unwrap();
///
/// // Custom configuration with age limit (use lenient to allow recent dates)
/// let config_with_age = ValidationConfig {
///     strict_date_validation: false,
///     allow_future_dates: true,
///     max_date_age_days: Some(365 * 10), // 10 years
/// };
/// validate_date(&date, &config_with_age).unwrap();
/// ```
pub fn validate_date(date: &DateTime<Utc>, config: &ValidationConfig) -> Result<()> {
    let now = Utc::now();

    // Check for future dates
    if !config.allow_future_dates && *date > now {
        return Err(NntpError::InvalidResponse(format!(
            "Date is in the future: {}",
            date.to_rfc2822()
        )));
    }

    // Check maximum age
    if let Some(max_age_days) = config.max_date_age_days {
        let max_age = Duration::days(max_age_days);
        let oldest_allowed = now - max_age;

        if *date < oldest_allowed {
            return Err(NntpError::InvalidResponse(format!(
                "Date is too old (older than {} days): {}",
                max_age_days,
                date.to_rfc2822()
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_validate_message_id_valid() {
        assert!(validate_message_id("<abc123@example.com>").is_ok());
        assert!(validate_message_id("<uuid-v4@localhost>").is_ok());
        assert!(validate_message_id("<a@b>").is_ok());
        assert!(validate_message_id("<very.long.local-part_123@domain.example.com>").is_ok());
    }

    #[test]
    fn test_validate_message_id_missing_brackets() {
        assert!(validate_message_id("abc123@example.com").is_err());
        assert!(validate_message_id("<abc123@example.com").is_err());
        assert!(validate_message_id("abc123@example.com>").is_err());
    }

    #[test]
    fn test_validate_message_id_missing_at() {
        assert!(validate_message_id("<abc123>").is_err());
        assert!(validate_message_id("<abc123.example.com>").is_err());
    }

    #[test]
    fn test_validate_message_id_whitespace() {
        assert!(validate_message_id("<abc 123@example.com>").is_err());
        assert!(validate_message_id("<abc123@example .com>").is_err());
        assert!(validate_message_id("<abc123@example.com >").is_err());
    }

    #[test]
    fn test_validate_message_id_empty_parts() {
        assert!(validate_message_id("<@example.com>").is_err());
        assert!(validate_message_id("<abc123@>").is_err());
    }

    #[test]
    fn test_validate_message_id_multiple_at() {
        assert!(validate_message_id("<abc@123@example.com>").is_err());
    }

    #[test]
    fn test_validate_newsgroup_valid() {
        assert!(validate_newsgroup_name("comp.lang.rust").is_ok());
        assert!(validate_newsgroup_name("alt.binaries.test").is_ok());
        assert!(validate_newsgroup_name("de.comp.lang.c++").is_ok());
        assert!(validate_newsgroup_name("test").is_ok()); // Single component
        assert!(validate_newsgroup_name("alt.test_group").is_ok());
        assert!(validate_newsgroup_name("comp.lang.c++-general").is_ok());
    }

    #[test]
    fn test_validate_newsgroup_empty_component() {
        assert!(validate_newsgroup_name("comp..rust").is_err());
        assert!(validate_newsgroup_name("..").is_err());
    }

    #[test]
    fn test_validate_newsgroup_leading_trailing_dot() {
        assert!(validate_newsgroup_name(".comp.rust").is_err());
        assert!(validate_newsgroup_name("comp.rust.").is_err());
        assert!(validate_newsgroup_name(".").is_err());
    }

    #[test]
    fn test_validate_newsgroup_invalid_chars() {
        assert!(validate_newsgroup_name("comp/lang/rust").is_err());
        assert!(validate_newsgroup_name("comp lang rust").is_err());
        assert!(validate_newsgroup_name("comp.Lang.rust").is_err()); // Uppercase
        assert!(validate_newsgroup_name("comp.lang.rust!").is_err());
    }

    #[test]
    fn test_validate_newsgroup_empty() {
        assert!(validate_newsgroup_name("").is_err());
    }

    #[test]
    fn test_parse_date_rfc5322() {
        let date = parse_date("Mon, 20 Jan 2025 12:00:00 +0000").unwrap();
        assert_eq!(date.year(), 2025);
        assert_eq!(date.month(), 1);
        assert_eq!(date.day(), 20);
    }

    #[test]
    fn test_parse_date_gmt_variant() {
        let date = parse_date("Mon, 20 Jan 2025 12:00:00 GMT").unwrap();
        assert_eq!(date.year(), 2025);
    }

    #[test]
    fn test_parse_date_various_timezones() {
        assert!(parse_date("Mon, 20 Jan 2025 12:00:00 -0500").is_ok());
        assert!(parse_date("Mon, 20 Jan 2025 12:00:00 +0100").is_ok());
        assert!(parse_date("20 Jan 2025 12:00:00 +0000").is_ok());
    }

    #[test]
    fn test_parse_date_invalid() {
        assert!(parse_date("not a date").is_err());
        assert!(parse_date("2025-01-20").is_err()); // ISO format, not RFC 5322
        assert!(parse_date("").is_err());
    }

    #[test]
    fn test_validation_config_strict() {
        let config = ValidationConfig::strict();
        assert!(config.strict_date_validation);
        assert!(!config.allow_future_dates);
        assert!(config.max_date_age_days.is_none());
    }

    #[test]
    fn test_validation_config_lenient() {
        let config = ValidationConfig::lenient();
        assert!(!config.strict_date_validation);
        assert!(config.allow_future_dates);
        assert!(config.max_date_age_days.is_none());
    }

    #[test]
    fn test_validation_config_default() {
        let config = ValidationConfig::default();
        assert_eq!(config, ValidationConfig::strict());
    }

    #[test]
    fn test_validation_config_custom() {
        let config = ValidationConfig {
            strict_date_validation: true,
            allow_future_dates: false,
            max_date_age_days: Some(365),
        };
        assert!(config.strict_date_validation);
        assert!(!config.allow_future_dates);
        assert_eq!(config.max_date_age_days, Some(365));
    }

    #[test]
    fn test_validate_date_current() {
        let now = Utc::now();
        let config = ValidationConfig::strict();
        assert!(validate_date(&now, &config).is_ok());
    }

    #[test]
    fn test_validate_date_future_rejected() {
        let future = Utc::now() + Duration::days(1);
        let config = ValidationConfig::strict();
        assert!(validate_date(&future, &config).is_err());
    }

    #[test]
    fn test_validate_date_future_allowed() {
        let future = Utc::now() + Duration::days(1);
        let config = ValidationConfig::lenient();
        assert!(validate_date(&future, &config).is_ok());
    }

    #[test]
    fn test_validate_date_old_within_limit() {
        let old = Utc::now() - Duration::days(100);
        let config = ValidationConfig {
            strict_date_validation: true,
            allow_future_dates: false,
            max_date_age_days: Some(365),
        };
        assert!(validate_date(&old, &config).is_ok());
    }

    #[test]
    fn test_validate_date_too_old() {
        let too_old = Utc::now() - Duration::days(400);
        let config = ValidationConfig {
            strict_date_validation: true,
            allow_future_dates: false,
            max_date_age_days: Some(365),
        };
        assert!(validate_date(&too_old, &config).is_err());
    }

    #[test]
    fn test_validate_date_no_age_limit() {
        let very_old = Utc::now() - Duration::days(10000);
        let config = ValidationConfig::strict();
        assert!(validate_date(&very_old, &config).is_ok());
    }
}

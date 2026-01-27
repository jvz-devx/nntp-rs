//! NNTP command builders and response parsers

// Module declarations
pub mod article;
pub mod group;
pub mod hdr;
pub mod list;
pub mod over;
pub mod response;

// Re-export all public items for backward compatibility
pub use article::*;
pub use group::*;
pub use hdr::*;
pub use list::*;
pub use over::*;
pub use response::*;

// Authentication and connection management commands

/// Build AUTHINFO USER command
pub fn authinfo_user(username: &str) -> String {
    format!("AUTHINFO USER {}\r\n", username)
}

/// Build AUTHINFO PASS command
pub fn authinfo_pass(password: &str) -> String {
    format!("AUTHINFO PASS {}\r\n", password)
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
///
/// Intentionally unused (RFC 4642 API completeness): TLS support in this library is
/// handled at the connection layer via the `use_tls` configuration option (implicit TLS),
/// not via STARTTLS (explicit upgrade). Most modern deployments use implicit TLS on
/// port 563 rather than STARTTLS on port 119. This function is provided for complete
/// RFC 4642 compliance and may be used in future if explicit TLS upgrade is implemented.
#[allow(dead_code)]
pub fn starttls() -> &'static str {
    "STARTTLS\r\n"
}

// Connection control and session management

/// Build COMPRESS DEFLATE command (RFC 8054)
///
/// Enables full session compression using deflate algorithm.
/// All data after successful negotiation (response 206) is compressed bidirectionally.
/// This provides the best compression but is not supported by all NNTP servers.
pub fn compress_deflate() -> &'static str {
    "COMPRESS DEFLATE\r\n"
}

/// Build XFEATURE COMPRESS GZIP command
///
/// Enables headers-only compression using gzip/zlib.
/// Only multiline responses (XOVER, HEAD, ARTICLE) are compressed after
/// successful negotiation (response 290 or 2xx). More widely supported than
/// RFC 8054 COMPRESS DEFLATE. Provides 50-80% bandwidth reduction.
pub fn xfeature_compress_gzip() -> &'static str {
    "XFEATURE COMPRESS GZIP\r\n"
}

/// Build QUIT command
pub fn quit() -> &'static str {
    "QUIT\r\n"
}

/// Build CAPABILITIES command (RFC 3977 §5.2)
///
/// Requests the list of capabilities supported by the server.
/// Response is multi-line, starting with 101.
pub fn capabilities() -> &'static str {
    "CAPABILITIES\r\n"
}

/// Build CAPABILITIES command with keyword (RFC 3977 §5.2)
///
/// Requests capabilities with optional keyword for specific capability info.
///
/// Intentionally unused (RFC 3977 §5.2 API completeness): The keyword parameter is rarely
/// used in practice - most clients use the basic `capabilities()` command instead. This
/// variant is provided for complete RFC 3977 compliance and potential future use cases
/// where capability-specific queries are needed.
#[allow(dead_code)]
pub fn capabilities_with_keyword(keyword: &str) -> String {
    format!("CAPABILITIES {}\r\n", keyword)
}

/// Build HELP command (RFC 3977 §7.2)
///
/// Requests help text from the server. Response is multi-line, starting with 100.
pub fn help() -> &'static str {
    "HELP\r\n"
}

/// Build DATE command (RFC 3977 §7.1)
///
/// Requests the server's current date and time.
/// Response: 111 yyyymmddhhmmss
pub fn date() -> &'static str {
    "DATE\r\n"
}

/// Build MODE READER command (RFC 3977 §5.3)
///
/// Instructs the server to switch to reader mode (for news reading clients).
pub fn mode_reader() -> &'static str {
    "MODE READER\r\n"
}

/// Build MODE STREAM command (RFC 4644 Section 2.3)
///
/// Requests to switch to streaming mode for efficient bulk article transfer.
/// Response is 203 on success.
pub fn mode_stream() -> &'static str {
    "MODE STREAM\r\n"
}

// Article posting and transfer

/// Build POST command (RFC 3977 §6.3.1)
///
/// Initiates article posting. Server responds with 340 if ready to accept.
/// After receiving 340, client sends article terminated by ".\r\n".
pub fn post() -> &'static str {
    "POST\r\n"
}

/// Build IHAVE command (RFC 3977 §6.2.1)
///
/// Offers an article for transfer by message-id.
/// Server responds with 335 if it wants the article, 435/436 if not.
pub fn ihave(message_id: &str) -> String {
    format!("IHAVE {}\r\n", message_id)
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

#[cfg(test)]
mod tests {
    use super::*;

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
}

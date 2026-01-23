//! NNTP response types and status codes

/// NNTP response with status code, message, and optional multi-line body
#[derive(Debug, Clone)]
pub struct NntpResponse {
    /// 3-digit NNTP response code
    pub code: u16,
    /// Status message from server
    pub message: String,
    /// Multi-line response body (empty for single-line responses)
    pub lines: Vec<String>,
}

/// NNTP binary response optimized for article fetching
///
/// Unlike `NntpResponse` which stores lines as strings, this type stores
/// the raw binary data directly. This avoids:
/// - Line-by-line allocations
/// - UTF-8 validation overhead
/// - Extra copies when rejoining lines
///
/// Use this for `ARTICLE`, `BODY`, and `HEAD` commands where performance matters.
#[derive(Debug, Clone)]
pub struct NntpBinaryResponse {
    /// 3-digit NNTP response code
    pub code: u16,
    /// Status message from server
    pub message: String,
    /// Raw binary response body (with dot-stuffing already removed)
    pub data: Vec<u8>,
}

impl NntpBinaryResponse {
    /// Check if response indicates success (2xx)
    pub fn is_success(&self) -> bool {
        self.code >= 200 && self.code < 300
    }

    /// Check if response indicates error (4xx or 5xx)
    pub fn is_error(&self) -> bool {
        self.code >= 400
    }
}

impl NntpResponse {
    /// Check if response indicates success (2xx)
    pub fn is_success(&self) -> bool {
        self.code >= 200 && self.code < 300
    }

    /// Check if response indicates continuation (3xx)
    pub fn is_continuation(&self) -> bool {
        self.code >= 300 && self.code < 400
    }

    /// Check if response indicates error (4xx or 5xx)
    pub fn is_error(&self) -> bool {
        self.code >= 400
    }
}

/// NNTP response codes (RFC 3977)
#[allow(dead_code)]
pub mod codes {
    // 1xx - Informational
    /// Help text follows
    pub const HELP_TEXT_FOLLOWS: u16 = 100;
    /// Capability list follows (RFC 3977 Section 5.2)
    pub const CAPABILITY_LIST: u16 = 101;
    /// Server date/time (RFC 3977 Section 7.1)
    pub const SERVER_DATE: u16 = 111;

    // 2xx - Success
    /// List of newsgroups follows (RFC 3977 Section 7.6)
    pub const LIST_INFORMATION_FOLLOWS: u16 = 215;
    /// List of new articles follows (RFC 3977 Section 7.4)
    pub const NEW_ARTICLE_LIST_FOLLOWS: u16 = 230;
    /// List of new newsgroups follows (RFC 3977 Section 7.3)
    pub const NEW_NEWSGROUPS_FOLLOW: u16 = 231;
    /// Server ready, posting allowed
    pub const READY_POSTING_ALLOWED: u16 = 200;
    /// Server ready, no posting
    pub const READY_NO_POSTING: u16 = 201;
    /// Slave status noted
    pub const SLAVE_STATUS_NOTED: u16 = 202;
    /// Streaming OK (RFC 4644 Section 2.3)
    pub const STREAMING_OK: u16 = 203;
    /// Closing connection
    pub const CLOSING_CONNECTION: u16 = 205;
    /// Compression active (RFC 8054)
    pub const COMPRESSION_ACTIVE: u16 = 206;
    /// Group selected
    pub const GROUP_SELECTED: u16 = 211;
    /// Article follows
    pub const ARTICLE_FOLLOWS: u16 = 220;
    /// Head follows
    pub const HEAD_FOLLOWS: u16 = 221;
    /// Body follows
    pub const BODY_FOLLOWS: u16 = 222;
    /// Article stat
    pub const ARTICLE_STAT: u16 = 223;
    /// Overview information follows
    pub const OVERVIEW_INFO_FOLLOWS: u16 = 224;
    /// Headers follow
    pub const HEADERS_FOLLOW: u16 = 225;
    /// Article transferred OK (RFC 3977 Section 6.3.2)
    pub const ARTICLE_TRANSFERRED: u16 = 235;
    /// Send article (RFC 4644 Section 2.4)
    pub const CHECK_SEND: u16 = 238;
    /// Article received OK (RFC 4644 Section 2.5)
    pub const TAKETHIS_RECEIVED: u16 = 239;
    /// Article posted successfully (RFC 3977 Section 6.3.1)
    pub const ARTICLE_POSTED: u16 = 240;
    /// Authentication accepted
    pub const AUTH_ACCEPTED: u16 = 281;

    // 3xx - Continuation
    /// Send article to be transferred (RFC 3977 Section 6.3.2)
    pub const SEND_ARTICLE_TRANSFER: u16 = 335;
    /// Send article to be posted
    pub const SEND_ARTICLE: u16 = 340;
    /// Continue with authentication
    pub const AUTH_CONTINUE: u16 = 381;
    /// SASL challenge (RFC 4643 Section 2.4)
    pub const SASL_CONTINUE: u16 = 383;

    // 4xx - Temporary errors
    /// Service temporarily unavailable
    pub const SERVICE_UNAVAILABLE: u16 = 400;
    /// Internal fault or server resource problem (RFC 3977)
    /// Also used for "unable to activate compression" (RFC 8054)
    pub const INTERNAL_FAULT: u16 = 403;
    /// No such newsgroup
    pub const NO_SUCH_GROUP: u16 = 411;
    /// No newsgroup selected
    pub const NO_GROUP_SELECTED: u16 = 412;
    /// No current article
    pub const NO_CURRENT_ARTICLE: u16 = 420;
    /// No next article
    pub const NO_NEXT_ARTICLE: u16 = 421;
    /// No previous article
    pub const NO_PREV_ARTICLE: u16 = 422;
    /// No article with that number
    pub const NO_SUCH_ARTICLE_NUMBER: u16 = 423;
    /// No article with that message-id
    pub const NO_SUCH_ARTICLE_ID: u16 = 430;
    /// Try again later (RFC 4644 Section 2.4)
    pub const CHECK_LATER: u16 = 431;
    /// Article not wanted (RFC 3977 Section 6.3.2)
    pub const ARTICLE_NOT_WANTED: u16 = 435;
    /// Transfer not possible; try again later (RFC 3977 Section 6.3.2)
    pub const TRANSFER_NOT_POSSIBLE: u16 = 436;
    /// Transfer rejected; do not retry (RFC 3977 Section 6.3.2)
    pub const TRANSFER_REJECTED: u16 = 437;
    /// Article not wanted (RFC 4644 Section 2.4)
    pub const CHECK_NOT_WANTED: u16 = 438;
    /// Article rejected (RFC 4644 Section 2.5)
    pub const TAKETHIS_REJECTED: u16 = 439;
    /// Posting not permitted (RFC 3977 Section 6.3.1)
    pub const POSTING_NOT_PERMITTED: u16 = 440;
    /// Posting failed (RFC 3977 Section 6.3.1)
    pub const POSTING_FAILED: u16 = 441;
    /// Authentication required (RFC 4643)
    pub const AUTH_REQUIRED: u16 = 480;
    /// Authentication rejected
    pub const AUTH_REJECTED: u16 = 481;
    /// Authentication out of sequence
    pub const AUTH_OUT_OF_SEQUENCE: u16 = 482;
    /// Encryption or authentication required (RFC 4643)
    pub const ENCRYPTION_REQUIRED: u16 = 483;

    // 5xx - Permanent errors
    /// Command not recognized
    pub const COMMAND_NOT_RECOGNIZED: u16 = 500;
    /// Command syntax error
    pub const COMMAND_SYNTAX_ERROR: u16 = 501;
    /// Access denied / command unavailable
    pub const ACCESS_DENIED: u16 = 502;
    /// Feature not supported / optional functionality absent (RFC 3977)
    pub const FEATURE_NOT_SUPPORTED: u16 = 503;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_success() {
        let response = NntpResponse {
            code: 200,
            message: "Ready".to_string(),
            lines: vec![],
        };
        assert!(response.is_success());
        assert!(!response.is_continuation());
        assert!(!response.is_error());
    }

    #[test]
    fn test_is_continuation() {
        let response = NntpResponse {
            code: 381,
            message: "Continue".to_string(),
            lines: vec![],
        };
        assert!(!response.is_success());
        assert!(response.is_continuation());
        assert!(!response.is_error());
    }

    #[test]
    fn test_is_error() {
        let response = NntpResponse {
            code: 481,
            message: "Auth rejected".to_string(),
            lines: vec![],
        };
        assert!(!response.is_success());
        assert!(!response.is_continuation());
        assert!(response.is_error());
    }

    #[test]
    fn test_boundary_codes() {
        // 199 is not success
        assert!(!NntpResponse {
            code: 199,
            message: String::new(),
            lines: vec![]
        }
        .is_success());
        // 200 is success
        assert!(NntpResponse {
            code: 200,
            message: String::new(),
            lines: vec![]
        }
        .is_success());
        // 299 is success
        assert!(NntpResponse {
            code: 299,
            message: String::new(),
            lines: vec![]
        }
        .is_success());
        // 300 is not success
        assert!(!NntpResponse {
            code: 300,
            message: String::new(),
            lines: vec![]
        }
        .is_success());
    }
}

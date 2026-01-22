//! RFC 5536 Article Format
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc5536
//!
//! Provides structures for representing and working with Usenet articles
//! conforming to RFC 5536 (Netnews Article Format).

use std::collections::HashMap;

use crate::encoded_words::decode_header_value;
use crate::{NntpError, Result};

/// Netnews article structure (RFC 5536)
///
/// An article consists of headers and a body, separated by a blank line.
/// Articles must conform to RFC 5536 and include all required headers.
///
/// # Required Headers (RFC 5536 Section 3.1)
///
/// - Date: When the article was created
/// - From: Author's identity
/// - Message-ID: Unique identifier
/// - Newsgroups: Target newsgroups (comma-separated)
/// - Path: Transit path (managed by servers)
/// - Subject: Article subject line
///
/// # Examples
///
/// ```
/// use nntp_rs::article::{Article, Headers};
/// use std::collections::HashMap;
///
/// let headers = Headers {
///     date: "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
///     from: "user@example.com".to_string(),
///     message_id: "<abc123@example.com>".to_string(),
///     newsgroups: vec!["comp.lang.rust".to_string()],
///     path: "news.example.com!not-for-mail".to_string(),
///     subject: "Test Article".to_string(),
///     references: None,
///     reply_to: None,
///     organization: None,
///     followup_to: None,
///     expires: None,
///     control: None,
///     distribution: None,
///     keywords: None,
///     summary: None,
///     supersedes: None,
///     approved: None,
///     lines: None,
///     user_agent: None,
///     xref: None,
///     extra: HashMap::new(),
/// };
///
/// // In practice, use ArticleBuilder to create articles
/// use nntp_rs::article::ArticleBuilder;
///
/// let article = ArticleBuilder::new()
///     .subject("Test Article")
///     .newsgroups(vec!["comp.lang.rust"])
///     .from("user@example.com")
///     .body("This is the article body.")
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct Article {
    /// Article headers
    pub headers: Headers,
    /// Article body (after blank line separator)
    pub body: String,
    /// Original raw article text for round-trip preservation
    pub(crate) raw: Option<String>,
}

/// Netnews article headers (RFC 5536)
///
/// Contains all standard headers defined in RFC 5536, plus an `extra`
/// HashMap for non-standard extension headers.
#[derive(Debug, Clone)]
pub struct Headers {
    // Required headers (RFC 5536 Section 3.1)
    /// Date when article was created (RFC 5536 Section 3.1.1)
    /// Format: RFC 5322 date-time (e.g., "Mon, 20 Jan 2025 12:00:00 +0000")
    pub date: String,

    /// Author's identity (RFC 5536 Section 3.1.2)
    /// Format: RFC 5322 mailbox (e.g., "John Doe <user@example.com>")
    pub from: String,

    /// Unique article identifier (RFC 5536 Section 3.1.3)
    /// Format: "<local-part@domain>" (e.g., "<abc123@example.com>")
    pub message_id: String,

    /// Target newsgroups, comma-separated (RFC 5536 Section 3.1.4)
    /// Example: ["comp.lang.rust", "comp.lang.c"]
    pub newsgroups: Vec<String>,

    /// Transit path through servers (RFC 5536 Section 3.1.5)
    /// Format: "server1!server2!not-for-mail"
    /// Managed by news servers, typically not set by clients
    pub path: String,

    /// Article subject line (RFC 5536 Section 3.1.6)
    pub subject: String,

    // Optional headers (RFC 5536 Section 3.2)
    /// References to previous articles in thread (RFC 5536 Section 3.2.12)
    /// Format: List of message-IDs (e.g., ["<msg1@example.com>", "<msg2@example.com>"])
    pub references: Option<Vec<String>>,

    /// Reply-To address (RFC 5536 Section 3.2.13)
    /// Format: RFC 5322 mailbox list
    pub reply_to: Option<String>,

    /// Poster's organization (RFC 5536 Section 3.2.10)
    pub organization: Option<String>,

    /// Where followups should be directed (RFC 5536 Section 3.2.3)
    /// Format: Comma-separated newsgroup list or "poster" keyword
    pub followup_to: Option<Vec<String>>,

    /// Expiration date for the article (RFC 5536 Section 3.2.2)
    /// Format: RFC 5322 date-time
    pub expires: Option<String>,

    /// Control message type (RFC 5536 Section 3.2.1)
    /// Format: "command arguments" (e.g., "cancel <msg-id>")
    pub control: Option<String>,

    /// Distribution scope (RFC 5536 Section 3.2.17)
    /// Example: "local", "world"
    pub distribution: Option<String>,

    /// Article keywords (RFC 5536 Section 3.2.8)
    /// Format: Comma-separated list
    pub keywords: Option<String>,

    /// Article summary (RFC 5536 Section 3.2.14)
    pub summary: Option<String>,

    /// Message-ID of article being replaced (RFC 5536 Section 3.2.12)
    /// Format: Single message-ID (e.g., "<old-msg-id@example.com>")
    /// Mutually exclusive with Control header (RFC 5536 Section 3.2.12)
    pub supersedes: Option<String>,

    /// Moderator approval (RFC 5536 Section 3.2.18)
    /// Required for posting to moderated groups
    pub approved: Option<String>,

    /// Number of lines in body (RFC 5536 Section 3.2.9)
    pub lines: Option<u32>,

    /// Client software identification (RFC 5536 Section 3.2.16)
    pub user_agent: Option<String>,

    /// Cross-reference information (RFC 5536 Section 3.2.15)
    /// Format: "server group:number group:number"
    pub xref: Option<String>,

    /// Additional non-standard headers
    /// Includes X-* headers and other extensions
    pub extra: HashMap<String, String>,
}

impl Article {
    /// Create a new article with the given headers and body
    pub fn new(headers: Headers, body: String) -> Self {
        Self {
            headers,
            body,
            raw: None,
        }
    }

    /// Get the raw article text if available
    pub fn raw(&self) -> Option<&str> {
        self.raw.as_deref()
    }

    /// Check if this article is a control message (RFC 5537 Section 5)
    ///
    /// Returns `true` if the article has a Control header, indicating it
    /// should trigger administrative actions rather than just being displayed.
    ///
    /// # Examples
    ///
    /// ```
    /// use nntp_rs::article::{Article, Headers};
    /// use std::collections::HashMap;
    ///
    /// let mut headers = Headers::new(
    ///     "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
    ///     "admin@example.com".to_string(),
    ///     "<cancel123@example.com>".to_string(),
    ///     vec!["comp.lang.rust".to_string()],
    ///     "news.example.com!not-for-mail".to_string(),
    ///     "cancel <spam123@example.com>".to_string(),
    /// );
    /// headers.control = Some("cancel <spam123@example.com>".to_string());
    ///
    /// let article = Article::new(headers, String::new());
    /// assert!(article.is_control_message());
    /// ```
    pub fn is_control_message(&self) -> bool {
        self.headers.control.is_some()
    }

    /// Parse the control message type from the Control header (RFC 5537 Section 5)
    ///
    /// Extracts the control command and arguments from the Control header field.
    /// Returns `None` if this is not a control message or if the Control header
    /// is malformed.
    ///
    /// # Control Message Types
    ///
    /// - **cancel** - Withdraws an article (RFC 5537 Section 5.3)
    /// - **newgroup** - Creates or modifies a newsgroup (RFC 5537 Section 5.2.1)
    /// - **rmgroup** - Removes a newsgroup (RFC 5537 Section 5.2.2)
    /// - **checkgroups** - Provides authoritative group list (RFC 5537 Section 5.2.3)
    /// - **ihave** - Legacy peer-to-peer article exchange (RFC 5537 Section 5.5)
    /// - **sendme** - Legacy peer-to-peer article exchange (RFC 5537 Section 5.5)
    ///
    /// # Examples
    ///
    /// ```
    /// use nntp_rs::article::{Article, Headers, ControlMessage};
    /// use std::collections::HashMap;
    ///
    /// let mut headers = Headers::new(
    ///     "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
    ///     "admin@example.com".to_string(),
    ///     "<cancel123@example.com>".to_string(),
    ///     vec!["comp.lang.rust".to_string()],
    ///     "news.example.com!not-for-mail".to_string(),
    ///     "cancel message".to_string(),
    /// );
    /// headers.control = Some("cancel <spam123@example.com>".to_string());
    ///
    /// let article = Article::new(headers, String::new());
    /// match article.parse_control_message() {
    ///     Some(ControlMessage::Cancel { message_id }) => {
    ///         assert_eq!(message_id, "<spam123@example.com>");
    ///     }
    ///     _ => panic!("Expected cancel control message"),
    /// }
    /// ```
    pub fn parse_control_message(&self) -> Option<ControlMessage> {
        let control = self.headers.control.as_ref()?;
        ControlMessage::parse(control)
    }

    /// Check if this article has MIME content (RFC 5536 Section 4)
    ///
    /// Returns `true` if the article contains a Content-Type header in its
    /// extra headers, indicating that the body uses MIME formatting.
    ///
    /// # Examples
    ///
    /// ```
    /// use nntp_rs::article::{Article, Headers};
    /// use std::collections::HashMap;
    ///
    /// let mut headers = Headers::new(
    ///     "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
    ///     "user@example.com".to_string(),
    ///     "<msg123@example.com>".to_string(),
    ///     vec!["comp.lang.rust".to_string()],
    ///     "news.example.com!not-for-mail".to_string(),
    ///     "Test Article".to_string(),
    /// );
    /// headers.extra.insert("Content-Type".to_string(), "text/plain; charset=utf-8".to_string());
    ///
    /// let article = Article::new(headers, "Article body".to_string());
    /// assert!(article.is_mime());
    /// ```
    pub fn is_mime(&self) -> bool {
        self.headers.extra.contains_key("Content-Type")
    }

    /// Get the Content-Type header value (RFC 5536 Section 4)
    ///
    /// Returns the Content-Type header if present, or `None` if this is not
    /// a MIME article. The Content-Type header specifies the media type and
    /// optional parameters like charset.
    ///
    /// # Examples
    ///
    /// ```
    /// use nntp_rs::article::{Article, Headers};
    /// use std::collections::HashMap;
    ///
    /// let mut headers = Headers::new(
    ///     "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
    ///     "user@example.com".to_string(),
    ///     "<msg123@example.com>".to_string(),
    ///     vec!["comp.lang.rust".to_string()],
    ///     "news.example.com!not-for-mail".to_string(),
    ///     "Test Article".to_string(),
    /// );
    /// headers.extra.insert("Content-Type".to_string(), "text/plain; charset=utf-8".to_string());
    ///
    /// let article = Article::new(headers, "Article body".to_string());
    /// assert_eq!(article.content_type(), Some("text/plain; charset=utf-8"));
    /// ```
    pub fn content_type(&self) -> Option<&str> {
        self.headers.extra.get("Content-Type").map(|s| s.as_str())
    }

    /// Check if this article is a multipart MIME message (RFC 5536 Section 4)
    ///
    /// Returns `true` if the Content-Type header starts with "multipart/",
    /// indicating that the body contains multiple parts separated by a boundary.
    ///
    /// # Examples
    ///
    /// ```
    /// use nntp_rs::article::{Article, Headers};
    /// use std::collections::HashMap;
    ///
    /// let mut headers = Headers::new(
    ///     "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
    ///     "user@example.com".to_string(),
    ///     "<msg123@example.com>".to_string(),
    ///     vec!["comp.lang.rust".to_string()],
    ///     "news.example.com!not-for-mail".to_string(),
    ///     "Test Article".to_string(),
    /// );
    /// headers.extra.insert(
    ///     "Content-Type".to_string(),
    ///     "multipart/mixed; boundary=\"boundary123\"".to_string()
    /// );
    ///
    /// let article = Article::new(headers, "Article body".to_string());
    /// assert!(article.is_multipart());
    /// ```
    pub fn is_multipart(&self) -> bool {
        self.content_type()
            .map(|ct| ct.trim().to_lowercase().starts_with("multipart/"))
            .unwrap_or(false)
    }

    /// Extract the charset parameter from the Content-Type header (RFC 5536 Section 4)
    ///
    /// Returns the charset parameter value if present in the Content-Type header.
    /// Common values include "utf-8", "iso-8859-1", "windows-1252", etc.
    ///
    /// # Examples
    ///
    /// ```
    /// use nntp_rs::article::{Article, Headers};
    /// use std::collections::HashMap;
    ///
    /// let mut headers = Headers::new(
    ///     "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
    ///     "user@example.com".to_string(),
    ///     "<msg123@example.com>".to_string(),
    ///     vec!["comp.lang.rust".to_string()],
    ///     "news.example.com!not-for-mail".to_string(),
    ///     "Test Article".to_string(),
    /// );
    /// headers.extra.insert(
    ///     "Content-Type".to_string(),
    ///     "text/plain; charset=utf-8".to_string()
    /// );
    ///
    /// let article = Article::new(headers, "Article body".to_string());
    /// assert_eq!(article.charset(), Some("utf-8"));
    /// ```
    pub fn charset(&self) -> Option<&str> {
        let content_type = self.content_type()?;

        // Look for charset parameter in Content-Type
        // Format: "text/plain; charset=utf-8" or "text/plain; charset=\"utf-8\""
        for param in content_type.split(';') {
            let param = param.trim();

            // Handle "charset=value" or "charset = value" with optional whitespace
            if let Some(eq_pos) = param.find('=') {
                let key = param[..eq_pos].trim();
                if key.eq_ignore_ascii_case("charset") {
                    let value = param[eq_pos + 1..].trim();
                    // Remove quotes if present
                    return Some(value.trim_matches('"').trim_matches('\''));
                }
            }
        }

        None
    }
}

/// Control message types (RFC 5537 Section 5)
///
/// Control messages are special articles that trigger administrative actions
/// on news servers rather than being displayed to users.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlMessage {
    /// Cancel an article (RFC 5537 Section 5.3)
    ///
    /// Format: `cancel <message-id>`
    ///
    /// Withdraws an article from circulation. The message-id specifies
    /// which article to cancel.
    Cancel {
        /// Message-ID of the article to cancel
        message_id: String,
    },

    /// Create or modify a newsgroup (RFC 5537 Section 5.2.1)
    ///
    /// Format: `newgroup <newsgroup-name> [moderated]`
    ///
    /// Creates a new newsgroup or modifies an existing one. The optional
    /// `moderated` keyword indicates the group should be moderated.
    Newgroup {
        /// Name of the newsgroup to create
        group: String,
        /// Whether the group should be moderated
        moderated: bool,
    },

    /// Remove a newsgroup (RFC 5537 Section 5.2.2)
    ///
    /// Format: `rmgroup <newsgroup-name>`
    ///
    /// Removes a newsgroup from the server.
    Rmgroup {
        /// Name of the newsgroup to remove
        group: String,
    },

    /// Provide authoritative group list (RFC 5537 Section 5.2.3)
    ///
    /// Format: `checkgroups [scope] [#serial-number]`
    ///
    /// Provides an authoritative list of valid newsgroups for a hierarchy.
    Checkgroups {
        /// Optional scope/hierarchy
        scope: Option<String>,
        /// Optional serial number for versioning
        serial: Option<String>,
    },

    /// Legacy peer-to-peer article exchange (RFC 5537 Section 5.5)
    ///
    /// Format: `ihave <msg-id> [<msg-id>...] <relayer-name>`
    ///
    /// Largely obsolete. Use NNTP IHAVE command instead (RFC 3977 Section 6.3.2).
    Ihave {
        /// List of message-IDs being offered
        message_ids: Vec<String>,
        /// Name of the relaying server
        relayer: Option<String>,
    },

    /// Legacy peer-to-peer article exchange (RFC 5537 Section 5.5)
    ///
    /// Format: `sendme <msg-id> [<msg-id>...] <relayer-name>`
    ///
    /// Largely obsolete. Requests articles from a peer.
    Sendme {
        /// List of message-IDs being requested
        message_ids: Vec<String>,
        /// Name of the relaying server
        relayer: Option<String>,
    },

    /// Unknown or unrecognized control message type
    ///
    /// Contains the raw control header value for custom handling.
    Unknown {
        /// The raw Control header value
        value: String,
    },
}

impl ControlMessage {
    /// Parse a control message from a Control header value
    ///
    /// # Arguments
    ///
    /// * `control` - The value of the Control header field
    ///
    /// # Examples
    ///
    /// ```
    /// use nntp_rs::article::ControlMessage;
    ///
    /// let msg = ControlMessage::parse("cancel <spam@example.com>").unwrap();
    /// match msg {
    ///     ControlMessage::Cancel { message_id } => {
    ///         assert_eq!(message_id, "<spam@example.com>");
    ///     }
    ///     _ => panic!("Expected cancel"),
    /// }
    ///
    /// let msg = ControlMessage::parse("newgroup comp.lang.rust moderated").unwrap();
    /// match msg {
    ///     ControlMessage::Newgroup { group, moderated } => {
    ///         assert_eq!(group, "comp.lang.rust");
    ///         assert!(moderated);
    ///     }
    ///     _ => panic!("Expected newgroup"),
    /// }
    /// ```
    pub fn parse(control: &str) -> Option<ControlMessage> {
        let control = control.trim();
        if control.is_empty() {
            return None;
        }

        let parts: Vec<&str> = control.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        let command = parts[0].to_lowercase();

        match command.as_str() {
            "cancel" => {
                // Format: cancel <message-id>
                if parts.len() < 2 {
                    return Some(ControlMessage::Unknown {
                        value: control.to_string(),
                    });
                }
                Some(ControlMessage::Cancel {
                    message_id: parts[1].to_string(),
                })
            }
            "newgroup" => {
                // Format: newgroup <group> [moderated]
                if parts.len() < 2 {
                    return Some(ControlMessage::Unknown {
                        value: control.to_string(),
                    });
                }
                let group = parts[1].to_string();
                let moderated = parts
                    .get(2)
                    .map(|s| s.to_lowercase() == "moderated")
                    .unwrap_or(false);
                Some(ControlMessage::Newgroup { group, moderated })
            }
            "rmgroup" => {
                // Format: rmgroup <group>
                if parts.len() < 2 {
                    return Some(ControlMessage::Unknown {
                        value: control.to_string(),
                    });
                }
                Some(ControlMessage::Rmgroup {
                    group: parts[1].to_string(),
                })
            }
            "checkgroups" => {
                // Format: checkgroups [scope] [#serial]
                let scope = parts
                    .get(1)
                    .filter(|s| !s.starts_with('#'))
                    .map(|s| s.to_string());
                let serial = parts
                    .iter()
                    .find(|s| s.starts_with('#'))
                    .map(|s| s.to_string());
                Some(ControlMessage::Checkgroups { scope, serial })
            }
            "ihave" => {
                // Format: ihave <msg-id> [<msg-id>...] <relayer-name>
                if parts.len() < 2 {
                    return Some(ControlMessage::Unknown {
                        value: control.to_string(),
                    });
                }
                // Last part might be relayer name (if it doesn't look like a message-id)
                let (message_ids, relayer) =
                    if parts.len() > 2 && !parts[parts.len() - 1].starts_with('<') {
                        let relayer = Some(parts[parts.len() - 1].to_string());
                        let ids: Vec<String> = parts[1..parts.len() - 1]
                            .iter()
                            .map(|s| s.to_string())
                            .collect();
                        (ids, relayer)
                    } else {
                        let ids: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
                        (ids, None)
                    };
                Some(ControlMessage::Ihave {
                    message_ids,
                    relayer,
                })
            }
            "sendme" => {
                // Format: sendme <msg-id> [<msg-id>...] <relayer-name>
                if parts.len() < 2 {
                    return Some(ControlMessage::Unknown {
                        value: control.to_string(),
                    });
                }
                // Last part might be relayer name (if it doesn't look like a message-id)
                let (message_ids, relayer) =
                    if parts.len() > 2 && !parts[parts.len() - 1].starts_with('<') {
                        let relayer = Some(parts[parts.len() - 1].to_string());
                        let ids: Vec<String> = parts[1..parts.len() - 1]
                            .iter()
                            .map(|s| s.to_string())
                            .collect();
                        (ids, relayer)
                    } else {
                        let ids: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
                        (ids, None)
                    };
                Some(ControlMessage::Sendme {
                    message_ids,
                    relayer,
                })
            }
            _ => {
                // Unknown control message type
                Some(ControlMessage::Unknown {
                    value: control.to_string(),
                })
            }
        }
    }
}

impl Headers {
    /// Create a new Headers struct with required fields
    ///
    /// # Arguments
    ///
    /// * `date` - RFC 5322 date-time string
    /// * `from` - Author mailbox
    /// * `message_id` - Unique message identifier
    /// * `newsgroups` - List of target newsgroups
    /// * `path` - Server transit path
    /// * `subject` - Article subject
    pub fn new(
        date: String,
        from: String,
        message_id: String,
        newsgroups: Vec<String>,
        path: String,
        subject: String,
    ) -> Self {
        Self {
            date,
            from,
            message_id,
            newsgroups,
            path,
            subject,
            references: None,
            reply_to: None,
            organization: None,
            followup_to: None,
            expires: None,
            control: None,
            distribution: None,
            keywords: None,
            summary: None,
            supersedes: None,
            approved: None,
            lines: None,
            user_agent: None,
            xref: None,
            extra: HashMap::new(),
        }
    }

    /// Validates all header fields according to RFC 5536 specifications
    ///
    /// Performs comprehensive validation of all header fields:
    /// - Checks that required fields are non-empty
    /// - Validates Message-ID format
    /// - Validates newsgroup names
    /// - Parses and validates date format and constraints
    /// - Checks mutual exclusivity of Supersedes and Control headers
    ///
    /// # Arguments
    ///
    /// * `config` - Validation configuration (controls date validation behavior)
    ///
    /// # Examples
    ///
    /// ```
    /// use nntp_rs::article::Headers;
    /// use nntp_rs::validation::ValidationConfig;
    /// use std::collections::HashMap;
    ///
    /// let headers = Headers {
    ///     date: "Tue, 20 Jan 2026 12:00:00 +0000".to_string(),
    ///     from: "user@example.com".to_string(),
    ///     message_id: "<abc123@example.com>".to_string(),
    ///     newsgroups: vec!["comp.lang.rust".to_string()],
    ///     path: "news.example.com!not-for-mail".to_string(),
    ///     subject: "Test Article".to_string(),
    ///     references: None,
    ///     reply_to: None,
    ///     organization: None,
    ///     followup_to: None,
    ///     expires: None,
    ///     control: None,
    ///     distribution: None,
    ///     keywords: None,
    ///     summary: None,
    ///     supersedes: None,
    ///     approved: None,
    ///     lines: None,
    ///     user_agent: None,
    ///     xref: None,
    ///     extra: HashMap::new(),
    /// };
    ///
    /// let config = ValidationConfig::default();
    /// headers.validate(&config).unwrap();
    /// ```
    pub fn validate(&self, config: &crate::validation::ValidationConfig) -> Result<()> {
        // Validate required fields are non-empty
        if self.date.trim().is_empty() {
            return Err(NntpError::InvalidResponse(
                "Date header cannot be empty".to_string(),
            ));
        }
        if self.from.trim().is_empty() {
            return Err(NntpError::InvalidResponse(
                "From header cannot be empty".to_string(),
            ));
        }
        if self.message_id.trim().is_empty() {
            return Err(NntpError::InvalidResponse(
                "Message-ID header cannot be empty".to_string(),
            ));
        }
        if self.newsgroups.is_empty() {
            return Err(NntpError::InvalidResponse(
                "Newsgroups header cannot be empty".to_string(),
            ));
        }
        if self.path.trim().is_empty() {
            return Err(NntpError::InvalidResponse(
                "Path header cannot be empty".to_string(),
            ));
        }
        if self.subject.trim().is_empty() {
            return Err(NntpError::InvalidResponse(
                "Subject header cannot be empty".to_string(),
            ));
        }

        // Validate Message-ID format
        crate::validation::validate_message_id(&self.message_id)?;

        // Validate all newsgroup names
        for newsgroup in &self.newsgroups {
            crate::validation::validate_newsgroup_name(newsgroup)?;
        }

        // Validate followup_to newsgroups if present
        if let Some(ref followup_to) = self.followup_to {
            for newsgroup in followup_to {
                // "poster" is a special keyword in Followup-To, not a newsgroup
                if newsgroup != "poster" {
                    crate::validation::validate_newsgroup_name(newsgroup)?;
                }
            }
        }

        // Parse and validate date
        let parsed_date = crate::validation::parse_date(&self.date)?;
        crate::validation::validate_date(&parsed_date, config)?;

        // Validate expires date if present
        if let Some(ref expires) = self.expires {
            let expires_date = crate::validation::parse_date(expires)?;
            // Expires should be in the future or current (not validated with config)
            // Just validate it's a valid date format
            let _ = expires_date;
        }

        // Validate References message-IDs if present
        if let Some(ref references) = self.references {
            for reference in references {
                crate::validation::validate_message_id(reference)?;
            }
        }

        // Validate Supersedes message-ID if present
        if let Some(ref supersedes) = self.supersedes {
            crate::validation::validate_message_id(supersedes)?;
        }

        // Check mutual exclusivity: Supersedes and Control (RFC 5536 Section 3.2.12)
        if self.supersedes.is_some() && self.control.is_some() {
            return Err(NntpError::InvalidResponse(
                "Article cannot have both Supersedes and Control headers".to_string(),
            ));
        }

        Ok(())
    }

    /// Parses the Path header into individual server components
    ///
    /// The Path header contains a "bang path" of servers that the article
    /// passed through, separated by '!' characters. Servers are listed in
    /// reverse chronological order (most recent first).
    ///
    /// # Examples
    ///
    /// ```
    /// use nntp_rs::article::Headers;
    /// use std::collections::HashMap;
    ///
    /// let headers = Headers {
    ///     date: "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
    ///     from: "user@example.com".to_string(),
    ///     message_id: "<abc123@example.com>".to_string(),
    ///     newsgroups: vec!["comp.lang.rust".to_string()],
    ///     path: "news1.example.com!news2.example.net!not-for-mail".to_string(),
    ///     subject: "Test".to_string(),
    ///     references: None,
    ///     reply_to: None,
    ///     organization: None,
    ///     followup_to: None,
    ///     expires: None,
    ///     control: None,
    ///     distribution: None,
    ///     keywords: None,
    ///     summary: None,
    ///     supersedes: None,
    ///     approved: None,
    ///     lines: None,
    ///     user_agent: None,
    ///     xref: None,
    ///     extra: HashMap::new(),
    /// };
    ///
    /// let path_components = headers.parse_path();
    /// assert_eq!(path_components, vec!["news1.example.com", "news2.example.net", "not-for-mail"]);
    /// ```
    pub fn parse_path(&self) -> Vec<String> {
        self.path
            .split('!')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    }

    /// Returns the originating server from the Path header
    ///
    /// The originating server is the first component of the path,
    /// representing the most recent server to handle the article.
    ///
    /// # Examples
    ///
    /// ```
    /// use nntp_rs::article::Headers;
    /// use std::collections::HashMap;
    ///
    /// let headers = Headers {
    ///     date: "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
    ///     from: "user@example.com".to_string(),
    ///     message_id: "<abc123@example.com>".to_string(),
    ///     newsgroups: vec!["comp.lang.rust".to_string()],
    ///     path: "news1.example.com!news2.example.net!not-for-mail".to_string(),
    ///     subject: "Test".to_string(),
    ///     references: None,
    ///     reply_to: None,
    ///     organization: None,
    ///     followup_to: None,
    ///     expires: None,
    ///     control: None,
    ///     distribution: None,
    ///     keywords: None,
    ///     summary: None,
    ///     supersedes: None,
    ///     approved: None,
    ///     lines: None,
    ///     user_agent: None,
    ///     xref: None,
    ///     extra: HashMap::new(),
    /// };
    ///
    /// assert_eq!(headers.originating_server(), Some("news1.example.com"));
    /// ```
    pub fn originating_server(&self) -> Option<&str> {
        self.path.split('!').next().filter(|s| !s.trim().is_empty())
    }

    /// Returns the number of servers in the Path header
    ///
    /// This represents the number of "hops" the article has made
    /// through the Usenet infrastructure.
    ///
    /// # Examples
    ///
    /// ```
    /// use nntp_rs::article::Headers;
    /// use std::collections::HashMap;
    ///
    /// let headers = Headers {
    ///     date: "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
    ///     from: "user@example.com".to_string(),
    ///     message_id: "<abc123@example.com>".to_string(),
    ///     newsgroups: vec!["comp.lang.rust".to_string()],
    ///     path: "news1.example.com!news2.example.net!not-for-mail".to_string(),
    ///     subject: "Test".to_string(),
    ///     references: None,
    ///     reply_to: None,
    ///     organization: None,
    ///     followup_to: None,
    ///     expires: None,
    ///     control: None,
    ///     distribution: None,
    ///     keywords: None,
    ///     summary: None,
    ///     supersedes: None,
    ///     approved: None,
    ///     lines: None,
    ///     user_agent: None,
    ///     xref: None,
    ///     extra: HashMap::new(),
    /// };
    ///
    /// assert_eq!(headers.path_length(), 3);
    /// ```
    pub fn path_length(&self) -> usize {
        self.parse_path().len()
    }
}

/// Parse raw article text into headers and body
///
/// Splits article at the first blank line (CRLF CRLF or LF LF).
/// Returns (headers_text, body_text) tuple.
fn split_article(raw: &str) -> (&str, &str) {
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
fn parse_comma_list(value: &str) -> Vec<String> {
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
fn parse_message_id_list(value: &str) -> Vec<String> {
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
fn unfold_header(value: &str) -> String {
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

/// Builder for constructing RFC 5536-compliant Usenet articles
///
/// Provides a convenient API for creating articles with automatic generation
/// of required headers like Date and Message-ID.
///
/// # Examples
///
/// ```
/// use nntp_rs::article::ArticleBuilder;
///
/// let article = ArticleBuilder::new()
///     .from("user@example.com")
///     .subject("Test Article")
///     .newsgroups(vec!["comp.lang.rust"])
///     .body("This is the article body.")
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct ArticleBuilder {
    from: Option<String>,
    subject: Option<String>,
    newsgroups: Vec<String>,
    body: String,
    // Optional headers
    date: Option<String>,
    message_id: Option<String>,
    path: Option<String>,
    references: Option<Vec<String>>,
    reply_to: Option<String>,
    organization: Option<String>,
    followup_to: Option<Vec<String>>,
    expires: Option<String>,
    control: Option<String>,
    distribution: Option<String>,
    keywords: Option<String>,
    summary: Option<String>,
    supersedes: Option<String>,
    approved: Option<String>,
    user_agent: Option<String>,
    extra: HashMap<String, String>,
}

impl Default for ArticleBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ArticleBuilder {
    /// Create a new ArticleBuilder with default values
    pub fn new() -> Self {
        Self {
            from: None,
            subject: None,
            newsgroups: Vec::new(),
            body: String::new(),
            date: None,
            message_id: None,
            path: None,
            references: None,
            reply_to: None,
            organization: None,
            followup_to: None,
            expires: None,
            control: None,
            distribution: None,
            keywords: None,
            summary: None,
            supersedes: None,
            approved: None,
            user_agent: None,
            extra: HashMap::new(),
        }
    }

    /// Set the From header (required)
    pub fn from(mut self, from: impl Into<String>) -> Self {
        self.from = Some(from.into());
        self
    }

    /// Set the Subject header (required)
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// Set the Newsgroups header (required, at least one newsgroup)
    pub fn newsgroups(mut self, newsgroups: Vec<impl Into<String>>) -> Self {
        self.newsgroups = newsgroups.into_iter().map(|s| s.into()).collect();
        self
    }

    /// Add a single newsgroup
    pub fn add_newsgroup(mut self, newsgroup: impl Into<String>) -> Self {
        self.newsgroups.push(newsgroup.into());
        self
    }

    /// Set the article body
    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = body.into();
        self
    }

    /// Set the Date header (auto-generated if not provided)
    pub fn date(mut self, date: impl Into<String>) -> Self {
        self.date = Some(date.into());
        self
    }

    /// Set the Message-ID header (auto-generated if not provided)
    pub fn message_id(mut self, message_id: impl Into<String>) -> Self {
        self.message_id = Some(message_id.into());
        self
    }

    /// Set the Path header (default: "not-for-mail")
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Set the References header for threading
    pub fn references(mut self, references: Vec<impl Into<String>>) -> Self {
        self.references = Some(references.into_iter().map(|s| s.into()).collect());
        self
    }

    /// Set the Reply-To header
    pub fn reply_to(mut self, reply_to: impl Into<String>) -> Self {
        self.reply_to = Some(reply_to.into());
        self
    }

    /// Set the Organization header
    pub fn organization(mut self, organization: impl Into<String>) -> Self {
        self.organization = Some(organization.into());
        self
    }

    /// Set the Followup-To header
    pub fn followup_to(mut self, followup_to: Vec<impl Into<String>>) -> Self {
        self.followup_to = Some(followup_to.into_iter().map(|s| s.into()).collect());
        self
    }

    /// Set the Expires header
    pub fn expires(mut self, expires: impl Into<String>) -> Self {
        self.expires = Some(expires.into());
        self
    }

    /// Set the Control header
    pub fn control(mut self, control: impl Into<String>) -> Self {
        self.control = Some(control.into());
        self
    }

    /// Set the Distribution header
    pub fn distribution(mut self, distribution: impl Into<String>) -> Self {
        self.distribution = Some(distribution.into());
        self
    }

    /// Set the Keywords header
    pub fn keywords(mut self, keywords: impl Into<String>) -> Self {
        self.keywords = Some(keywords.into());
        self
    }

    /// Set the Summary header
    pub fn summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// Set the Supersedes header (RFC 5536 Section 3.2.12)
    ///
    /// Indicates this article replaces a previous article.
    /// Mutually exclusive with the Control header.
    pub fn supersedes(mut self, message_id: impl Into<String>) -> Self {
        self.supersedes = Some(message_id.into());
        self
    }

    /// Set the Approved header (for moderated groups)
    pub fn approved(mut self, approved: impl Into<String>) -> Self {
        self.approved = Some(approved.into());
        self
    }

    /// Set the User-Agent header
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    /// Add a custom header
    pub fn extra_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra.insert(name.into(), value.into());
        self
    }

    /// Build the article, validating required fields and generating defaults
    ///
    /// Returns an error if required fields (From, Subject, Newsgroups) are missing.
    /// Auto-generates Date and Message-ID if not provided.
    pub fn build(self) -> Result<Article> {
        // Validate required fields
        let from = self
            .from
            .ok_or_else(|| NntpError::InvalidResponse("From header is required".to_string()))?;
        let subject = self
            .subject
            .ok_or_else(|| NntpError::InvalidResponse("Subject header is required".to_string()))?;

        if self.newsgroups.is_empty() {
            return Err(NntpError::InvalidResponse(
                "At least one newsgroup is required".to_string(),
            ));
        }

        // RFC 5536 Section 3.2.12: Supersedes and Control are mutually exclusive
        if self.supersedes.is_some() && self.control.is_some() {
            return Err(NntpError::InvalidResponse(
                "Article cannot have both Supersedes and Control headers".to_string(),
            ));
        }

        // Auto-generate Date if not provided
        let date = self.date.unwrap_or_else(|| {
            use chrono::Utc;
            Utc::now().format("%a, %d %b %Y %H:%M:%S %z").to_string()
        });

        // Auto-generate Message-ID if not provided
        let message_id = self.message_id.unwrap_or_else(|| {
            use uuid::Uuid;
            let uuid = Uuid::new_v4();
            // Extract domain from From header if present
            let domain = from
                .split('@')
                .nth(1)
                .and_then(|d| d.split('>').next())
                .unwrap_or("localhost");
            format!("<{uuid}@{domain}>")
        });

        // Default Path to "not-for-mail" (will be updated by news server)
        let path = self.path.unwrap_or_else(|| "not-for-mail".to_string());

        let headers = Headers {
            date,
            from,
            message_id,
            newsgroups: self.newsgroups,
            path,
            subject,
            references: self.references,
            reply_to: self.reply_to,
            organization: self.organization,
            followup_to: self.followup_to,
            expires: self.expires,
            control: self.control,
            distribution: self.distribution,
            keywords: self.keywords,
            summary: self.summary,
            supersedes: self.supersedes,
            approved: self.approved,
            lines: None, // Will be calculated by server
            user_agent: self.user_agent,
            xref: None, // Will be added by server
            extra: self.extra,
        };

        Ok(Article {
            headers,
            body: self.body,
            raw: None,
        })
    }

    /// Build and serialize the article for posting
    ///
    /// Returns the article as a string with CRLF line endings and dot-stuffing
    /// applied, ready to be sent to an NNTP server via POST or IHAVE.
    pub fn build_for_posting(self) -> Result<String> {
        let article = self.build()?;
        article.serialize_for_posting()
    }
}

impl Article {
    /// Serialize the article for posting with CRLF line endings and dot-stuffing
    ///
    /// Converts the article to the wire format required by NNTP POST/IHAVE:
    /// - CRLF line endings (\r\n)
    /// - Dot-stuffing: lines starting with '.' are prefixed with '.'
    /// - Headers appear first, followed by blank line, then body
    ///
    /// # Examples
    ///
    /// ```
    /// use nntp_rs::article::ArticleBuilder;
    ///
    /// let article = ArticleBuilder::new()
    ///     .from("user@example.com")
    ///     .subject("Test")
    ///     .newsgroups(vec!["test.group"])
    ///     .body("Hello world")
    ///     .build()
    ///     .unwrap();
    ///
    /// let wire_format = article.serialize_for_posting().unwrap();
    /// assert!(wire_format.contains("\r\n"));
    /// ```
    pub fn serialize_for_posting(&self) -> Result<String> {
        let mut result = String::new();

        // Write required headers
        result.push_str(&format!("Date: {}\r\n", self.headers.date));
        result.push_str(&format!("From: {}\r\n", self.headers.from));
        result.push_str(&format!("Message-ID: {}\r\n", self.headers.message_id));
        result.push_str(&format!(
            "Newsgroups: {}\r\n",
            self.headers.newsgroups.join(",")
        ));
        result.push_str(&format!("Path: {}\r\n", self.headers.path));
        result.push_str(&format!("Subject: {}\r\n", self.headers.subject));

        // Write optional headers
        if let Some(ref references) = self.headers.references {
            result.push_str(&format!("References: {}\r\n", references.join(" ")));
        }
        if let Some(ref reply_to) = self.headers.reply_to {
            result.push_str(&format!("Reply-To: {}\r\n", reply_to));
        }
        if let Some(ref organization) = self.headers.organization {
            result.push_str(&format!("Organization: {}\r\n", organization));
        }
        if let Some(ref followup_to) = self.headers.followup_to {
            result.push_str(&format!("Followup-To: {}\r\n", followup_to.join(",")));
        }
        if let Some(ref expires) = self.headers.expires {
            result.push_str(&format!("Expires: {}\r\n", expires));
        }
        if let Some(ref control) = self.headers.control {
            result.push_str(&format!("Control: {}\r\n", control));
        }
        if let Some(ref distribution) = self.headers.distribution {
            result.push_str(&format!("Distribution: {}\r\n", distribution));
        }
        if let Some(ref keywords) = self.headers.keywords {
            result.push_str(&format!("Keywords: {}\r\n", keywords));
        }
        if let Some(ref summary) = self.headers.summary {
            result.push_str(&format!("Summary: {}\r\n", summary));
        }
        if let Some(ref supersedes) = self.headers.supersedes {
            result.push_str(&format!("Supersedes: {}\r\n", supersedes));
        }
        if let Some(ref approved) = self.headers.approved {
            result.push_str(&format!("Approved: {}\r\n", approved));
        }
        if let Some(ref user_agent) = self.headers.user_agent {
            result.push_str(&format!("User-Agent: {}\r\n", user_agent));
        }

        // Write extra headers
        for (name, value) in &self.headers.extra {
            result.push_str(&format!("{}: {}\r\n", name, value));
        }

        // Blank line separates headers from body
        result.push_str("\r\n");

        // Write body with dot-stuffing
        for line in self.body.lines() {
            if line.starts_with('.') {
                result.push('.');
            }
            result.push_str(line);
            result.push_str("\r\n");
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_article_new() {
        let headers = Headers::new(
            "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
            "user@example.com".to_string(),
            "<abc123@example.com>".to_string(),
            vec!["comp.lang.rust".to_string()],
            "news.example.com!not-for-mail".to_string(),
            "Test Subject".to_string(),
        );

        let article = Article::new(headers, "Test body".to_string());

        assert_eq!(article.body, "Test body");
        assert_eq!(article.headers.subject, "Test Subject");
        assert_eq!(article.headers.from, "user@example.com");
        assert!(article.raw().is_none());
    }

    #[test]
    fn test_headers_new() {
        let headers = Headers::new(
            "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
            "user@example.com".to_string(),
            "<abc123@example.com>".to_string(),
            vec!["comp.lang.rust".to_string(), "comp.lang.c".to_string()],
            "news.example.com!not-for-mail".to_string(),
            "Test Subject".to_string(),
        );

        assert_eq!(headers.date, "Mon, 20 Jan 2025 12:00:00 +0000");
        assert_eq!(headers.from, "user@example.com");
        assert_eq!(headers.message_id, "<abc123@example.com>");
        assert_eq!(headers.newsgroups.len(), 2);
        assert_eq!(headers.newsgroups[0], "comp.lang.rust");
        assert_eq!(headers.newsgroups[1], "comp.lang.c");
        assert_eq!(headers.path, "news.example.com!not-for-mail");
        assert_eq!(headers.subject, "Test Subject");
    }

    #[test]
    fn test_headers_optional_fields() {
        let mut headers = Headers::new(
            "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
            "user@example.com".to_string(),
            "<abc123@example.com>".to_string(),
            vec!["comp.lang.rust".to_string()],
            "news.example.com!not-for-mail".to_string(),
            "Test Subject".to_string(),
        );

        // Test optional fields start as None
        assert!(headers.references.is_none());
        assert!(headers.reply_to.is_none());
        assert!(headers.organization.is_none());

        // Set some optional fields
        headers.references = Some(vec!["<prev@example.com>".to_string()]);
        headers.organization = Some("Test Org".to_string());
        headers.lines = Some(42);

        assert_eq!(headers.references.as_ref().unwrap().len(), 1);
        assert_eq!(headers.organization.as_ref().unwrap(), "Test Org");
        assert_eq!(headers.lines.unwrap(), 42);
    }

    #[test]
    fn test_article_with_optional_headers() {
        let mut headers = Headers::new(
            "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
            "user@example.com".to_string(),
            "<abc123@example.com>".to_string(),
            vec!["comp.lang.rust".to_string()],
            "news.example.com!not-for-mail".to_string(),
            "Re: Test".to_string(),
        );

        headers.references = Some(vec![
            "<msg1@example.com>".to_string(),
            "<msg2@example.com>".to_string(),
        ]);
        headers.user_agent = Some("test-client/1.0".to_string());

        let article = Article::new(headers, "Reply body".to_string());

        assert_eq!(article.headers.references.as_ref().unwrap().len(), 2);
        assert_eq!(
            article.headers.user_agent.as_ref().unwrap(),
            "test-client/1.0"
        );
    }

    #[test]
    fn test_extra_headers() {
        let mut headers = Headers::new(
            "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
            "user@example.com".to_string(),
            "<abc123@example.com>".to_string(),
            vec!["comp.lang.rust".to_string()],
            "news.example.com!not-for-mail".to_string(),
            "Test".to_string(),
        );

        headers
            .extra
            .insert("X-Custom-Header".to_string(), "custom value".to_string());
        headers
            .extra
            .insert("X-Another".to_string(), "another value".to_string());

        assert_eq!(headers.extra.len(), 2);
        assert_eq!(
            headers.extra.get("X-Custom-Header").unwrap(),
            "custom value"
        );
        assert_eq!(headers.extra.get("X-Another").unwrap(), "another value");
    }

    #[test]
    fn test_article_clone() {
        let headers = Headers::new(
            "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
            "user@example.com".to_string(),
            "<abc123@example.com>".to_string(),
            vec!["comp.lang.rust".to_string()],
            "news.example.com!not-for-mail".to_string(),
            "Test".to_string(),
        );

        let article1 = Article::new(headers, "Body".to_string());
        let article2 = article1.clone();

        assert_eq!(article1.body, article2.body);
        assert_eq!(article1.headers.subject, article2.headers.subject);
        assert_eq!(article1.headers.from, article2.headers.from);
    }

    #[test]
    fn test_multiple_newsgroups() {
        let headers = Headers::new(
            "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
            "user@example.com".to_string(),
            "<abc123@example.com>".to_string(),
            vec![
                "comp.lang.rust".to_string(),
                "comp.lang.c".to_string(),
                "comp.lang.python".to_string(),
            ],
            "news.example.com!not-for-mail".to_string(),
            "Cross-posted Article".to_string(),
        );

        assert_eq!(headers.newsgroups.len(), 3);
        assert!(headers.newsgroups.contains(&"comp.lang.rust".to_string()));
        assert!(headers.newsgroups.contains(&"comp.lang.c".to_string()));
        assert!(headers.newsgroups.contains(&"comp.lang.python".to_string()));
    }

    #[test]
    fn test_references_threading() {
        let mut headers = Headers::new(
            "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
            "user@example.com".to_string(),
            "<reply@example.com>".to_string(),
            vec!["comp.lang.rust".to_string()],
            "news.example.com!not-for-mail".to_string(),
            "Re: Test".to_string(),
        );

        // Set up threading references
        headers.references = Some(vec![
            "<original@example.com>".to_string(),
            "<reply1@example.com>".to_string(),
            "<reply2@example.com>".to_string(),
        ]);

        let refs = headers.references.as_ref().unwrap();
        assert_eq!(refs.len(), 3);
        assert_eq!(refs[0], "<original@example.com>");
        assert_eq!(refs[2], "<reply2@example.com>");
    }

    #[test]
    fn test_control_message() {
        let mut headers = Headers::new(
            "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
            "user@example.com".to_string(),
            "<cancel@example.com>".to_string(),
            vec!["comp.lang.rust".to_string()],
            "news.example.com!not-for-mail".to_string(),
            "cancel <original@example.com>".to_string(),
        );

        headers.control = Some("cancel <original@example.com>".to_string());

        assert!(headers.control.is_some());
        assert_eq!(
            headers.control.as_ref().unwrap(),
            "cancel <original@example.com>"
        );
    }

    #[test]
    fn test_moderated_group_approved() {
        let mut headers = Headers::new(
            "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
            "user@example.com".to_string(),
            "<abc@example.com>".to_string(),
            vec!["comp.lang.rust.moderated".to_string()],
            "news.example.com!not-for-mail".to_string(),
            "Approved Post".to_string(),
        );

        headers.approved = Some("moderator@example.com".to_string());

        assert!(headers.approved.is_some());
        assert_eq!(headers.approved.as_ref().unwrap(), "moderator@example.com");
    }

    #[test]
    fn test_headers_validate_valid() {
        let headers = Headers::new(
            "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
            "user@example.com".to_string(),
            "<abc123@example.com>".to_string(),
            vec!["comp.lang.rust".to_string()],
            "news.example.com!not-for-mail".to_string(),
            "Test Article".to_string(),
        );

        let config = crate::validation::ValidationConfig::default();
        assert!(headers.validate(&config).is_ok());
    }

    #[test]
    fn test_headers_validate_empty_date() {
        let headers = Headers::new(
            "".to_string(),
            "user@example.com".to_string(),
            "<abc123@example.com>".to_string(),
            vec!["comp.lang.rust".to_string()],
            "news.example.com!not-for-mail".to_string(),
            "Test".to_string(),
        );

        let config = crate::validation::ValidationConfig::default();
        assert!(headers.validate(&config).is_err());
    }

    #[test]
    fn test_headers_validate_empty_from() {
        let headers = Headers::new(
            "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
            "".to_string(),
            "<abc123@example.com>".to_string(),
            vec!["comp.lang.rust".to_string()],
            "news.example.com!not-for-mail".to_string(),
            "Test".to_string(),
        );

        let config = crate::validation::ValidationConfig::default();
        assert!(headers.validate(&config).is_err());
    }

    #[test]
    fn test_headers_validate_invalid_message_id() {
        let headers = Headers::new(
            "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
            "user@example.com".to_string(),
            "abc123@example.com".to_string(), // Missing angle brackets
            vec!["comp.lang.rust".to_string()],
            "news.example.com!not-for-mail".to_string(),
            "Test".to_string(),
        );

        let config = crate::validation::ValidationConfig::default();
        assert!(headers.validate(&config).is_err());
    }

    #[test]
    fn test_headers_validate_invalid_newsgroup() {
        let headers = Headers::new(
            "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
            "user@example.com".to_string(),
            "<abc123@example.com>".to_string(),
            vec!["comp..rust".to_string()], // Empty component
            "news.example.com!not-for-mail".to_string(),
            "Test".to_string(),
        );

        let config = crate::validation::ValidationConfig::default();
        assert!(headers.validate(&config).is_err());
    }

    #[test]
    fn test_headers_validate_invalid_date_format() {
        let headers = Headers::new(
            "not a date".to_string(),
            "user@example.com".to_string(),
            "<abc123@example.com>".to_string(),
            vec!["comp.lang.rust".to_string()],
            "news.example.com!not-for-mail".to_string(),
            "Test".to_string(),
        );

        let config = crate::validation::ValidationConfig::default();
        assert!(headers.validate(&config).is_err());
    }

    #[test]
    fn test_headers_validate_with_references() {
        let mut headers = Headers::new(
            "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
            "user@example.com".to_string(),
            "<abc123@example.com>".to_string(),
            vec!["comp.lang.rust".to_string()],
            "news.example.com!not-for-mail".to_string(),
            "Re: Test".to_string(),
        );

        headers.references = Some(vec![
            "<msg1@example.com>".to_string(),
            "<msg2@example.com>".to_string(),
        ]);

        let config = crate::validation::ValidationConfig::default();
        assert!(headers.validate(&config).is_ok());
    }

    #[test]
    fn test_headers_validate_invalid_reference() {
        let mut headers = Headers::new(
            "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
            "user@example.com".to_string(),
            "<abc123@example.com>".to_string(),
            vec!["comp.lang.rust".to_string()],
            "news.example.com!not-for-mail".to_string(),
            "Test".to_string(),
        );

        headers.references = Some(vec!["invalid-reference".to_string()]); // Missing brackets

        let config = crate::validation::ValidationConfig::default();
        assert!(headers.validate(&config).is_err());
    }

    #[test]
    fn test_headers_validate_supersedes_and_control_mutually_exclusive() {
        let mut headers = Headers::new(
            "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
            "user@example.com".to_string(),
            "<abc123@example.com>".to_string(),
            vec!["comp.lang.rust".to_string()],
            "news.example.com!not-for-mail".to_string(),
            "Test".to_string(),
        );

        headers.supersedes = Some("<old@example.com>".to_string());
        headers.control = Some("cancel <old@example.com>".to_string());

        let config = crate::validation::ValidationConfig::default();
        assert!(headers.validate(&config).is_err());
    }

    #[test]
    fn test_headers_validate_with_followup_to() {
        let mut headers = Headers::new(
            "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
            "user@example.com".to_string(),
            "<abc123@example.com>".to_string(),
            vec!["comp.lang.rust".to_string()],
            "news.example.com!not-for-mail".to_string(),
            "Test".to_string(),
        );

        headers.followup_to = Some(vec!["alt.test".to_string()]);

        let config = crate::validation::ValidationConfig::default();
        assert!(headers.validate(&config).is_ok());
    }

    #[test]
    fn test_headers_validate_followup_to_poster() {
        let mut headers = Headers::new(
            "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
            "user@example.com".to_string(),
            "<abc123@example.com>".to_string(),
            vec!["comp.lang.rust".to_string()],
            "news.example.com!not-for-mail".to_string(),
            "Test".to_string(),
        );

        headers.followup_to = Some(vec!["poster".to_string()]);

        let config = crate::validation::ValidationConfig::default();
        assert!(headers.validate(&config).is_ok());
    }

    #[test]
    fn test_headers_validate_invalid_followup_to() {
        let mut headers = Headers::new(
            "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
            "user@example.com".to_string(),
            "<abc123@example.com>".to_string(),
            vec!["comp.lang.rust".to_string()],
            "news.example.com!not-for-mail".to_string(),
            "Test".to_string(),
        );

        headers.followup_to = Some(vec!["invalid..group".to_string()]);

        let config = crate::validation::ValidationConfig::default();
        assert!(headers.validate(&config).is_err());
    }
}

//! Article type definitions
//!
//! This module contains the core data structures for representing Usenet articles.

use std::collections::HashMap;
use std::fmt::Write;

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
    /// Format: List of message-IDs (e.g., `["<msg1@example.com>", "<msg2@example.com>"]`)
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
    /// Format: "command arguments" (e.g., `"cancel <msg-id>"`)
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
        // Pre-allocate capacity: typical headers are ~1KB, body varies
        let mut result = String::with_capacity(1024 + self.body.len());

        // Write required headers
        // SAFETY: write! to String is infallible (OOM aside)
        #[expect(clippy::unwrap_used)]
        {
            write!(result, "Date: {}\r\n", self.headers.date).unwrap();
            write!(result, "From: {}\r\n", self.headers.from).unwrap();
            write!(result, "Message-ID: {}\r\n", self.headers.message_id).unwrap();
            write!(
                result,
                "Newsgroups: {}\r\n",
                self.headers.newsgroups.join(",")
            )
            .unwrap();
            write!(result, "Path: {}\r\n", self.headers.path).unwrap();
            write!(result, "Subject: {}\r\n", self.headers.subject).unwrap();

            // Write optional headers
            if let Some(ref references) = self.headers.references {
                write!(result, "References: {}\r\n", references.join(" ")).unwrap();
            }
            if let Some(ref reply_to) = self.headers.reply_to {
                write!(result, "Reply-To: {}\r\n", reply_to).unwrap();
            }
            if let Some(ref organization) = self.headers.organization {
                write!(result, "Organization: {}\r\n", organization).unwrap();
            }
            if let Some(ref followup_to) = self.headers.followup_to {
                write!(result, "Followup-To: {}\r\n", followup_to.join(",")).unwrap();
            }
            if let Some(ref expires) = self.headers.expires {
                write!(result, "Expires: {}\r\n", expires).unwrap();
            }
            if let Some(ref control) = self.headers.control {
                write!(result, "Control: {}\r\n", control).unwrap();
            }
            if let Some(ref distribution) = self.headers.distribution {
                write!(result, "Distribution: {}\r\n", distribution).unwrap();
            }
            if let Some(ref keywords) = self.headers.keywords {
                write!(result, "Keywords: {}\r\n", keywords).unwrap();
            }
            if let Some(ref summary) = self.headers.summary {
                write!(result, "Summary: {}\r\n", summary).unwrap();
            }
            if let Some(ref supersedes) = self.headers.supersedes {
                write!(result, "Supersedes: {}\r\n", supersedes).unwrap();
            }
            if let Some(ref approved) = self.headers.approved {
                write!(result, "Approved: {}\r\n", approved).unwrap();
            }
            if let Some(ref user_agent) = self.headers.user_agent {
                write!(result, "User-Agent: {}\r\n", user_agent).unwrap();
            }

            // Write extra headers
            for (name, value) in &self.headers.extra {
                write!(result, "{}: {}\r\n", name, value).unwrap();
            }
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

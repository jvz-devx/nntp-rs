//! Article builder for constructing valid articles
//!
//! This module provides the ArticleBuilder for creating RFC 5536 compliant articles.

use std::collections::HashMap;

use super::types::{Article, Headers};
use crate::{NntpError, Result};

#[must_use]
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

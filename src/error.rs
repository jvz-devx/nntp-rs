//! NNTP error types

use thiserror::Error;

/// NNTP protocol and connection errors
#[derive(Error, Debug)]
pub enum NntpError {
    /// IO error during network operations
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// TLS error during secure connection
    #[error("TLS error: {0}")]
    Tls(String),

    /// Connection timeout
    #[error("Connection timeout")]
    Timeout,

    /// Invalid response from server
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// NNTP protocol error with response code
    #[error("NNTP error {code}: {message}")]
    Protocol {
        /// NNTP response code (e.g., 411, 430, 502)
        code: u16,
        /// Error message from server
        message: String,
    },

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    /// No such newsgroup
    #[error("No such newsgroup: {0}")]
    NoSuchGroup(String),

    /// No such article
    #[error("No such article: {0}")]
    NoSuchArticle(String),

    /// No newsgroup selected
    #[error("No newsgroup selected")]
    NoGroupSelected,

    /// Invalid article number
    #[error("Invalid article number")]
    InvalidArticleNumber,

    /// Posting not permitted
    #[error("Posting not permitted")]
    PostingNotPermitted,

    /// Posting failed
    #[error("Posting failed: {0}")]
    PostingFailed(String),

    /// Article not wanted (IHAVE rejected)
    #[error("Article not wanted")]
    ArticleNotWanted,

    /// Transfer not possible; try again later
    #[error("Transfer not possible: {0}")]
    TransferNotPossible(String),

    /// Transfer rejected; do not retry
    #[error("Transfer rejected: {0}")]
    TransferRejected(String),

    /// Encryption required for authentication
    #[error("Encryption required: {0}")]
    EncryptionRequired(String),

    /// Connection closed unexpectedly
    #[error("Connection closed")]
    ConnectionClosed,

    /// UTF-8 decoding error
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    /// Generic error
    #[error("{0}")]
    Other(String),
}

/// Result type alias using NntpError
pub type Result<T> = std::result::Result<T, NntpError>;

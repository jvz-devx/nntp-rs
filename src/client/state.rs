//! Connection state types for NNTP client

/// NNTP connection state tracking authentication progress
///
/// Tracks the authentication state of an NNTP connection according to RFC 4643.
/// Commands may be restricted based on the current state.
pub(super) enum ConnectionState {
    /// Connected and ready for commands (not authenticated)
    Ready,
    /// Authentication in progress (AUTHINFO USER sent, waiting for PASS or SASL exchange)
    InProgress,
    /// Successfully authenticated
    Authenticated,
    /// Connection closed
    Closed,
}

/// Compression mode for NNTP connection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CompressionMode {
    /// No compression
    None,
    /// Headers-only compression (XFEATURE COMPRESS GZIP)
    /// Only multiline responses (XOVER, HEAD, ARTICLE) are gzip-compressed
    HeadersOnly,
    /// Full session compression (RFC 8054 COMPRESS DEFLATE)
    /// All data after negotiation is deflate-compressed bidirectionally
    FullSession,
}

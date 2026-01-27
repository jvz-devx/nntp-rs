//! RFC 5536 Article Format
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc5536
//!
//! Provides structures for representing and working with Usenet articles
//! conforming to RFC 5536 (Netnews Article Format).
//!
//! This module is organized into:
//! - `types`: Core article data structures (Article, Headers, ControlMessage)
//! - `parsing`: Article and header parsing functions
//! - `builder`: ArticleBuilder for constructing valid articles

// Module declarations - will be populated in subsequent refactoring steps
mod builder;
mod parsing;
mod types;

// Re-export public API
pub use self::builder::ArticleBuilder;
pub use self::parsing::{parse_article, parse_headers};
pub use self::types::{Article, ControlMessage, Headers};

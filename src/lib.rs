#![doc = include_str!("../README.md")]

/// RFC 5536 Article Format
pub mod article;
/// Article assembler for binary downloads
pub mod assembler;
/// Header caching for NNTP client
pub mod cache;
mod capabilities;
mod client;
/// NNTP command builders and response parsers
pub mod commands;
mod config;
/// RFC 2047 Encoded Words support for international headers
pub mod encoded_words;
mod error;
/// NZB file format parser
pub mod nzb;
/// PAR2 file format parser for error correction
pub mod par2;
mod pool;
/// Rate limiting for bandwidth and connection management
pub mod ratelimit;
mod response;
/// SASL authentication framework (RFC 4643)
pub mod sasl;
/// Segment fetcher for Usenet binary downloads
pub mod segments;
/// Multi-server support with automatic failover
pub mod servers;
/// RFC 5536 Article validation utilities
pub mod validation;
/// yEnc binary encoding/decoding for Usenet
pub mod yenc;

pub use article::{parse_article, parse_headers, Article, ArticleBuilder, ControlMessage, Headers};
pub use assembler::{ArticleAssembler, PartInfo, PartStatus};
pub use cache::{HeaderCache, LruHeaderCache};
pub use capabilities::Capabilities;
pub use client::NntpClient;
pub use commands::{DistributionInfo, HdrEntry, ModeratorInfo, XoverEntry};
pub use config::ServerConfig;
pub use error::{NntpError, Result};
pub use nzb::{parse_nzb, Nzb, NzbFile, NzbSegment};
pub use par2::{
    CreatorPacket, FileDescriptionPacket, FileStatus, FileVerification, IfscPacket, MainPacket,
    PacketHeader, PacketType, Par2File, Par2Set, RecoverySlicePacket,
};
pub use pool::{NntpPool, RetryConfig};
pub use ratelimit::{BandwidthLimiter, ConnectionLimiter, ConnectionPermit};
pub use response::{codes, NntpBinaryResponse, NntpResponse};
pub use sasl::{decode_sasl_data, encode_sasl_data, SaslMechanism, SaslPlain};
pub use segments::{FetchConfig, FetchProgress, SegmentFetchResult, SegmentFetcher, SegmentStatus};
pub use servers::{FailoverStrategy, GroupStats, ServerGroup, ServerStats};
pub use validation::{
    parse_date, validate_date, validate_message_id, validate_newsgroup_name, ValidationConfig,
};
pub use yenc::{
    decode as yenc_decode, encode as yenc_encode, YencDecoded, YencEnd, YencHeader,
    YencMultipartAssembler, YencPart,
};

//! yEnc binary encoding/decoding for Usenet
//!
//! yEnc is a binary-to-text encoding scheme designed specifically for Usenet.
//! It has only 1-2% overhead compared to 33-40% for Base64.
//!
//! Reference: http://www.yenc.org/yenc-draft.1.3.txt

pub mod assembler;
pub mod decode;
pub mod encode;
pub mod params;
pub mod types;

// Re-export public types and functions for backward compatibility
pub use assembler::YencMultipartAssembler;
pub use decode::decode;
pub use encode::encode;
pub use types::{YencDecoded, YencEnd, YencHeader, YencPart};

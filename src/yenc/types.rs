/// yEnc header from =ybegin line
#[derive(Debug, Clone, PartialEq)]
pub struct YencHeader {
    /// Line length (typically 128, max 997)
    pub line: usize,
    /// Total file size in bytes
    pub size: u64,
    /// Original filename
    pub name: String,
    /// Part number (for multi-part files)
    pub part: Option<u32>,
    /// Total number of parts (for multi-part files)
    pub total: Option<u32>,
}

/// yEnc part header from =ypart line (for multi-part files)
#[derive(Debug, Clone, PartialEq)]
pub struct YencPart {
    /// Byte offset where this part begins in the original file
    pub begin: u64,
    /// Byte offset where this part ends in the original file
    pub end: u64,
}

/// yEnc trailer from =yend line
#[derive(Debug, Clone, PartialEq)]
pub struct YencEnd {
    /// Size of decoded data in bytes
    pub size: u64,
    /// CRC32 of entire decoded file (for single-part) or this part (for multi-part)
    pub crc32: Option<u32>,
    /// CRC32 of this part only (for multi-part files)
    pub pcrc32: Option<u32>,
}

/// Complete yEnc decoded result
#[derive(Debug, Clone)]
pub struct YencDecoded {
    /// Parsed header information
    pub header: YencHeader,
    /// Part information (for multi-part files)
    pub part: Option<YencPart>,
    /// Trailer information
    pub trailer: YencEnd,
    /// Decoded binary data
    pub data: Vec<u8>,
    /// Calculated CRC32 of decoded data
    pub calculated_crc32: u32,
}

impl YencDecoded {
    /// Verify CRC32 matches expected value
    pub fn verify_crc32(&self) -> bool {
        // For multi-part files, check pcrc32 (part CRC)
        if let Some(expected) = self.trailer.pcrc32 {
            return self.calculated_crc32 == expected;
        }
        // For single-part files, check crc32
        if let Some(expected) = self.trailer.crc32 {
            return self.calculated_crc32 == expected;
        }
        // No CRC to verify
        false
    }

    /// Check if this is a multi-part file
    pub fn is_multipart(&self) -> bool {
        self.header.part.is_some() && self.header.total.is_some()
    }
}

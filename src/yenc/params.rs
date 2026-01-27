use crate::{NntpError, Result};
use std::collections::HashMap;

use super::types::{YencEnd, YencHeader, YencPart};

/// Parse yEnc =ybegin header line
///
/// Format: =ybegin line=128 size=123456 name=file.bin [part=1 total=5]
pub(crate) fn parse_ybegin(line: &str) -> Result<YencHeader> {
    if !line.starts_with("=ybegin ") {
        return Err(NntpError::InvalidResponse(format!(
            "Invalid yEnc header: {}",
            line
        )));
    }

    let params = parse_yenc_params(&line[8..])?;

    let line_len = params
        .get("line")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| NntpError::InvalidResponse("Missing 'line' parameter".to_string()))?;

    let size = params
        .get("size")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| NntpError::InvalidResponse("Missing 'size' parameter".to_string()))?;

    let name = params
        .get("name")
        .ok_or_else(|| NntpError::InvalidResponse("Missing 'name' parameter".to_string()))?
        .to_string();

    let part = params.get("part").and_then(|s| s.parse().ok());
    let total = params.get("total").and_then(|s| s.parse().ok());

    Ok(YencHeader {
        line: line_len,
        size,
        name,
        part,
        total,
    })
}

/// Parse yEnc =ypart line
///
/// Format: =ypart begin=1 end=123456
pub(crate) fn parse_ypart(line: &str) -> Result<YencPart> {
    if !line.starts_with("=ypart ") {
        return Err(NntpError::InvalidResponse(format!(
            "Invalid yEnc part header: {}",
            line
        )));
    }

    let params = parse_yenc_params(&line[7..])?;

    let begin = params
        .get("begin")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| NntpError::InvalidResponse("Missing 'begin' parameter".to_string()))?;

    let end = params
        .get("end")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| NntpError::InvalidResponse("Missing 'end' parameter".to_string()))?;

    Ok(YencPart { begin, end })
}

/// Parse yEnc =yend line
///
/// Format: =yend size=123456 [crc32=12345678] [pcrc32=87654321]
pub(crate) fn parse_yend(line: &str) -> Result<YencEnd> {
    if !line.starts_with("=yend ") {
        return Err(NntpError::InvalidResponse(format!(
            "Invalid yEnc trailer: {}",
            line
        )));
    }

    let params = parse_yenc_params(&line[6..])?;

    let size = params
        .get("size")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| NntpError::InvalidResponse("Missing 'size' parameter".to_string()))?;

    let crc32 = params.get("crc32").and_then(|s| {
        // CRC32 is in hex format
        u32::from_str_radix(s, 16).ok()
    });

    let pcrc32 = params
        .get("pcrc32")
        .and_then(|s| u32::from_str_radix(s, 16).ok());

    Ok(YencEnd {
        size,
        crc32,
        pcrc32,
    })
}

/// Parse yEnc key=value parameters
fn parse_yenc_params(params: &str) -> Result<HashMap<String, String>> {
    let mut result = HashMap::new();
    let mut chars = params.chars().peekable();

    while chars.peek().is_some() {
        // Skip whitespace
        while chars.peek() == Some(&' ') {
            chars.next();
        }

        if chars.peek().is_none() {
            break;
        }

        // Parse key
        let mut key = String::new();
        while let Some(&ch) = chars.peek() {
            if ch == '=' {
                chars.next(); // consume '='
                break;
            }
            key.push(ch);
            chars.next();
        }

        if key.is_empty() {
            break;
        }

        // Parse value (everything until next space or end)
        let mut value = String::new();
        while let Some(&ch) = chars.peek() {
            if ch == ' ' {
                break;
            }
            value.push(ch);
            chars.next();
        }

        result.insert(key, value);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ybegin_simple() {
        let line = "=ybegin line=128 size=123456 name=test.bin";
        let header = parse_ybegin(line).unwrap();

        assert_eq!(header.line, 128);
        assert_eq!(header.size, 123456);
        assert_eq!(header.name, "test.bin");
        assert_eq!(header.part, None);
        assert_eq!(header.total, None);
    }

    #[test]
    fn test_parse_ybegin_multipart() {
        let line = "=ybegin part=1 total=5 line=128 size=123456 name=file.rar";
        let header = parse_ybegin(line).unwrap();

        assert_eq!(header.line, 128);
        assert_eq!(header.size, 123456);
        assert_eq!(header.name, "file.rar");
        assert_eq!(header.part, Some(1));
        assert_eq!(header.total, Some(5));
    }

    #[test]
    fn test_parse_ypart() {
        let line = "=ypart begin=1 end=384000";
        let part = parse_ypart(line).unwrap();

        assert_eq!(part.begin, 1);
        assert_eq!(part.end, 384000);
    }

    #[test]
    fn test_parse_yend() {
        let line = "=yend size=384000 pcrc32=12345678";
        let end = parse_yend(line).unwrap();

        assert_eq!(end.size, 384000);
        assert_eq!(end.pcrc32, Some(0x12345678));
    }

    #[test]
    fn test_parse_yenc_params() {
        let params = parse_yenc_params("line=128 size=123456 name=test.bin").unwrap();
        assert_eq!(params.get("line"), Some(&"128".to_string()));
        assert_eq!(params.get("size"), Some(&"123456".to_string()));
        assert_eq!(params.get("name"), Some(&"test.bin".to_string()));
    }
}

//! NZB file format parser and generator
//!
//! NZB is an XML-based file format used to describe Usenet binary posts.
//! It contains metadata and segment references for efficient binary downloads.
//!
//! Reference: https://sabnzbd.org/wiki/extra/nzb-spec

use crate::{NntpError, Result};
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};
use std::collections::{HashMap, HashSet};
use std::io::Cursor;

/// NZB file containing metadata and file references
#[derive(Debug, Clone, PartialEq)]
pub struct Nzb {
    /// Metadata from <head> section (e.g., title, password, tag, category)
    pub meta: HashMap<String, String>,
    /// List of files described in this NZB
    pub files: Vec<NzbFile>,
}

/// A single file entry in an NZB
#[derive(Debug, Clone, PartialEq)]
pub struct NzbFile {
    /// Poster name/email
    pub poster: String,
    /// Unix timestamp of posting
    pub date: i64,
    /// Subject line
    pub subject: String,
    /// Newsgroups where this file was posted
    pub groups: Vec<String>,
    /// Segments (parts) of this file
    pub segments: Vec<NzbSegment>,
}

/// A segment (part) of a file
#[derive(Debug, Clone, PartialEq)]
pub struct NzbSegment {
    /// Size of this segment in bytes
    pub bytes: u64,
    /// Segment number (1-based)
    pub number: u32,
    /// Message-ID for retrieving this segment
    pub message_id: String,
}

impl NzbFile {
    /// Calculate total size of all segments
    pub fn total_bytes(&self) -> u64 {
        self.segments.iter().map(|s| s.bytes).sum()
    }

    /// Validate segment numbering
    ///
    /// Returns an error if:
    /// - Segments are not sequential starting from 1
    /// - There are duplicate segment numbers
    /// - Segments are empty
    pub fn validate_segments(&self) -> Result<()> {
        if self.segments.is_empty() {
            return Err(NntpError::InvalidResponse(
                "File has no segments".to_string(),
            ));
        }

        let mut seen = HashSet::new();
        let mut max_number = 0u32;

        for seg in &self.segments {
            if seg.number < 1 {
                return Err(NntpError::InvalidResponse(format!(
                    "Invalid segment number: {}",
                    seg.number
                )));
            }

            if !seen.insert(seg.number) {
                return Err(NntpError::InvalidResponse(format!(
                    "Duplicate segment number: {}",
                    seg.number
                )));
            }

            max_number = max_number.max(seg.number);
        }

        // Check all numbers from 1 to max_number are present
        for i in 1..=max_number {
            if !seen.contains(&i) {
                return Err(NntpError::InvalidResponse(format!(
                    "Missing segment number: {}",
                    i
                )));
            }
        }

        Ok(())
    }

    /// Get missing segment numbers (if any)
    pub fn missing_segments(&self) -> Vec<u32> {
        if self.segments.is_empty() {
            return vec![];
        }

        let mut seen = HashSet::new();
        let mut max_number = 0u32;

        for seg in &self.segments {
            seen.insert(seg.number);
            max_number = max_number.max(seg.number);
        }

        let mut missing = Vec::new();

        for i in 1..=max_number {
            if !seen.contains(&i) {
                missing.push(i);
            }
        }

        missing
    }
}

impl Nzb {
    /// Calculate total size of all files
    pub fn total_bytes(&self) -> u64 {
        self.files.iter().map(|f| f.total_bytes()).sum()
    }

    /// Validate all files in the NZB
    pub fn validate(&self) -> Result<()> {
        if self.files.is_empty() {
            return Err(NntpError::InvalidResponse("NZB has no files".to_string()));
        }

        for (i, file) in self.files.iter().enumerate() {
            file.validate_segments()
                .map_err(|e| NntpError::InvalidResponse(format!("File {}: {}", i, e)))?;
        }

        Ok(())
    }

    /// Generate XML string from NZB structure
    ///
    /// Creates a properly formatted NZB XML file with all metadata and file references.
    ///
    /// # Returns
    /// XML string representation of this NZB
    ///
    /// # Example
    /// ```
    /// use nntp_rs::{Nzb, NzbFile, NzbSegment};
    /// use std::collections::HashMap;
    ///
    /// let nzb = Nzb {
    ///     meta: HashMap::from([("title".to_string(), "Test File".to_string())]),
    ///     files: vec![NzbFile {
    ///         poster: "user@example.com".to_string(),
    ///         date: 1234567890,
    ///         subject: "Test [1/1]".to_string(),
    ///         groups: vec!["alt.binaries.test".to_string()],
    ///         segments: vec![NzbSegment {
    ///             bytes: 768000,
    ///             number: 1,
    ///             message_id: "part1@example.com".to_string(),
    ///         }],
    ///     }],
    /// };
    ///
    /// let xml = nzb.to_xml();
    /// assert!(xml.contains("<nzb"));
    /// assert!(xml.contains("Test File"));
    /// ```
    pub fn to_xml(&self) -> String {
        // Build the XML body first with quick-xml
        let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);

        // Root <nzb> element with namespace
        let mut nzb_elem = BytesStart::new("nzb");
        nzb_elem.push_attribute(("xmlns", "http://www.newzbin.com/DTD/2003/nzb"));
        writer.write_event(Event::Start(nzb_elem)).unwrap();

        // <head> section if we have metadata
        if !self.meta.is_empty() {
            writer
                .write_event(Event::Start(BytesStart::new("head")))
                .unwrap();

            for (key, value) in &self.meta {
                let mut meta_elem = BytesStart::new("meta");
                meta_elem.push_attribute(("type", key.as_str()));
                writer.write_event(Event::Start(meta_elem)).unwrap();

                // BytesText automatically escapes XML entities
                writer
                    .write_event(Event::Text(BytesText::new(value)))
                    .unwrap();

                writer
                    .write_event(Event::End(BytesEnd::new("meta")))
                    .unwrap();
            }

            writer
                .write_event(Event::End(BytesEnd::new("head")))
                .unwrap();
        }

        // <file> elements
        for file in &self.files {
            let mut file_elem = BytesStart::new("file");
            // push_attribute automatically escapes, so don't manually escape
            file_elem.push_attribute(("poster", file.poster.as_str()));
            file_elem.push_attribute(("date", file.date.to_string().as_str()));
            file_elem.push_attribute(("subject", file.subject.as_str()));
            writer.write_event(Event::Start(file_elem)).unwrap();

            // <groups>
            writer
                .write_event(Event::Start(BytesStart::new("groups")))
                .unwrap();
            for group in &file.groups {
                writer
                    .write_event(Event::Start(BytesStart::new("group")))
                    .unwrap();
                // BytesText automatically escapes
                writer
                    .write_event(Event::Text(BytesText::new(group)))
                    .unwrap();
                writer
                    .write_event(Event::End(BytesEnd::new("group")))
                    .unwrap();
            }
            writer
                .write_event(Event::End(BytesEnd::new("groups")))
                .unwrap();

            // <segments>
            writer
                .write_event(Event::Start(BytesStart::new("segments")))
                .unwrap();
            for segment in &file.segments {
                let mut seg_elem = BytesStart::new("segment");
                seg_elem.push_attribute(("bytes", segment.bytes.to_string().as_str()));
                seg_elem.push_attribute(("number", segment.number.to_string().as_str()));
                writer.write_event(Event::Start(seg_elem)).unwrap();

                // BytesText automatically escapes
                writer
                    .write_event(Event::Text(BytesText::new(&segment.message_id)))
                    .unwrap();

                writer
                    .write_event(Event::End(BytesEnd::new("segment")))
                    .unwrap();
            }
            writer
                .write_event(Event::End(BytesEnd::new("segments")))
                .unwrap();

            writer
                .write_event(Event::End(BytesEnd::new("file")))
                .unwrap();
        }

        // Close root element
        writer
            .write_event(Event::End(BytesEnd::new("nzb")))
            .unwrap();

        let body = writer.into_inner().into_inner();
        let body_str = String::from_utf8(body).unwrap();

        // Prepend XML declaration and DOCTYPE
        let mut result = String::new();
        result.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        result.push_str("<!DOCTYPE nzb PUBLIC \"-//newzBin//DTD NZB 1.1//EN\" \"http://www.newzbin.com/DTD/nzb/nzb-1.1.dtd\">\n");
        result.push_str(&body_str);

        result
    }
}

/// Parse an NZB file from XML string
///
/// # Arguments
/// * `xml` - NZB file contents as string
///
/// # Returns
/// Parsed `Nzb` structure or error
///
/// # Example
/// ```
/// use nntp_rs::parse_nzb;
///
/// let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
/// <!DOCTYPE nzb PUBLIC "-//newzBin//DTD NZB 1.1//EN" "http://www.newzbin.com/DTD/nzb/nzb-1.1.dtd">
/// <nzb xmlns="http://www.newzbin.com/DTD/2003/nzb">
///   <head>
///     <meta type="title">Example File</meta>
///   </head>
///   <file poster="user@example.com" date="1234567890" subject="Example [1/1]">
///     <groups>
///       <group>alt.binaries.test</group>
///     </groups>
///     <segments>
///       <segment bytes="768000" number="1">part1of1@example.com</segment>
///     </segments>
///   </file>
/// </nzb>"#;
///
/// let nzb = parse_nzb(xml).unwrap();
/// assert_eq!(nzb.files.len(), 1);
/// assert_eq!(nzb.files[0].segments.len(), 1);
/// ```
pub fn parse_nzb(xml: &str) -> Result<Nzb> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut nzb = Nzb {
        meta: HashMap::new(),
        files: Vec::new(),
    };

    let mut in_head = false;
    let mut in_file = false;
    let mut in_groups = false;
    let mut in_segments = false;
    let mut in_meta = false;
    let mut meta_type = String::new();

    let mut current_file: Option<NzbFile> = None;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = e.name();
                match name.as_ref() {
                    b"head" => in_head = true,
                    b"meta" if in_head => {
                        in_meta = true;
                        // Extract type attribute
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"type" {
                                meta_type = String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                    }
                    b"file" => {
                        in_file = true;
                        let mut poster = String::new();
                        let mut date = 0i64;
                        let mut subject = String::new();

                        // Extract attributes
                        for attr in e.attributes().flatten() {
                            let key = attr.key.as_ref();
                            let value = attr.unescape_value().unwrap_or_default().to_string();

                            match key {
                                b"poster" => poster = value,
                                b"date" => date = value.parse().unwrap_or(0),
                                b"subject" => subject = value,
                                _ => {}
                            }
                        }

                        current_file = Some(NzbFile {
                            poster,
                            date,
                            subject,
                            groups: Vec::new(),
                            segments: Vec::new(),
                        });
                    }
                    b"groups" if in_file => in_groups = true,
                    b"segments" if in_file => in_segments = true,
                    b"segment" if in_segments => {
                        // Will handle in Text event
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                // Handle self-closing tags if needed
                let name = e.name();
                if name.as_ref() == b"segment" && in_segments {
                    if let Some(ref mut _file) = current_file {
                        let mut _bytes = 0u64;
                        let mut _number = 0u32;

                        for attr in e.attributes().flatten() {
                            let key = attr.key.as_ref();
                            let value = String::from_utf8_lossy(&attr.value).to_string();

                            match key {
                                b"bytes" => _bytes = value.parse().unwrap_or(0),
                                b"number" => _number = value.parse().unwrap_or(0),
                                _ => {}
                            }
                        }

                        // Empty segment tag, no message-id - skip
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let name = e.name();
                match name.as_ref() {
                    b"head" => in_head = false,
                    b"meta" => {
                        in_meta = false;
                        meta_type.clear();
                    }
                    b"file" => {
                        in_file = false;
                        if let Some(file) = current_file.take() {
                            nzb.files.push(file);
                        }
                    }
                    b"groups" => in_groups = false,
                    b"segments" => in_segments = false,
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape().unwrap_or_default().trim().to_string();

                if in_meta && !meta_type.is_empty() {
                    nzb.meta.insert(meta_type.clone(), text);
                } else if in_groups {
                    if let Some(ref mut file) = current_file {
                        if !text.is_empty() {
                            file.groups.push(text);
                        }
                    }
                } else if in_segments {
                    // This is the message-id inside <segment>
                    // We need to get attributes from the parent Start event
                    // This is handled differently - we'll process on End
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(NntpError::InvalidResponse(format!(
                    "XML parse error: {}",
                    e
                )))
            }
            _ => {}
        }

        buf.clear();
    }

    // Second pass to get segment data properly
    parse_nzb_segments(&mut nzb, xml)?;

    Ok(nzb)
}

/// Helper to parse segments (second pass needed for message-id extraction)
fn parse_nzb_segments(nzb: &mut Nzb, xml: &str) -> Result<()> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut file_index = 0;
    let mut in_segments = false;
    let mut buf = Vec::new();

    let mut segment_bytes = 0u64;
    let mut segment_number = 0u32;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = e.name();
                match name.as_ref() {
                    b"file" => {
                        // Do nothing, we're just tracking
                    }
                    b"segments" => in_segments = true,
                    b"segment" if in_segments => {
                        // Extract attributes
                        segment_bytes = 0;
                        segment_number = 0;

                        for attr in e.attributes().flatten() {
                            let key = attr.key.as_ref();
                            let value = String::from_utf8_lossy(&attr.value).to_string();

                            match key {
                                b"bytes" => segment_bytes = value.parse().unwrap_or(0),
                                b"number" => segment_number = value.parse().unwrap_or(0),
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) if in_segments => {
                let message_id = e.unescape().unwrap_or_default().trim().to_string();

                if !message_id.is_empty() && segment_number > 0 && file_index < nzb.files.len() {
                    nzb.files[file_index].segments.push(NzbSegment {
                        bytes: segment_bytes,
                        number: segment_number,
                        message_id,
                    });
                }
            }
            Ok(Event::End(ref e)) => {
                let name = e.name();
                match name.as_ref() {
                    b"file" => file_index += 1,
                    b"segments" => in_segments = false,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(NntpError::InvalidResponse(format!(
                    "XML parse error in segments: {}",
                    e
                )))
            }
            _ => {}
        }

        buf.clear();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_nzb_simple() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE nzb PUBLIC "-//newzBin//DTD NZB 1.1//EN" "http://www.newzbin.com/DTD/nzb/nzb-1.1.dtd">
<nzb xmlns="http://www.newzbin.com/DTD/2003/nzb">
  <head>
    <meta type="title">Test File</meta>
  </head>
  <file poster="user@example.com" date="1234567890" subject="Test [1/1]">
    <groups>
      <group>alt.binaries.test</group>
    </groups>
    <segments>
      <segment bytes="768000" number="1">part1of1@example.com</segment>
    </segments>
  </file>
</nzb>"#;

        let nzb = parse_nzb(xml).unwrap();

        assert_eq!(nzb.meta.get("title"), Some(&"Test File".to_string()));
        assert_eq!(nzb.files.len(), 1);

        let file = &nzb.files[0];
        assert_eq!(file.poster, "user@example.com");
        assert_eq!(file.date, 1234567890);
        assert_eq!(file.subject, "Test [1/1]");
        assert_eq!(file.groups, vec!["alt.binaries.test"]);
        assert_eq!(file.segments.len(), 1);

        let seg = &file.segments[0];
        assert_eq!(seg.bytes, 768000);
        assert_eq!(seg.number, 1);
        assert_eq!(seg.message_id, "part1of1@example.com");
    }

    #[test]
    fn test_parse_nzb_multiple_segments() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<nzb xmlns="http://www.newzbin.com/DTD/2003/nzb">
  <file poster="poster@example.com" date="1600000000" subject="Multi-part [1/3]">
    <groups>
      <group>alt.binaries.test</group>
    </groups>
    <segments>
      <segment bytes="100000" number="1">seg1@example.com</segment>
      <segment bytes="100000" number="2">seg2@example.com</segment>
      <segment bytes="100000" number="3">seg3@example.com</segment>
    </segments>
  </file>
</nzb>"#;

        let nzb = parse_nzb(xml).unwrap();
        assert_eq!(nzb.files.len(), 1);
        assert_eq!(nzb.files[0].segments.len(), 3);
        assert_eq!(nzb.files[0].total_bytes(), 300000);

        // Validate segments are sequential
        assert!(nzb.files[0].validate_segments().is_ok());
    }

    #[test]
    fn test_parse_nzb_multiple_files() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<nzb xmlns="http://www.newzbin.com/DTD/2003/nzb">
  <file poster="user1@example.com" date="1234567890" subject="File 1">
    <groups>
      <group>alt.binaries.test</group>
    </groups>
    <segments>
      <segment bytes="50000" number="1">file1seg1@example.com</segment>
    </segments>
  </file>
  <file poster="user2@example.com" date="1234567900" subject="File 2">
    <groups>
      <group>alt.binaries.test</group>
    </groups>
    <segments>
      <segment bytes="75000" number="1">file2seg1@example.com</segment>
    </segments>
  </file>
</nzb>"#;

        let nzb = parse_nzb(xml).unwrap();
        assert_eq!(nzb.files.len(), 2);
        assert_eq!(nzb.total_bytes(), 125000);

        assert_eq!(nzb.files[0].subject, "File 1");
        assert_eq!(nzb.files[1].subject, "File 2");
    }

    #[test]
    fn test_parse_nzb_multiple_groups() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<nzb xmlns="http://www.newzbin.com/DTD/2003/nzb">
  <file poster="user@example.com" date="1234567890" subject="Cross-posted">
    <groups>
      <group>alt.binaries.test</group>
      <group>alt.binaries.backup</group>
      <group>comp.test</group>
    </groups>
    <segments>
      <segment bytes="10000" number="1">msg@example.com</segment>
    </segments>
  </file>
</nzb>"#;

        let nzb = parse_nzb(xml).unwrap();
        assert_eq!(nzb.files[0].groups.len(), 3);
        assert_eq!(nzb.files[0].groups[0], "alt.binaries.test");
        assert_eq!(nzb.files[0].groups[1], "alt.binaries.backup");
        assert_eq!(nzb.files[0].groups[2], "comp.test");
    }

    #[test]
    fn test_parse_nzb_meta_tags() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<nzb xmlns="http://www.newzbin.com/DTD/2003/nzb">
  <head>
    <meta type="title">My Download</meta>
    <meta type="password">secret123</meta>
    <meta type="tag">linux</meta>
    <meta type="category">software</meta>
  </head>
  <file poster="user@example.com" date="1234567890" subject="Test">
    <groups>
      <group>alt.binaries.test</group>
    </groups>
    <segments>
      <segment bytes="1000" number="1">msg@example.com</segment>
    </segments>
  </file>
</nzb>"#;

        let nzb = parse_nzb(xml).unwrap();
        assert_eq!(nzb.meta.get("title"), Some(&"My Download".to_string()));
        assert_eq!(nzb.meta.get("password"), Some(&"secret123".to_string()));
        assert_eq!(nzb.meta.get("tag"), Some(&"linux".to_string()));
        assert_eq!(nzb.meta.get("category"), Some(&"software".to_string()));
    }

    #[test]
    fn test_validate_segments_sequential() {
        let file = NzbFile {
            poster: "user@example.com".to_string(),
            date: 1234567890,
            subject: "Test".to_string(),
            groups: vec!["alt.test".to_string()],
            segments: vec![
                NzbSegment {
                    bytes: 100,
                    number: 1,
                    message_id: "seg1@example.com".to_string(),
                },
                NzbSegment {
                    bytes: 100,
                    number: 2,
                    message_id: "seg2@example.com".to_string(),
                },
                NzbSegment {
                    bytes: 100,
                    number: 3,
                    message_id: "seg3@example.com".to_string(),
                },
            ],
        };

        assert!(file.validate_segments().is_ok());
    }

    #[test]
    fn test_validate_segments_missing() {
        let file = NzbFile {
            poster: "user@example.com".to_string(),
            date: 1234567890,
            subject: "Test".to_string(),
            groups: vec!["alt.test".to_string()],
            segments: vec![
                NzbSegment {
                    bytes: 100,
                    number: 1,
                    message_id: "seg1@example.com".to_string(),
                },
                NzbSegment {
                    bytes: 100,
                    number: 3,
                    message_id: "seg3@example.com".to_string(),
                },
            ],
        };

        let result = file.validate_segments();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing segment"));
    }

    #[test]
    fn test_validate_segments_duplicate() {
        let file = NzbFile {
            poster: "user@example.com".to_string(),
            date: 1234567890,
            subject: "Test".to_string(),
            groups: vec!["alt.test".to_string()],
            segments: vec![
                NzbSegment {
                    bytes: 100,
                    number: 1,
                    message_id: "seg1@example.com".to_string(),
                },
                NzbSegment {
                    bytes: 100,
                    number: 1,
                    message_id: "seg1dup@example.com".to_string(),
                },
            ],
        };

        let result = file.validate_segments();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Duplicate"));
    }

    #[test]
    fn test_validate_segments_empty() {
        let file = NzbFile {
            poster: "user@example.com".to_string(),
            date: 1234567890,
            subject: "Test".to_string(),
            groups: vec!["alt.test".to_string()],
            segments: vec![],
        };

        let result = file.validate_segments();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no segments"));
    }

    #[test]
    fn test_missing_segments_detection() {
        let file = NzbFile {
            poster: "user@example.com".to_string(),
            date: 1234567890,
            subject: "Test".to_string(),
            groups: vec!["alt.test".to_string()],
            segments: vec![
                NzbSegment {
                    bytes: 100,
                    number: 1,
                    message_id: "seg1@example.com".to_string(),
                },
                NzbSegment {
                    bytes: 100,
                    number: 3,
                    message_id: "seg3@example.com".to_string(),
                },
                NzbSegment {
                    bytes: 100,
                    number: 5,
                    message_id: "seg5@example.com".to_string(),
                },
            ],
        };

        let missing = file.missing_segments();
        assert_eq!(missing, vec![2, 4]);
    }

    #[test]
    fn test_parse_nzb_message_ids_with_angles() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<nzb xmlns="http://www.newzbin.com/DTD/2003/nzb">
  <file poster="user@example.com" date="1234567890" subject="Test">
    <groups>
      <group>alt.binaries.test</group>
    </groups>
    <segments>
      <segment bytes="1000" number="1">&lt;part1@example.com&gt;</segment>
    </segments>
  </file>
</nzb>"#;

        let nzb = parse_nzb(xml).unwrap();
        assert_eq!(nzb.files[0].segments[0].message_id, "<part1@example.com>");
    }

    #[test]
    fn test_parse_nzb_special_characters() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<nzb xmlns="http://www.newzbin.com/DTD/2003/nzb">
  <head>
    <meta type="title">File &amp; Stuff</meta>
  </head>
  <file poster="user@example.com" date="1234567890" subject="Test &quot;quoted&quot;">
    <groups>
      <group>alt.binaries.test</group>
    </groups>
    <segments>
      <segment bytes="1000" number="1">msg@example.com</segment>
    </segments>
  </file>
</nzb>"#;

        let nzb = parse_nzb(xml).unwrap();
        assert_eq!(nzb.meta.get("title"), Some(&"File & Stuff".to_string()));
        assert_eq!(nzb.files[0].subject, r#"Test "quoted""#);
    }

    #[test]
    fn test_nzb_validate_all_files() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<nzb xmlns="http://www.newzbin.com/DTD/2003/nzb">
  <file poster="user@example.com" date="1234567890" subject="File 1">
    <groups>
      <group>alt.binaries.test</group>
    </groups>
    <segments>
      <segment bytes="100" number="1">seg1@example.com</segment>
    </segments>
  </file>
  <file poster="user@example.com" date="1234567890" subject="File 2">
    <groups>
      <group>alt.binaries.test</group>
    </groups>
    <segments>
      <segment bytes="200" number="1">seg2@example.com</segment>
      <segment bytes="200" number="2">seg3@example.com</segment>
    </segments>
  </file>
</nzb>"#;

        let nzb = parse_nzb(xml).unwrap();
        assert!(nzb.validate().is_ok());
    }

    #[test]
    fn test_parse_nzb_empty_meta() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<nzb xmlns="http://www.newzbin.com/DTD/2003/nzb">
  <file poster="user@example.com" date="1234567890" subject="Test">
    <groups>
      <group>alt.binaries.test</group>
    </groups>
    <segments>
      <segment bytes="1000" number="1">msg@example.com</segment>
    </segments>
  </file>
</nzb>"#;

        let nzb = parse_nzb(xml).unwrap();
        assert!(nzb.meta.is_empty());
        assert_eq!(nzb.files.len(), 1);
    }

    #[test]
    fn test_parse_nzb_binary_newsgroup() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<nzb xmlns="http://www.newzbin.com/DTD/2003/nzb">
  <file poster="BigUploader &lt;uploader@example.com&gt;" date="1600000000" subject="[01/10] - &quot;archive.part01.rar&quot; yEnc (1/25)">
    <groups>
      <group>alt.binaries.boneless</group>
      <group>alt.binaries.test</group>
    </groups>
    <segments>
      <segment bytes="768000" number="1">Xm4D9P2qE@JBinUp.local</segment>
      <segment bytes="768000" number="2">Xm4D9P2qF@JBinUp.local</segment>
      <segment bytes="500000" number="3">Xm4D9P2qG@JBinUp.local</segment>
    </segments>
  </file>
</nzb>"#;

        let nzb = parse_nzb(xml).unwrap();
        assert_eq!(nzb.files.len(), 1);

        let file = &nzb.files[0];
        assert!(file.poster.contains("BigUploader"));
        assert!(file.subject.contains("archive.part01.rar"));
        assert_eq!(file.groups.len(), 2);
        assert_eq!(file.segments.len(), 3);
        assert_eq!(file.total_bytes(), 2036000);
        assert!(file.validate_segments().is_ok());
    }

    // Generator tests

    #[test]
    fn test_to_xml_simple() {
        let nzb = Nzb {
            meta: HashMap::from([("title".to_string(), "Test File".to_string())]),
            files: vec![NzbFile {
                poster: "user@example.com".to_string(),
                date: 1234567890,
                subject: "Test [1/1]".to_string(),
                groups: vec!["alt.binaries.test".to_string()],
                segments: vec![NzbSegment {
                    bytes: 768000,
                    number: 1,
                    message_id: "part1of1@example.com".to_string(),
                }],
            }],
        };

        let xml = nzb.to_xml();

        // Verify XML structure
        assert!(xml.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml.contains("<!DOCTYPE nzb"));
        assert!(xml.contains("<nzb xmlns=\"http://www.newzbin.com/DTD/2003/nzb\">"));
        assert!(xml.contains("<head>"));
        assert!(xml.contains("<meta type=\"title\">Test File</meta>"));
        assert!(xml.contains("poster=\"user@example.com\""));
        assert!(xml.contains("date=\"1234567890\""));
        assert!(xml.contains("subject=\"Test [1/1]\""));
        assert!(xml.contains("<group>alt.binaries.test</group>"));
        assert!(xml.contains("bytes=\"768000\""));
        assert!(xml.contains("number=\"1\""));
        assert!(xml.contains("part1of1@example.com"));
    }

    #[test]
    fn test_to_xml_roundtrip() {
        let original = Nzb {
            meta: HashMap::from([
                ("title".to_string(), "Roundtrip Test".to_string()),
                ("password".to_string(), "secret123".to_string()),
            ]),
            files: vec![NzbFile {
                poster: "uploader@test.com".to_string(),
                date: 1600000000,
                subject: "Test File [1/2]".to_string(),
                groups: vec!["alt.binaries.test".to_string(), "alt.test".to_string()],
                segments: vec![
                    NzbSegment {
                        bytes: 100000,
                        number: 1,
                        message_id: "seg1@example.com".to_string(),
                    },
                    NzbSegment {
                        bytes: 200000,
                        number: 2,
                        message_id: "seg2@example.com".to_string(),
                    },
                ],
            }],
        };

        let xml = original.to_xml();
        let parsed = parse_nzb(&xml).unwrap();

        // Verify roundtrip preserves all data
        assert_eq!(parsed.meta.get("title"), original.meta.get("title"));
        assert_eq!(parsed.meta.get("password"), original.meta.get("password"));
        assert_eq!(parsed.files.len(), original.files.len());

        let parsed_file = &parsed.files[0];
        let orig_file = &original.files[0];

        assert_eq!(parsed_file.poster, orig_file.poster);
        assert_eq!(parsed_file.date, orig_file.date);
        assert_eq!(parsed_file.subject, orig_file.subject);
        assert_eq!(parsed_file.groups, orig_file.groups);
        assert_eq!(parsed_file.segments.len(), orig_file.segments.len());

        for (parsed_seg, orig_seg) in parsed_file.segments.iter().zip(&orig_file.segments) {
            assert_eq!(parsed_seg.bytes, orig_seg.bytes);
            assert_eq!(parsed_seg.number, orig_seg.number);
            assert_eq!(parsed_seg.message_id, orig_seg.message_id);
        }
    }

    #[test]
    fn test_to_xml_xml_escaping() {
        let nzb = Nzb {
            meta: HashMap::from([(
                "title".to_string(),
                "Test <with> & \"special\" 'chars'".to_string(),
            )]),
            files: vec![NzbFile {
                poster: "user<test>@example.com".to_string(),
                date: 1234567890,
                subject: "File & <data> [1/1]".to_string(),
                groups: vec!["alt.binaries.<test>".to_string()],
                segments: vec![NzbSegment {
                    bytes: 1000,
                    number: 1,
                    message_id: "<msg&id>@example.com".to_string(),
                }],
            }],
        };

        let xml = nzb.to_xml();

        // Verify special characters are escaped in element text (BytesText handles this)
        assert!(xml.contains("Test &lt;with&gt; &amp; &quot;special&quot; &apos;chars&apos;"));
        // Attributes are manually escaped
        assert!(xml.contains("user&lt;test&gt;@example.com"));
        assert!(xml.contains("File &amp; &lt;data&gt; [1/1]"));
        // Element text
        assert!(xml.contains("alt.binaries.&lt;test&gt;"));
        assert!(xml.contains("&lt;msg&amp;id&gt;@example.com"));

        // Verify roundtrip works with escaped characters
        let parsed = parse_nzb(&xml).unwrap();
        assert_eq!(
            parsed.meta.get("title"),
            Some(&"Test <with> & \"special\" 'chars'".to_string())
        );
        assert_eq!(parsed.files[0].poster, "user<test>@example.com");
        assert_eq!(parsed.files[0].subject, "File & <data> [1/1]");
    }

    #[test]
    fn test_to_xml_multiple_files() {
        let nzb = Nzb {
            meta: HashMap::new(),
            files: vec![
                NzbFile {
                    poster: "user1@example.com".to_string(),
                    date: 1234567890,
                    subject: "File1 [1/1]".to_string(),
                    groups: vec!["alt.binaries.test".to_string()],
                    segments: vec![NzbSegment {
                        bytes: 1000,
                        number: 1,
                        message_id: "file1@example.com".to_string(),
                    }],
                },
                NzbFile {
                    poster: "user2@example.com".to_string(),
                    date: 1234567891,
                    subject: "File2 [1/1]".to_string(),
                    groups: vec!["alt.binaries.test".to_string()],
                    segments: vec![NzbSegment {
                        bytes: 2000,
                        number: 1,
                        message_id: "file2@example.com".to_string(),
                    }],
                },
            ],
        };

        let xml = nzb.to_xml();

        // Verify both files are present
        assert!(xml.contains("File1 [1/1]"));
        assert!(xml.contains("File2 [1/1]"));
        assert!(xml.contains("user1@example.com"));
        assert!(xml.contains("user2@example.com"));
        assert!(xml.contains("file1@example.com"));
        assert!(xml.contains("file2@example.com"));

        // Verify roundtrip
        let parsed = parse_nzb(&xml).unwrap();
        assert_eq!(parsed.files.len(), 2);
    }

    #[test]
    fn test_to_xml_no_metadata() {
        let nzb = Nzb {
            meta: HashMap::new(),
            files: vec![NzbFile {
                poster: "user@example.com".to_string(),
                date: 1234567890,
                subject: "Test [1/1]".to_string(),
                groups: vec!["alt.binaries.test".to_string()],
                segments: vec![NzbSegment {
                    bytes: 1000,
                    number: 1,
                    message_id: "test@example.com".to_string(),
                }],
            }],
        };

        let xml = nzb.to_xml();

        // Verify no <head> section when no metadata
        assert!(!xml.contains("<head>"));
        assert!(!xml.contains("<meta"));

        // But structure should still be valid
        assert!(xml.contains("<nzb"));
        assert!(xml.contains("<file"));

        // Verify roundtrip
        let parsed = parse_nzb(&xml).unwrap();
        assert!(parsed.meta.is_empty());
        assert_eq!(parsed.files.len(), 1);
    }

    #[test]
    fn test_to_xml_multiple_groups() {
        let nzb = Nzb {
            meta: HashMap::new(),
            files: vec![NzbFile {
                poster: "user@example.com".to_string(),
                date: 1234567890,
                subject: "Crossposted [1/1]".to_string(),
                groups: vec![
                    "alt.binaries.test".to_string(),
                    "alt.binaries.other".to_string(),
                    "comp.os.test".to_string(),
                ],
                segments: vec![NzbSegment {
                    bytes: 1000,
                    number: 1,
                    message_id: "test@example.com".to_string(),
                }],
            }],
        };

        let xml = nzb.to_xml();

        // Verify all groups are present
        assert!(xml.contains("<group>alt.binaries.test</group>"));
        assert!(xml.contains("<group>alt.binaries.other</group>"));
        assert!(xml.contains("<group>comp.os.test</group>"));

        // Verify roundtrip
        let parsed = parse_nzb(&xml).unwrap();
        assert_eq!(parsed.files[0].groups.len(), 3);
        assert_eq!(parsed.files[0].groups[0], "alt.binaries.test");
        assert_eq!(parsed.files[0].groups[1], "alt.binaries.other");
        assert_eq!(parsed.files[0].groups[2], "comp.os.test");
    }

    #[test]
    fn test_to_xml_many_segments() {
        let segments: Vec<NzbSegment> = (1..=100)
            .map(|i| NzbSegment {
                bytes: 10000,
                number: i,
                message_id: format!("seg{}@example.com", i),
            })
            .collect();

        let nzb = Nzb {
            meta: HashMap::new(),
            files: vec![NzbFile {
                poster: "user@example.com".to_string(),
                date: 1234567890,
                subject: "Large file [100/100]".to_string(),
                groups: vec!["alt.binaries.test".to_string()],
                segments,
            }],
        };

        let xml = nzb.to_xml();

        // Verify all segments present
        assert!(xml.contains("seg1@example.com"));
        assert!(xml.contains("seg50@example.com"));
        assert!(xml.contains("seg100@example.com"));

        // Verify roundtrip
        let parsed = parse_nzb(&xml).unwrap();
        assert_eq!(parsed.files[0].segments.len(), 100);
        assert_eq!(parsed.files[0].segments[0].number, 1);
        assert_eq!(parsed.files[0].segments[99].number, 100);
    }

    #[test]
    fn test_to_xml_unicode_characters() {
        let nzb = Nzb {
            meta: HashMap::from([("title".to_string(), "Test 文件 🎉".to_string())]),
            files: vec![NzbFile {
                poster: "用户@example.com".to_string(),
                date: 1234567890,
                subject: "Тестовый файл [1/1]".to_string(),
                groups: vec!["alt.binaries.тест".to_string()],
                segments: vec![NzbSegment {
                    bytes: 1000,
                    number: 1,
                    message_id: "📧@example.com".to_string(),
                }],
            }],
        };

        let xml = nzb.to_xml();

        // Verify unicode is preserved
        assert!(xml.contains("Test 文件 🎉"));
        assert!(xml.contains("用户@example.com"));
        assert!(xml.contains("Тестовый файл"));
        assert!(xml.contains("alt.binaries.тест"));

        // Verify roundtrip preserves unicode
        let parsed = parse_nzb(&xml).unwrap();
        assert_eq!(parsed.meta.get("title"), Some(&"Test 文件 🎉".to_string()));
        assert_eq!(parsed.files[0].poster, "用户@example.com");
        assert_eq!(parsed.files[0].subject, "Тестовый файл [1/1]");
    }

    #[test]
    fn test_to_xml_all_meta_types() {
        let nzb = Nzb {
            meta: HashMap::from([
                ("title".to_string(), "Complete Archive".to_string()),
                ("password".to_string(), "secret123".to_string()),
                ("tag".to_string(), "movies".to_string()),
                ("category".to_string(), "TV".to_string()),
            ]),
            files: vec![NzbFile {
                poster: "user@example.com".to_string(),
                date: 1234567890,
                subject: "Test [1/1]".to_string(),
                groups: vec!["alt.binaries.test".to_string()],
                segments: vec![NzbSegment {
                    bytes: 1000,
                    number: 1,
                    message_id: "test@example.com".to_string(),
                }],
            }],
        };

        let xml = nzb.to_xml();

        // Verify all meta types are present
        assert!(xml.contains("<meta type=\"title\">Complete Archive</meta>"));
        assert!(xml.contains("<meta type=\"password\">secret123</meta>"));
        assert!(xml.contains("<meta type=\"tag\">movies</meta>"));
        assert!(xml.contains("<meta type=\"category\">TV</meta>"));

        // Verify roundtrip
        let parsed = parse_nzb(&xml).unwrap();
        assert_eq!(parsed.meta.len(), 4);
        assert_eq!(
            parsed.meta.get("title"),
            Some(&"Complete Archive".to_string())
        );
        assert_eq!(parsed.meta.get("password"), Some(&"secret123".to_string()));
        assert_eq!(parsed.meta.get("tag"), Some(&"movies".to_string()));
        assert_eq!(parsed.meta.get("category"), Some(&"TV".to_string()));
    }

    #[test]
    fn test_to_xml_large_numbers() {
        let nzb = Nzb {
            meta: HashMap::new(),
            files: vec![NzbFile {
                poster: "user@example.com".to_string(),
                date: i64::MAX,
                subject: "Test [1/1]".to_string(),
                groups: vec!["alt.binaries.test".to_string()],
                segments: vec![NzbSegment {
                    bytes: u64::MAX,
                    number: u32::MAX,
                    message_id: "test@example.com".to_string(),
                }],
            }],
        };

        let xml = nzb.to_xml();

        // Verify large numbers are preserved
        assert!(xml.contains(&i64::MAX.to_string()));
        assert!(xml.contains(&u64::MAX.to_string()));
        assert!(xml.contains(&u32::MAX.to_string()));

        // Verify roundtrip
        let parsed = parse_nzb(&xml).unwrap();
        assert_eq!(parsed.files[0].date, i64::MAX);
        assert_eq!(parsed.files[0].segments[0].bytes, u64::MAX);
        assert_eq!(parsed.files[0].segments[0].number, u32::MAX);
    }
}

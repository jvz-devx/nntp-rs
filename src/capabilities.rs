//! NNTP capabilities parsing and storage (RFC 3977 Section 5.2)
//!
//! The CAPABILITIES command returns a list of capabilities supported by the server.
//! Each capability may have optional arguments.

use std::collections::HashMap;

/// Represents the capabilities supported by an NNTP server
#[must_use]
#[derive(Debug, Clone)]
pub struct Capabilities {
    /// Map of capability name to its arguments
    /// Example: "COMPRESS" -> ["DEFLATE", "GZIP"]
    capabilities: HashMap<String, Vec<String>>,
}

impl Capabilities {
    /// Create an empty Capabilities instance
    pub fn new() -> Self {
        Self {
            capabilities: HashMap::new(),
        }
    }

    /// Parse capabilities from NNTP response lines
    ///
    /// # Format
    /// Each line is: `CAPABILITY [arg1 arg2 ...]`
    ///
    /// # Example
    /// ```text
    /// VERSION 2
    /// READER
    /// POST
    /// IHAVE
    /// COMPRESS DEFLATE GZIP
    /// ```
    pub fn parse(lines: &[String]) -> Self {
        let mut capabilities = HashMap::new();

        for line in lines {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            let capability = parts[0].to_uppercase();
            let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
            capabilities.insert(capability, args);
        }

        Self { capabilities }
    }

    /// Check if a capability is supported
    #[must_use]
    pub fn has(&self, capability: &str) -> bool {
        self.capabilities.contains_key(&capability.to_uppercase())
    }

    /// Get arguments for a capability
    ///
    /// Returns None if the capability is not supported
    #[must_use]
    pub fn get_args(&self, capability: &str) -> Option<&Vec<String>> {
        self.capabilities.get(&capability.to_uppercase())
    }

    /// Get all capability names
    pub fn list(&self) -> Vec<String> {
        self.capabilities.keys().cloned().collect()
    }

    /// Check if the server supports a specific capability with a specific argument
    ///
    /// # Example
    /// ```no_run
    /// # use nntp_rs::Capabilities;
    /// # let caps = Capabilities::new();
    /// if caps.has_arg("COMPRESS", "DEFLATE") {
    ///     println!("Server supports DEFLATE compression");
    /// }
    /// ```
    pub fn has_arg(&self, capability: &str, arg: &str) -> bool {
        self.get_args(capability)
            .map(|args| args.iter().any(|a| a.eq_ignore_ascii_case(arg)))
            .unwrap_or(false)
    }
}

impl Default for Capabilities {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_capabilities() {
        let lines = vec![
            "VERSION 2".to_string(),
            "READER".to_string(),
            "POST".to_string(),
            "COMPRESS DEFLATE GZIP".to_string(),
        ];

        let caps = Capabilities::parse(&lines);

        assert!(caps.has("VERSION"));
        assert!(caps.has("READER"));
        assert!(caps.has("POST"));
        assert!(caps.has("COMPRESS"));
        assert!(!caps.has("STREAMING"));
    }

    #[test]
    fn test_capability_args() {
        let lines = vec!["COMPRESS DEFLATE GZIP".to_string(), "VERSION 2".to_string()];

        let caps = Capabilities::parse(&lines);

        // .unwrap() is safe here: test input guarantees capability exists
        let compress_args = caps.get_args("COMPRESS").unwrap();
        assert_eq!(compress_args.len(), 2);
        assert_eq!(compress_args[0], "DEFLATE");
        assert_eq!(compress_args[1], "GZIP");

        // .unwrap() is safe here: test input guarantees capability exists
        let version_args = caps.get_args("VERSION").unwrap();
        assert_eq!(version_args.len(), 1);
        assert_eq!(version_args[0], "2");
    }

    #[test]
    fn test_has_arg() {
        let lines = vec!["COMPRESS DEFLATE GZIP".to_string()];
        let caps = Capabilities::parse(&lines);

        assert!(caps.has_arg("COMPRESS", "DEFLATE"));
        assert!(caps.has_arg("COMPRESS", "GZIP"));
        assert!(!caps.has_arg("COMPRESS", "BZIP2"));
        assert!(!caps.has_arg("STREAMING", "CHECK"));
    }

    #[test]
    fn test_case_insensitive() {
        let lines = vec!["compress deflate gzip".to_string()];
        let caps = Capabilities::parse(&lines);

        assert!(caps.has("COMPRESS"));
        assert!(caps.has("compress"));
        assert!(caps.has_arg("COMPRESS", "deflate"));
        assert!(caps.has_arg("compress", "DEFLATE"));
    }

    #[test]
    fn test_empty_lines() {
        let lines = vec!["".to_string(), "VERSION 2".to_string(), "".to_string()];
        let caps = Capabilities::parse(&lines);

        assert!(caps.has("VERSION"));
        assert_eq!(caps.list().len(), 1);
    }

    #[test]
    fn test_list_capabilities() {
        let lines = vec![
            "VERSION 2".to_string(),
            "READER".to_string(),
            "POST".to_string(),
        ];
        let caps = Capabilities::parse(&lines);

        let list = caps.list();
        assert_eq!(list.len(), 3);
        assert!(list.contains(&"VERSION".to_string()));
        assert!(list.contains(&"READER".to_string()));
        assert!(list.contains(&"POST".to_string()));
    }

    #[test]
    fn test_get_args_missing_capability() {
        let lines = vec!["VERSION 2".to_string(), "READER".to_string()];
        let caps = Capabilities::parse(&lines);

        // get_args returns None for missing capabilities
        assert!(caps.get_args("COMPRESS").is_none());
        assert!(caps.get_args("STREAMING").is_none());
        assert!(caps.get_args("NONEXISTENT").is_none());
    }

    #[test]
    fn test_capability_with_no_args() {
        let lines = vec!["READER".to_string(), "POST".to_string()];
        let caps = Capabilities::parse(&lines);

        // Capabilities without arguments have empty arg vectors
        assert!(caps.has("READER"));
        let reader_args = caps.get_args("READER").unwrap();
        assert_eq!(reader_args.len(), 0);

        assert!(caps.has("POST"));
        let post_args = caps.get_args("POST").unwrap();
        assert_eq!(post_args.len(), 0);
    }
}

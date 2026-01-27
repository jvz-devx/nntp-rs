//! NNTP server configuration

/// NNTP server configuration
///
/// Contains all the information needed to connect to an NNTP server.
///
/// # Example
///
/// ```
/// use nntp_rs::ServerConfig;
///
/// // Recommended: use the constructor methods
/// let config = ServerConfig::tls("news.example.com", "user", "pass");
///
/// // Or construct manually
/// let config = ServerConfig {
///     host: "news.example.com".to_string(),
///     port: 563,
///     tls: true,
///     allow_insecure_tls: false,
///     username: "user".to_string(),
///     password: "pass".to_string(),
/// };
/// ```
#[must_use]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ServerConfig {
    /// Server hostname (e.g., "news.example.com")
    pub host: String,

    /// Server port (typically 119 for plain, 563 for TLS)
    pub port: u16,

    /// Use TLS/SSL encryption
    ///
    /// **Note:** Currently, TLS is always enabled when connecting to port 563.
    /// This field is maintained for configuration compatibility but does not
    /// affect runtime behavior. To use a plain unencrypted connection, use port 119
    /// (via `ServerConfig::plain()` or specify `port: 119` manually).
    ///
    /// In a future major version (0.2.0+), this may be replaced with a `TlsMode` enum
    /// to eliminate the separate `allow_insecure_tls` field and provide clearer semantics.
    #[cfg_attr(feature = "serde", serde(default = "default_tls"))]
    pub tls: bool,

    /// Allow insecure TLS connections (self-signed certificates, expired certificates)
    ///
    /// **Security Warning:** Setting this to `true` disables certificate validation,
    /// making your connection vulnerable to man-in-the-middle attacks. Only use this
    /// for testing or with servers you trust on a secure network.
    ///
    /// When `true`:
    /// - Self-signed certificates are accepted
    /// - Expired certificates are accepted
    /// - Certificate hostname mismatches are accepted
    /// - Invalid certificate chains are accepted
    ///
    /// Default: `false` (secure certificate validation enabled)
    #[cfg_attr(feature = "serde", serde(default))]
    pub allow_insecure_tls: bool,

    /// Username for authentication
    pub username: String,

    /// Password for authentication
    pub password: String,
}

#[cfg(feature = "serde")]
fn default_tls() -> bool {
    true
}

impl ServerConfig {
    /// Create a new server configuration
    ///
    /// # Arguments
    ///
    /// * `host` - Server hostname
    /// * `port` - Server port
    /// * `tls` - Whether to use TLS/SSL
    /// * `username` - Authentication username
    /// * `password` - Authentication password
    pub fn new(
        host: impl Into<String>,
        port: u16,
        tls: bool,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            host: host.into(),
            port,
            tls,
            allow_insecure_tls: false,
            username: username.into(),
            password: password.into(),
        }
    }

    /// Create a configuration for a TLS connection on the standard secure port (563)
    pub fn tls(
        host: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self::new(host, 563, true, username, password)
    }

    /// Create a configuration for a plain connection on the standard port (119)
    ///
    /// **Warning:** Plain connections transmit credentials in clear text.
    /// Use TLS connections whenever possible.
    pub fn plain(
        host: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self::new(host, 119, false, username, password)
    }

    /// Create a TLS configuration that accepts self-signed certificates
    ///
    /// **Security Warning:** This configuration disables certificate validation,
    /// making your connection vulnerable to man-in-the-middle attacks. Only use
    /// this for testing or with servers you trust on a secure network.
    ///
    /// # Example
    ///
    /// ```
    /// use nntp_rs::ServerConfig;
    ///
    /// // For a local NNTP server with a self-signed certificate
    /// let config = ServerConfig::tls_insecure("localhost", "user", "pass");
    /// ```
    pub fn tls_insecure(
        host: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        let mut config = Self::tls(host, username, password);
        config.allow_insecure_tls = true;
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let config = ServerConfig::new("news.example.com", 563, true, "user", "pass");
        assert_eq!(config.host, "news.example.com");
        assert_eq!(config.port, 563);
        assert!(config.tls);
        assert!(!config.allow_insecure_tls);
        assert_eq!(config.username, "user");
        assert_eq!(config.password, "pass");
    }

    #[test]
    fn test_tls_helper() {
        let config = ServerConfig::tls("news.example.com", "user", "pass");
        assert_eq!(config.port, 563);
        assert!(config.tls);
        assert!(!config.allow_insecure_tls);
    }

    #[test]
    fn test_plain_helper() {
        let config = ServerConfig::plain("news.example.com", "user", "pass");
        assert_eq!(config.port, 119);
        assert!(!config.tls);
        assert!(!config.allow_insecure_tls);
    }

    #[test]
    fn test_tls_insecure_helper() {
        let config = ServerConfig::tls_insecure("localhost", "user", "pass");
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 563);
        assert!(config.tls);
        assert!(config.allow_insecure_tls);
        assert_eq!(config.username, "user");
        assert_eq!(config.password, "pass");
    }

    #[test]
    fn test_insecure_tls_default_false() {
        let config = ServerConfig::new("news.example.com", 563, true, "user", "pass");
        assert!(!config.allow_insecure_tls);
    }
}

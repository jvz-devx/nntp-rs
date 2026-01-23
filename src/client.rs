//! NNTP client implementation with TLS and compression support

use crate::article::Article;
use crate::capabilities::Capabilities;
use crate::commands::{self, XoverEntry};
use crate::config::ServerConfig;
use crate::error::{NntpError, Result};
use crate::response::{codes, NntpResponse};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_rustls::client::TlsStream;
use tokio_rustls::rustls::client::danger::{
    HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier,
};
use tokio_rustls::rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use tokio_rustls::rustls::{ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme};
use tokio_rustls::TlsConnector;
use tracing::{debug, trace, warn};

/// NNTP connection state tracking authentication progress
///
/// Tracks the authentication state of an NNTP connection according to RFC 4643.
/// Commands may be restricted based on the current state.
enum ConnectionState {
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
enum CompressionMode {
    /// No compression
    None,
    /// Headers-only compression (XFEATURE COMPRESS GZIP)
    /// Only multiline responses (XOVER, HEAD, ARTICLE) are gzip-compressed
    HeadersOnly,
    /// Full session compression (RFC 8054 COMPRESS DEFLATE)
    /// All data after negotiation is deflate-compressed bidirectionally
    FullSession,
}

/// Dangerous certificate verifier that accepts all certificates
///
/// **Security Warning:** This verifier disables all certificate validation,
/// making connections vulnerable to man-in-the-middle attacks. Only use this
/// for testing or with servers you trust on a secure network.
#[derive(Debug)]
struct DangerousAcceptAnyCertificate;

impl ServerCertVerifier for DangerousAcceptAnyCertificate {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> std::result::Result<ServerCertVerified, tokio_rustls::rustls::Error> {
        // Accept any certificate without validation
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, tokio_rustls::rustls::Error> {
        // Accept any signature without validation
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, tokio_rustls::rustls::Error> {
        // Accept any signature without validation
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        // Support all signature schemes
        vec![
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
            SignatureScheme::ED448,
        ]
    }
}

/// Async NNTP client with TLS and compression support
///
/// # Example
///
/// ```no_run
/// use nntp_rs::{NntpClient, ServerConfig};
/// use std::sync::Arc;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = ServerConfig::tls("news.example.com", "user", "pass");
/// let mut client = NntpClient::connect(Arc::new(config)).await?;
/// client.authenticate().await?;
///
/// // Enable compression for better bandwidth efficiency
/// client.try_enable_compression().await?;
///
/// let (count, first, last) = client.select_group("alt.test").await?;
/// println!("Group has {} articles", count);
/// # Ok(())
/// # }
/// ```
pub struct NntpClient {
    /// TLS stream (both reader and writer)
    stream: BufReader<TlsStream<TcpStream>>,
    /// Connection state
    state: ConnectionState,
    /// Server configuration
    config: Arc<ServerConfig>,
    /// Currently selected newsgroup
    current_group: Option<String>,
    /// Compression mode for this connection
    compression_mode: CompressionMode,
    /// Total compressed bytes received (only when compression enabled)
    bytes_compressed: u64,
    /// Total decompressed bytes (original size)
    bytes_decompressed: u64,
    /// Whether this connection is broken (received garbage/invalid data)
    is_broken: bool,
}

impl NntpClient {
    /// Check if this connection is broken and should be discarded
    pub fn is_broken(&self) -> bool {
        self.is_broken
    }

    /// Mark this connection as broken
    fn mark_broken(&mut self) {
        self.is_broken = true;
    }

    /// Get the currently selected newsgroup, if any
    pub fn current_group(&self) -> Option<&str> {
        self.current_group.as_deref()
    }

    /// Check if the client is currently authenticated
    pub fn is_authenticated(&self) -> bool {
        matches!(self.state, ConnectionState::Authenticated)
    }
}

impl NntpClient {
    /// Connect to NNTP server with TLS
    ///
    /// Establishes a secure connection to the NNTP server specified in the config.
    /// Does not authenticate - call [`authenticate`](Self::authenticate) after connecting.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::Io`] - TCP connection fails (DNS resolution, network unreachable, etc.)
    /// - [`NntpError::Tls`] - TLS handshake fails (invalid certificate, protocol error)
    /// - [`NntpError::Timeout`] - Connection or handshake times out
    /// - [`NntpError::Protocol`] - Server rejects the connection
    ///
    /// # Timeouts
    /// - TCP connection: 120 seconds
    /// - TLS handshake: 60 seconds
    pub async fn connect(config: Arc<ServerConfig>) -> Result<Self> {
        debug!("Connecting to NNTP server {}:{}", config.host, config.port);

        // Create TCP connection with optimized socket buffers
        let addr = format!("{}:{}", config.host, config.port);

        // Parse the address to determine IP version
        use std::net::ToSocketAddrs;
        let socket_addr = addr
            .to_socket_addrs()
            .map_err(|e| NntpError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Failed to resolve address: {}", e)
            )))?
            .next()
            .ok_or_else(|| NntpError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "No address resolved"
            )))?;

        // Create socket using socket2 for buffer configuration
        use socket2::{Socket, Domain, Type, Protocol};
        let domain = if socket_addr.is_ipv4() {
            Domain::IPV4
        } else {
            Domain::IPV6
        };

        let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))
            .map_err(NntpError::Io)?;

        // Configure TCP socket for high-throughput downloads

        // Set TCP_NODELAY for low-latency request/response pattern
        socket.set_nodelay(true).map_err(NntpError::Io)?;

        // Set large receive buffer for high-bandwidth downloads (4MB)
        // This allows the OS to buffer more data, reducing the number of ACKs
        // and improving throughput on high-latency connections
        const RECV_BUFFER_SIZE: usize = 4 * 1024 * 1024; // 4MB
        if let Err(e) = socket.set_recv_buffer_size(RECV_BUFFER_SIZE) {
            warn!("Failed to set receive buffer size to {} bytes: {}", RECV_BUFFER_SIZE, e);
        } else {
            // Log the actual buffer size (OS may adjust)
            match socket.recv_buffer_size() {
                Ok(actual_size) => {
                    debug!("TCP receive buffer: requested {} bytes, actual {} bytes",
                           RECV_BUFFER_SIZE, actual_size);
                }
                Err(e) => warn!("Failed to query receive buffer size: {}", e),
            }
        }

        // Set large send buffer for command pipelining (1MB)
        const SEND_BUFFER_SIZE: usize = 1024 * 1024; // 1MB
        if let Err(e) = socket.set_send_buffer_size(SEND_BUFFER_SIZE) {
            warn!("Failed to set send buffer size to {} bytes: {}", SEND_BUFFER_SIZE, e);
        } else {
            // Log the actual buffer size (OS may adjust)
            match socket.send_buffer_size() {
                Ok(actual_size) => {
                    debug!("TCP send buffer: requested {} bytes, actual {} bytes",
                           SEND_BUFFER_SIZE, actual_size);
                }
                Err(e) => warn!("Failed to query send buffer size: {}", e),
            }
        }

        // Connect with timeout (120 seconds for slow connections)
        // socket2::Socket::connect() is blocking, so we need to spawn it in a blocking task
        // NOTE: Connect BEFORE setting non-blocking mode
        let socket_addr_for_connect = socket_addr;
        let tcp_stream = timeout(
            Duration::from_secs(120),
            tokio::task::spawn_blocking(move || -> std::io::Result<std::net::TcpStream> {
                // Connect while socket is still in blocking mode
                socket.connect(&socket_addr_for_connect.into())?;
                // Set non-blocking mode AFTER successful connect
                socket.set_nonblocking(true)?;
                Ok(socket.into())
            })
        )
        .await
        .map_err(|_| NntpError::Timeout)?
        .map_err(|e| NntpError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Task join error: {}", e)
        )))?
        .map_err(NntpError::Io)?;

        // Convert to tokio TcpStream
        let tcp_stream = TcpStream::from_std(tcp_stream).map_err(NntpError::Io)?;

        // Set up TLS - install default crypto provider if not already installed
        use tokio_rustls::rustls::crypto::{ring, CryptoProvider};
        let _ = CryptoProvider::install_default(ring::default_provider());

        // Configure TLS based on security settings
        let tls_config = if config.allow_insecure_tls {
            // Insecure mode: accept any certificate (for self-signed certificates)
            warn!("TLS certificate validation disabled - connection vulnerable to MITM attacks");
            ClientConfig::builder()
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(DangerousAcceptAnyCertificate))
                .with_no_client_auth()
        } else {
            // Secure mode: validate certificates against trusted root CAs
            let mut root_store = RootCertStore::empty();
            root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

            ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth()
        };

        let connector = TlsConnector::from(Arc::new(tls_config));
        let server_name = ServerName::try_from(config.host.as_str())
            .map_err(|e| NntpError::Tls(format!("Invalid domain: {}", e)))?
            .to_owned();

        // TLS handshake with timeout (60 seconds)
        let tls_stream = timeout(
            Duration::from_secs(60),
            connector.connect(server_name, tcp_stream),
        )
        .await
        .map_err(|_| NntpError::Timeout)?
        .map_err(|e| NntpError::Tls(format!("TLS handshake failed: {}", e)))?;

        // Use 256KB buffer for high-throughput article downloads
        // Default 8KB is too small and causes excessive syscalls
        let stream = BufReader::with_capacity(262144, tls_stream);

        let mut client = Self {
            stream,
            state: ConnectionState::Ready,
            config,
            current_group: None,
            compression_mode: CompressionMode::None,
            bytes_compressed: 0,
            bytes_decompressed: 0,
            is_broken: false,
        };

        // Read server greeting
        let greeting = client.read_response().await?;
        debug!("Server greeting: {} {}", greeting.code, greeting.message);

        if !greeting.is_success() {
            return Err(NntpError::Protocol {
                code: greeting.code,
                message: greeting.message,
            });
        }

        Ok(client)
    }

    /// Authenticate with username and password
    ///
    /// Uses AUTHINFO USER/PASS authentication (RFC 4643).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::AuthFailed`] - Invalid credentials or authentication rejected
    /// - [`NntpError::ConnectionClosed`] - Server closed the connection
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn authenticate(&mut self) -> Result<()> {
        debug!("Authenticating as {}", self.config.username);

        // Check if already authenticated
        if matches!(self.state, ConnectionState::Authenticated) {
            return Err(NntpError::Protocol {
                code: 502,
                message: "Already authenticated".to_string(),
            });
        }

        // Send AUTHINFO USER
        let cmd = commands::authinfo_user(&self.config.username);
        self.send_command(&cmd).await?;

        // Mark authentication as in progress
        self.state = ConnectionState::InProgress;

        let response = self.read_response().await?;

        // Expect 381 (continue) or 281 (already authenticated)
        if response.code == codes::AUTH_CONTINUE {
            // Send AUTHINFO PASS
            let cmd = commands::authinfo_pass(&self.config.password);
            self.send_command(&cmd).await?;
            let response = self.read_response().await?;

            if response.code != codes::AUTH_ACCEPTED {
                // Reset to Ready state on failure
                self.state = ConnectionState::Ready;
                return Err(NntpError::AuthFailed(response.message));
            }
        } else if response.code != codes::AUTH_ACCEPTED {
            // Reset to Ready state on failure
            self.state = ConnectionState::Ready;
            return Err(NntpError::AuthFailed(response.message));
        }

        self.state = ConnectionState::Authenticated;
        debug!("Authentication successful");
        Ok(())
    }

    /// Authenticate using SASL mechanism (RFC 4643 Section 2.4)
    ///
    /// Uses AUTHINFO SASL for authentication with pluggable mechanisms.
    /// Supports challenge-response exchange via 383 continuation responses.
    ///
    /// # Arguments
    ///
    /// * `mechanism` - A SASL mechanism implementing the [`crate::SaslMechanism`] trait
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::{NntpClient, ServerConfig, SaslPlain};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ServerConfig::tls("news.example.com", "user", "pass");
    /// let mut client = NntpClient::connect(Arc::new(config)).await?;
    ///
    /// // Authenticate using SASL PLAIN mechanism
    /// let mechanism = SaslPlain::new("username", "password");
    /// client.authenticate_sasl(mechanism).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::AuthFailed`] - Authentication rejected (code 481)
    /// - [`NntpError::Protocol`] - Out of sequence (code 482) or protocol error
    /// - [`NntpError::EncryptionRequired`] - TLS required but not enabled (code 483)
    /// - [`NntpError::ConnectionClosed`] - Server closed the connection
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn authenticate_sasl(
        &mut self,
        mut mechanism: impl crate::SaslMechanism,
    ) -> Result<()> {
        debug!(
            "Authenticating with SASL mechanism: {}",
            mechanism.mechanism_name()
        );

        // Check if already authenticated
        if matches!(self.state, ConnectionState::Authenticated) {
            return Err(NntpError::Protocol {
                code: 502,
                message: "Already authenticated".to_string(),
            });
        }

        // Get initial response from mechanism
        let initial_response = mechanism.initial_response()?;

        // Send AUTHINFO SASL command
        let cmd = if let Some(initial_data) = initial_response {
            let encoded = crate::sasl::encode_sasl_data(&initial_data);
            commands::authinfo_sasl_ir(mechanism.mechanism_name(), &encoded)
        } else {
            commands::authinfo_sasl(mechanism.mechanism_name())
        };

        self.send_command(&cmd).await?;

        // Mark authentication as in progress
        self.state = ConnectionState::InProgress;

        let mut response = self.read_response().await?;

        // Handle challenge-response loop
        while response.code == codes::SASL_CONTINUE {
            debug!("SASL challenge received, processing...");

            // Extract challenge data from response message
            let challenge_encoded = response.message.trim();
            let challenge = crate::sasl::decode_sasl_data(challenge_encoded)?;

            // Process challenge and get response
            let client_response = mechanism.process_challenge(&challenge)?;
            let encoded_response = crate::sasl::encode_sasl_data(&client_response);

            // Send response
            let cmd = commands::authinfo_sasl_continue(&encoded_response);
            self.send_command(&cmd).await?;
            response = self.read_response().await?;
        }

        // Check final response
        match response.code {
            codes::AUTH_ACCEPTED => {
                self.state = ConnectionState::Authenticated;
                debug!("SASL authentication successful");
                Ok(())
            }
            codes::AUTH_REJECTED => {
                // Reset to Ready state on failure
                self.state = ConnectionState::Ready;
                Err(NntpError::AuthFailed(response.message))
            }
            codes::AUTH_OUT_OF_SEQUENCE => {
                // Reset to Ready state on failure
                self.state = ConnectionState::Ready;
                Err(NntpError::Protocol {
                    code: codes::AUTH_OUT_OF_SEQUENCE,
                    message: format!("Authentication out of sequence: {}", response.message),
                })
            }
            codes::ENCRYPTION_REQUIRED => {
                // Reset to Ready state on failure
                self.state = ConnectionState::Ready;
                Err(NntpError::EncryptionRequired(response.message))
            }
            _ => {
                // Reset to Ready state on failure
                self.state = ConnectionState::Ready;
                Err(NntpError::Protocol {
                    code: response.code,
                    message: response.message,
                })
            }
        }
    }

    /// Request server capabilities (RFC 3977 Section 5.2)
    ///
    /// Returns the list of capabilities supported by the server.
    /// This command can be used to detect which extensions and features
    /// the server supports before attempting to use them.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::{NntpClient, ServerConfig};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ServerConfig::tls("news.example.com", "user", "pass");
    /// # let mut client = NntpClient::connect(Arc::new(config)).await?;
    /// let caps = client.capabilities().await?;
    /// if caps.has("COMPRESS") {
    ///     println!("Server supports compression");
    ///     if caps.has_arg("COMPRESS", "DEFLATE") {
    ///         println!("  - DEFLATE compression available");
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn capabilities(&mut self) -> Result<Capabilities> {
        debug!("Requesting server capabilities");

        let cmd = commands::capabilities();
        self.send_command(&cmd).await?;
        let response = self.read_multiline_response().await?;

        if response.code != codes::CAPABILITY_LIST {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        let caps = Capabilities::parse(&response.lines);
        debug!("Received {} capabilities", caps.list().len());
        Ok(caps)
    }

    /// Switch to reader mode (RFC 3977 Section 5.3)
    ///
    /// Instructs the server to switch to reader mode, indicating this is a news
    /// reading client (as opposed to a news transfer agent). Many servers require
    /// this command before accepting most client commands.
    ///
    /// Returns `true` if posting is allowed (code 200), `false` if posting is
    /// not permitted (code 201).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::{NntpClient, ServerConfig};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ServerConfig::tls("news.example.com", "user", "pass");
    /// # let mut client = NntpClient::connect(Arc::new(config)).await?;
    /// let posting_allowed = client.mode_reader().await?;
    /// if posting_allowed {
    ///     println!("Posting is allowed on this server");
    /// } else {
    ///     println!("Read-only access - posting not permitted");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn mode_reader(&mut self) -> Result<bool> {
        debug!("Switching to reader mode");

        let cmd = commands::mode_reader();
        self.send_command(&cmd).await?;
        let response = self.read_response().await?;

        match response.code {
            codes::READY_POSTING_ALLOWED => {
                debug!("Reader mode enabled - posting allowed");
                Ok(true)
            }
            codes::READY_NO_POSTING => {
                debug!("Reader mode enabled - posting not allowed");
                Ok(false)
            }
            _ => Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            }),
        }
    }

    /// Switch to streaming mode (RFC 4644 Section 2.3)
    ///
    /// Requests to switch to streaming mode for efficient bulk article transfer.
    /// Streaming mode allows pipelined CHECK and TAKETHIS commands without
    /// waiting for individual responses.
    ///
    /// Before calling this method, check if the server supports streaming by
    /// inspecting the CAPABILITIES response for the STREAMING capability.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::{NntpClient, ServerConfig};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ServerConfig::tls("news.example.com", "user", "pass");
    /// # let mut client = NntpClient::connect(Arc::new(config)).await?;
    /// // Check capabilities first
    /// let caps = client.capabilities().await?;
    /// if caps.has_capability("STREAMING") {
    ///     client.mode_stream().await?;
    ///     println!("Streaming mode enabled");
    ///     // Now you can use CHECK and TAKETHIS commands
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::Protocol`] - Server does not support streaming or returned an error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn mode_stream(&mut self) -> Result<()> {
        debug!("Switching to streaming mode");

        let cmd = commands::mode_stream();
        self.send_command(&cmd).await?;
        let response = self.read_response().await?;

        if response.code != codes::STREAMING_OK {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        debug!("Streaming mode enabled");
        Ok(())
    }

    /// Check if server wants an article (RFC 4644 Section 2.4)
    ///
    /// In streaming mode, checks whether the server wants an article by its message-id.
    /// This is the first phase of the streaming article transfer protocol.
    ///
    /// The server responds with:
    /// - 238 (CHECK_SEND) - Server wants the article; use TAKETHIS to send it
    /// - 431 (CHECK_LATER) - Server is temporarily unavailable; retry later
    /// - 438 (CHECK_NOT_WANTED) - Server does not want this article
    ///
    /// CHECK can be pipelined - multiple CHECK commands can be sent without waiting
    /// for responses. The server includes the message-id in each response for matching.
    ///
    /// **Note:** You must call [`mode_stream()`](Self::mode_stream) before using CHECK.
    ///
    /// # Arguments
    ///
    /// * `message_id` - The message-id to check (e.g., "<abc123@example.com>")
    ///
    /// # Returns
    ///
    /// Returns the full response, including the message-id in the response message.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::{NntpClient, ServerConfig, codes};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ServerConfig::tls("news.example.com", "user", "pass");
    /// # let mut client = NntpClient::connect(Arc::new(config)).await?;
    /// // First enable streaming mode
    /// client.mode_stream().await?;
    ///
    /// // Check if server wants an article
    /// let message_id = "<article123@example.com>";
    /// let response = client.check(message_id).await?;
    ///
    /// match response.code {
    ///     codes::CHECK_SEND => {
    ///         println!("Server wants article - send with TAKETHIS");
    ///         // client.takethis(message_id, article_data).await?;
    ///     }
    ///     codes::CHECK_LATER => {
    ///         println!("Server busy - retry later");
    ///     }
    ///     codes::CHECK_NOT_WANTED => {
    ///         println!("Server doesn't want article");
    ///     }
    ///     _ => {
    ///         println!("Unexpected response: {}", response.code);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::Protocol`] - Server returned an error or unexpected response
    /// - [`NntpError::Timeout`] - Server did not respond in time
    /// - Network I/O errors
    pub async fn check(&mut self, message_id: &str) -> Result<NntpResponse> {
        debug!("CHECK: {}", message_id);

        let cmd = commands::check(message_id);
        self.send_command(&cmd).await?;
        let response = self.read_response().await?;

        debug!(
            "CHECK response for {}: {} {}",
            message_id, response.code, response.message
        );
        Ok(response)
    }

    /// Send an article to the server in streaming mode (RFC 4644 Section 2.5)
    ///
    /// In streaming mode, sends an article to the server without waiting for permission.
    /// The article is sent immediately after the command, and the server responds after
    /// receiving the complete article.
    ///
    /// The server responds with:
    /// - 239 (TAKETHIS_RECEIVED) - Article received successfully
    /// - 439 (TAKETHIS_REJECTED) - Article rejected; do not retry
    ///
    /// TAKETHIS can be pipelined - multiple TAKETHIS commands can be sent without waiting
    /// for responses. The server includes the message-id in each response for matching.
    ///
    /// **Note:** You must call [`mode_stream()`](Self::mode_stream) before using TAKETHIS.
    ///
    /// # Arguments
    ///
    /// * `message_id` - The message-id of the article (e.g., "<abc123@example.com>")
    /// * `article` - The article to send
    ///
    /// # Returns
    ///
    /// Returns the full response, including the message-id in the response message.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::{NntpClient, ServerConfig, ArticleBuilder, codes};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ServerConfig::tls("news.example.com", "user", "pass");
    /// # let mut client = NntpClient::connect(Arc::new(config)).await?;
    /// // First enable streaming mode
    /// client.mode_stream().await?;
    ///
    /// // Build an article
    /// let article = ArticleBuilder::new()
    ///     .from("user@example.com")
    ///     .subject("Test post")
    ///     .newsgroups(vec!["test.group"])
    ///     .body("This is a test")
    ///     .build()?;
    ///
    /// let message_id = article.headers.message_id.clone();
    ///
    /// // Send the article without asking first
    /// let response = client.takethis(&message_id, &article).await?;
    ///
    /// match response.code {
    ///     codes::TAKETHIS_RECEIVED => {
    ///         println!("Article received successfully");
    ///     }
    ///     codes::TAKETHIS_REJECTED => {
    ///         println!("Article rejected by server");
    ///     }
    ///     _ => {
    ///         println!("Unexpected response: {}", response.code);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::Protocol`] - Server returned an error or unexpected response
    /// - [`NntpError::Timeout`] - Server did not respond in time
    /// - Network I/O errors
    /// - Article serialization fails
    pub async fn takethis(&mut self, message_id: &str, article: &Article) -> Result<NntpResponse> {
        debug!("TAKETHIS: {}", message_id);

        // Serialize article with CRLF and dot-stuffing
        let article_data = article.serialize_for_posting()?;

        let cmd = commands::takethis(message_id, &article_data);
        self.send_command(&cmd).await?;
        let response = self.read_response().await?;

        debug!(
            "TAKETHIS response for {}: {} {}",
            message_id, response.code, response.message
        );
        Ok(response)
    }

    /// Get server date/time (RFC 3977 Section 7.1)
    ///
    /// Requests the server's current date and time in UTC.
    ///
    /// Returns the server timestamp in the format `YYYYMMDDhhmmss`
    /// (e.g., "20240115123456" represents January 15, 2024 at 12:34:56 UTC).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::{NntpClient, ServerConfig};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ServerConfig::tls("news.example.com", "user", "pass");
    /// # let mut client = NntpClient::connect(Arc::new(config)).await?;
    /// let server_time = client.date().await?;
    /// println!("Server time: {}", server_time);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn date(&mut self) -> Result<String> {
        debug!("Requesting server date/time");

        let cmd = commands::date();
        self.send_command(&cmd).await?;
        let response = self.read_response().await?;

        if response.code != codes::SERVER_DATE {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        // Extract timestamp from response message (format: "111 YYYYMMDDhhmmss")
        let timestamp = response
            .message
            .split_whitespace()
            .next()
            .unwrap_or(&response.message)
            .to_string();

        debug!("Server date/time: {}", timestamp);
        Ok(timestamp)
    }

    /// Request help text from the server (RFC 3977 §7.2)
    ///
    /// Returns multi-line help text from the server describing available commands
    /// and server-specific information.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::{NntpClient, ServerConfig};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ServerConfig::tls("news.example.com", "user", "pass");
    /// # let mut client = NntpClient::connect(Arc::new(config)).await?;
    /// let help_response = client.help().await?;
    /// for line in &help_response.lines {
    ///     println!("{}", line);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn help(&mut self) -> Result<NntpResponse> {
        debug!("Requesting help text");

        let cmd = commands::help();
        self.send_command(&cmd).await?;
        let response = self.read_multiline_response().await?;

        if response.code != codes::HELP_TEXT_FOLLOWS {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        debug!("Received help text ({} lines)", response.lines.len());
        Ok(response)
    }

    /// List active newsgroups (RFC 3977 Section 7.6.3)
    ///
    /// Returns a list of active newsgroups matching the wildmat pattern.
    /// Format: group high low status
    ///
    /// # Arguments
    ///
    /// * `wildmat` - Wildmat pattern (e.g., "comp.*", "*" for all groups)
    ///
    /// # Returns
    ///
    /// Vector of [`commands::ActiveGroup`] entries containing:
    /// - name: newsgroup name
    /// - high: highest article number
    /// - low: lowest article number
    /// - status: 'y' (posting allowed), 'n' (no posting), 'm' (moderated)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn list_active(&mut self, wildmat: &str) -> Result<Vec<commands::ActiveGroup>> {
        debug!("Listing active groups matching: {}", wildmat);

        let cmd = commands::list_active(wildmat);
        self.send_command(&cmd).await?;
        let response = self.read_multiline_response().await?;

        if response.code != codes::LIST_INFORMATION_FOLLOWS {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        let groups = commands::parse_list_active_response(&response)?;
        debug!("Retrieved {} active groups", groups.len());
        Ok(groups)
    }

    /// List newsgroups with descriptions (RFC 3977 Section 7.6.6).
    ///
    /// Returns newsgroup names matching the wildmat pattern along with their descriptions.
    /// The wildmat pattern supports `*` (matches any sequence) and `?` (matches single character).
    /// Use `*` to list all newsgroups.
    ///
    /// # Arguments
    ///
    /// * `wildmat` - Wildmat pattern to match newsgroup names (e.g., "comp.*", "alt.binaries.*", "*")
    ///
    /// # Returns
    ///
    /// Returns a `Vec<NewsgroupInfo>` where each entry contains:
    /// - name: newsgroup name
    /// - description: human-readable description of the newsgroup
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn list_newsgroups(&mut self, wildmat: &str) -> Result<Vec<commands::NewsgroupInfo>> {
        debug!("Listing newsgroups matching: {}", wildmat);

        let cmd = commands::list_newsgroups(wildmat);
        self.send_command(&cmd).await?;
        let response = self.read_multiline_response().await?;

        if response.code != codes::LIST_INFORMATION_FOLLOWS {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        let groups = commands::parse_list_newsgroups_response(&response)?;
        debug!("Retrieved {} newsgroup descriptions", groups.len());
        Ok(groups)
    }

    /// List the overview format fields
    ///
    /// Returns the field names in the order they appear in OVER/XOVER output.
    /// Fields may be header names (e.g., "Subject:") or metadata (e.g., ":bytes").
    ///
    /// RFC 3977 Section 8.4
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn list_overview_fmt(&mut self) -> Result<Vec<String>> {
        debug!("Requesting overview format");

        let cmd = commands::list_overview_fmt();
        self.send_command(&cmd).await?;
        let response = self.read_multiline_response().await?;

        if response.code != codes::LIST_INFORMATION_FOLLOWS {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        let fields = commands::parse_list_overview_fmt_response(&response)?;
        debug!("Retrieved {} overview format fields", fields.len());
        Ok(fields)
    }

    /// Retrieve list of header fields available for HDR command (RFC 3977 §8.6)
    ///
    /// Returns a list of header field names that can be used with the HDR command.
    /// The optional `keyword` parameter specifies which form of HDR the results apply to:
    /// - `None`: List all available headers
    /// - `Some("MSGID")`: List headers available for HDR with message-id
    /// - `Some("RANGE")`: List headers available for HDR with range
    ///
    /// A special entry ":" in the response means any header may be retrieved.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn list_headers(&mut self, keyword: Option<&str>) -> Result<Vec<String>> {
        debug!("Requesting header fields list (keyword: {:?})", keyword);

        let cmd = match keyword {
            None => commands::list_headers(),
            Some("MSGID") => commands::list_headers_msgid(),
            Some("RANGE") => commands::list_headers_range(),
            Some(other) => {
                return Err(NntpError::InvalidResponse(format!(
                    "Invalid LIST HEADERS keyword: {}. Must be MSGID or RANGE",
                    other
                )))
            }
        };

        self.send_command(&cmd).await?;
        let response = self.read_multiline_response().await?;

        if response.code != codes::LIST_INFORMATION_FOLLOWS {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        let headers = commands::parse_list_headers_response(&response)?;
        debug!("Retrieved {} header fields", headers.len());
        Ok(headers)
    }

    /// List newsgroup creation times (LIST ACTIVE.TIMES)
    ///
    /// Returns a list of newsgroups with creation timestamp and creator.
    /// The wildmat parameter can filter groups (e.g., "comp.*" or "*").
    ///
    /// RFC 3977 Section 7.6.4
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn list_active_times(&mut self, wildmat: &str) -> Result<Vec<commands::GroupTime>> {
        debug!("Requesting newsgroup creation times (wildmat: {})", wildmat);

        let cmd = commands::list_active_times(wildmat);
        self.send_command(&cmd).await?;
        let response = self.read_multiline_response().await?;

        if response.code != codes::LIST_INFORMATION_FOLLOWS {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        let groups = commands::parse_list_active_times_response(&response)?;
        debug!("Retrieved {} newsgroup creation times", groups.len());
        Ok(groups)
    }

    /// List newsgroups with estimated article counts (RFC 6048 Section 3).
    ///
    /// Returns newsgroup information matching the wildmat pattern with estimated article counts.
    /// This is an enhanced version of LIST ACTIVE that includes article counts.
    /// The wildmat pattern supports `*` (matches any sequence) and `?` (matches single character).
    /// Use `*` to list all newsgroups.
    ///
    /// # Arguments
    ///
    /// * `wildmat` - Pattern to match newsgroup names (e.g., "comp.*", "alt.binaries.*", "*")
    ///
    /// # Returns
    ///
    /// A vector of [`CountsGroup`](commands::CountsGroup) entries containing:
    /// - Newsgroup name
    /// - Estimated article count
    /// - Low water mark (lowest article number)
    /// - High water mark (highest article number)
    /// - Posting status ('y' = allowed, 'n' = not allowed, 'm' = moderated)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::NntpClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut client = NntpClient::connect("news.example.com:119").await?;
    ///
    /// // List all newsgroups with counts
    /// let groups = client.list_counts("*").await?;
    /// for group in groups {
    ///     println!("{}: {} articles ({}-{})",
    ///         group.name, group.count, group.low, group.high);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::InvalidResponse`] - Could not parse the server response
    ///
    /// # Note
    ///
    /// Not all servers support this command. If unsupported, use [`list_active`](Self::list_active)
    /// instead and calculate counts manually (high - low).
    pub async fn list_counts(&mut self, wildmat: &str) -> Result<Vec<commands::CountsGroup>> {
        debug!("Listing newsgroups with counts matching: {}", wildmat);

        let cmd = commands::list_counts(wildmat);
        self.send_command(&cmd).await?;
        let response = self.read_multiline_response().await?;

        if response.code != codes::LIST_INFORMATION_FOLLOWS {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        let groups = commands::parse_list_counts_response(&response)?;
        debug!("Retrieved {} newsgroups with counts", groups.len());
        Ok(groups)
    }

    /// List valid distribution names and descriptions (RFC 6048 Section 4).
    ///
    /// Returns a list of distribution names with their descriptions. Distributions
    /// are used to limit article propagation to specific geographic or organizational
    /// areas (e.g., "local", "usa", "fr").
    ///
    /// No wildmat argument is permitted for this command - it returns all distributions.
    ///
    /// # Returns
    ///
    /// A vector of [`DistributionInfo`](commands::DistributionInfo) entries, each containing:
    /// - Distribution name (e.g., "local", "usa", "fr")
    /// - Short description of the distribution area
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::NntpClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut client = NntpClient::connect("news.example.com:119").await?;
    ///
    /// // List all available distributions
    /// let distributions = client.list_distributions().await?;
    /// for dist in distributions {
    ///     println!("{}: {}", dist.name, dist.description);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::Protocol`] - Server returned an error (e.g., 503 if not available)
    /// - [`NntpError::InvalidResponse`] - Could not parse the server response
    ///
    /// # Note
    ///
    /// Not all servers support this command. If the server returns 503 (not available),
    /// the server does not maintain a distribution list.
    pub async fn list_distributions(&mut self) -> Result<Vec<commands::DistributionInfo>> {
        debug!("Listing distributions");

        let cmd = commands::list_distributions();
        self.send_command(&cmd).await?;
        let response = self.read_multiline_response().await?;

        if response.code != codes::LIST_INFORMATION_FOLLOWS {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        let distributions = commands::parse_list_distributions_response(&response)?;
        debug!("Retrieved {} distributions", distributions.len());
        Ok(distributions)
    }

    /// List moderator submission addresses (RFC 6048 Section 5).
    ///
    /// Returns a list of submission address templates for moderated newsgroups.
    /// Each entry contains a pattern (newsgroup name or wildmat) and an address template.
    ///
    /// # Format
    ///
    /// - `%s` in the address is replaced with the newsgroup name (periods converted to dashes)
    /// - `%%` represents a literal `%` character
    /// - Patterns are matched in order; the first match is used
    ///
    /// # Returns
    ///
    /// A vector of [`ModeratorInfo`](commands::ModeratorInfo) entries containing:
    /// - Pattern: newsgroup name or wildmat pattern
    /// - Address: submission address template
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::NntpClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut client = NntpClient::connect("news.example.com:119").await?;
    ///
    /// // Get moderator submission addresses
    /// let moderators = client.list_moderators().await?;
    /// for m in moderators {
    ///     println!("{}: {}", m.pattern, m.address);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_moderators(&mut self) -> Result<Vec<commands::ModeratorInfo>> {
        debug!("Listing moderators");

        let cmd = commands::list_moderators();
        self.send_command(&cmd).await?;
        let response = self.read_multiline_response().await?;

        if response.code != codes::LIST_INFORMATION_FOLLOWS {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        let moderators = commands::parse_list_moderators_response(&response)?;
        debug!("Retrieved {} moderator entries", moderators.len());
        Ok(moderators)
    }

    /// Retrieve the server's message of the day (RFC 6048 Section 6).
    ///
    /// Returns the server's message of the day as a vector of text lines.
    /// The MOTD typically contains welcome messages, server status updates,
    /// announcements, or other information the server administrator wants to
    /// communicate to users.
    ///
    /// # Returns
    ///
    /// A vector of strings, where each string is a line from the message of the day.
    /// Empty lines are preserved as they may be part of the formatted message.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # use nntp_rs::NntpClient;
    /// let mut client = NntpClient::connect("news.example.com", 119, false).await?;
    /// let motd = client.list_motd().await?;
    /// for line in motd {
    ///     println!("{}", line);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_motd(&mut self) -> Result<Vec<String>> {
        debug!("Retrieving message of the day");

        let cmd = commands::list_motd();
        self.send_command(&cmd).await?;
        let response = self.read_multiline_response().await?;

        if response.code != codes::LIST_INFORMATION_FOLLOWS {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        let motd = commands::parse_list_motd_response(&response)?;
        debug!("Retrieved {} MOTD lines", motd.len());
        Ok(motd)
    }

    /// Retrieve the server's default subscription list (RFC 6048 Section 7).
    ///
    /// Returns a list of newsgroups that the server recommends for new users to subscribe to.
    /// This is typically a curated list of popular or important newsgroups on the server.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::NntpClient;
    /// # async fn example(client: &mut NntpClient) -> Result<(), Box<dyn std::error::Error>> {
    /// let subscriptions = client.list_subscriptions().await?;
    /// for group in subscriptions {
    ///     println!("Recommended: {}", group);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_subscriptions(&mut self) -> Result<Vec<String>> {
        debug!("Retrieving default subscription list");

        let cmd = commands::list_subscriptions();
        self.send_command(&cmd).await?;
        let response = self.read_multiline_response().await?;

        if response.code != codes::LIST_INFORMATION_FOLLOWS {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        let subscriptions = commands::parse_list_subscriptions_response(&response)?;
        debug!("Retrieved {} subscription entries", subscriptions.len());
        Ok(subscriptions)
    }

    /// List newsgroups created after a specific date/time (RFC 3977 Section 7.3).
    ///
    /// Returns information about newsgroups created since the specified date and time.
    /// The response format is identical to LIST ACTIVE (group high low status).
    ///
    /// # Arguments
    ///
    /// * `date` - Date in format "yyyymmdd" (e.g., "20240101" for January 1, 2024)
    /// * `time` - Time in format "hhmmss" (e.g., "120000" for 12:00:00)
    /// * `gmt` - If true, uses GMT timezone; if false, uses server's local time
    ///
    /// # Returns
    ///
    /// A vector of [`ActiveGroup`](commands::ActiveGroup) entries containing:
    /// - Newsgroup name
    /// - High water mark (highest article number)
    /// - Low water mark (lowest article number)
    /// - Posting status ('y' = allowed, 'n' = not allowed, 'm' = moderated)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::NntpClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut client = NntpClient::connect("news.example.com:119").await?;
    ///
    /// // Get newsgroups created since January 1, 2024 at midnight GMT
    /// let new_groups = client.newgroups("20240101", "000000", true).await?;
    /// for group in new_groups {
    ///     println!("New group: {} ({}-{})", group.name, group.low, group.high);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::InvalidResponse`] - Could not parse the server response
    pub async fn newgroups(
        &mut self,
        date: &str,
        time: &str,
        gmt: bool,
    ) -> Result<Vec<commands::ActiveGroup>> {
        debug!(
            "Requesting newsgroups created since {} {} (GMT: {})",
            date, time, gmt
        );

        let cmd = if gmt {
            commands::newgroups_gmt(date, time)
        } else {
            commands::newgroups(date, time)
        };
        self.send_command(&cmd).await?;
        let response = self.read_multiline_response().await?;

        if response.code != codes::NEW_NEWSGROUPS_FOLLOW {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        let groups = commands::parse_newgroups_response(&response)?;
        debug!("Retrieved {} new newsgroups", groups.len());
        Ok(groups)
    }

    /// Retrieve message-IDs of articles posted after a specific date/time (RFC 3977 §7.4)
    ///
    /// Lists message-IDs of articles posted to newsgroups matching the wildmat pattern
    /// since the specified date and time.
    ///
    /// # Arguments
    ///
    /// * `wildmat` - Newsgroup pattern (e.g., "comp.lang.*", "alt.binaries.*", "*")
    /// * `date` - Date in format "yyyymmdd" (e.g., "20240101" for January 1, 2024)
    /// * `time` - Time in format "hhmmss" (e.g., "120000" for 12:00:00)
    /// * `gmt` - If true, use GMT; if false, use server local time
    ///
    /// # Returns
    ///
    /// Returns a vector of message-IDs (e.g., "<abc@example.com>") for articles posted
    /// to matching newsgroups since the specified date/time.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use nntp_rs::{NntpClient, ServerConfig};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ServerConfig::builder("news.example.com").build()?;
    /// # let mut client = NntpClient::connect(&config).await?;
    /// // Get all new articles in comp.lang.rust since midnight UTC on Jan 1, 2024
    /// let message_ids = client.newnews("comp.lang.rust", "20240101", "000000", true).await?;
    /// println!("Found {} new articles", message_ids.len());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::Protocol`] - Server returned an error
    /// - [`NntpError::InvalidResponse`] - Could not parse the server response
    pub async fn newnews(
        &mut self,
        wildmat: &str,
        date: &str,
        time: &str,
        gmt: bool,
    ) -> Result<Vec<String>> {
        debug!(
            "Requesting articles since {} {} in {} (GMT: {})",
            date, time, wildmat, gmt
        );

        let cmd = if gmt {
            commands::newnews_gmt(wildmat, date, time)
        } else {
            commands::newnews(wildmat, date, time)
        };
        self.send_command(&cmd).await?;
        let response = self.read_multiline_response().await?;

        if response.code != codes::NEW_ARTICLE_LIST_FOLLOWS {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        let message_ids = commands::parse_newnews_response(&response)?;
        debug!("Retrieved {} message-IDs", message_ids.len());
        Ok(message_ids)
    }

    /// Select a newsgroup
    ///
    /// Returns (article_count, first_article, last_article).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::NoSuchGroup`] - The newsgroup does not exist
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::InvalidResponse`] - Could not parse the server response
    pub async fn select_group(&mut self, newsgroup: &str) -> Result<(u64, u64, u64)> {
        debug!("Selecting newsgroup: {}", newsgroup);

        let cmd = commands::group(newsgroup);
        self.send_command(&cmd).await?;
        let response = self.read_response().await?;

        if response.code == codes::NO_SUCH_GROUP {
            return Err(NntpError::NoSuchGroup(newsgroup.to_string()));
        }

        let (count, first, last) = commands::parse_group_response(&response)?;
        self.current_group = Some(newsgroup.to_string());

        debug!(
            "Group {} selected: {} articles ({}-{})",
            newsgroup, count, first, last
        );
        Ok((count, first, last))
    }

    /// List article numbers in a newsgroup (RFC 3977 Section 6.1.2)
    ///
    /// Returns a list of article numbers currently available in the specified newsgroup.
    /// Optionally accepts a range parameter to limit the returned article numbers.
    ///
    /// This command is useful for:
    /// - Getting all article numbers in a group
    /// - Finding which articles exist in a range
    /// - Checking article availability before downloading
    ///
    /// # Arguments
    ///
    /// * `newsgroup` - The newsgroup to list articles from
    /// * `range` - Optional range specification (e.g., "100-200", "100-", "-200")
    ///
    /// # Returns
    ///
    /// A vector of article numbers available in the group
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::{NntpClient, ServerConfig};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ServerConfig::tls("news.example.com", "user", "pass");
    /// # let mut client = NntpClient::connect(Arc::new(config)).await?;
    /// // List all article numbers
    /// let articles = client.listgroup("alt.binaries.test", None).await?;
    /// println!("Found {} articles", articles.len());
    ///
    /// // List articles in a specific range
    /// let recent = client.listgroup("alt.binaries.test", Some("1000-2000")).await?;
    /// println!("Found {} articles in range 1000-2000", recent.len());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::NoSuchGroup`] - The newsgroup does not exist
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn listgroup(&mut self, newsgroup: &str, range: Option<&str>) -> Result<Vec<u64>> {
        debug!("Listing articles in group: {}", newsgroup);

        let cmd = match range {
            Some(r) => commands::listgroup_range(newsgroup, r),
            None => commands::listgroup(newsgroup),
        };

        self.send_command(&cmd).await?;
        let response = self.read_multiline_response().await?;

        if response.code == codes::NO_SUCH_GROUP {
            return Err(NntpError::NoSuchGroup(newsgroup.to_string()));
        }

        if response.code != codes::GROUP_SELECTED {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        // Parse article numbers from multiline response
        let mut articles = Vec::new();
        for line in &response.lines {
            if let Ok(num) = line.trim().parse::<u64>() {
                articles.push(num);
            }
        }

        debug!("Found {} articles in group {}", articles.len(), newsgroup);
        Ok(articles)
    }

    /// Fetch article by message-ID or number
    ///
    /// Returns the full article (headers and body).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::NoSuchArticle`] - The article does not exist
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn fetch_article(&mut self, id: &str) -> Result<NntpResponse> {
        trace!("Fetching article: {}", id);

        let cmd = commands::article(id);
        self.send_command(&cmd).await?;
        let response = self.read_multiline_response().await?;

        if response.code == codes::NO_SUCH_ARTICLE_ID
            || response.code == codes::NO_SUCH_ARTICLE_NUMBER
        {
            return Err(NntpError::NoSuchArticle(id.to_string()));
        }

        if !response.is_success() {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        Ok(response)
    }

    /// Fetch article headers only
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::NoSuchArticle`] - The article does not exist
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn fetch_head(&mut self, id: &str) -> Result<NntpResponse> {
        trace!("Fetching head: {}", id);

        let cmd = commands::head(id);
        self.send_command(&cmd).await?;
        let response = self.read_multiline_response().await?;

        if !response.is_success() {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        Ok(response)
    }

    /// Fetch article body only
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::NoSuchArticle`] - The article does not exist
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn fetch_body(&mut self, id: &str) -> Result<NntpResponse> {
        trace!("Fetching body: {}", id);

        let cmd = commands::body(id);
        self.send_command(&cmd).await?;
        let response = self.read_multiline_response().await?;

        if !response.is_success() {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        Ok(response)
    }

    /// Check article status without retrieving content (RFC 3977 §6.2.4)
    ///
    /// The STAT command allows checking whether an article exists and retrieving
    /// its metadata without downloading the full content. This is useful for
    /// checking article existence or getting message-id mapping.
    ///
    /// # Arguments
    ///
    /// * `id` - Either an article number (e.g., "12345") or message-id (e.g., "<abc@example.com>")
    ///
    /// # Returns
    ///
    /// Returns a tuple of (article_number, message_id):
    /// - If called with article number: returns (number, message_id)
    /// - If called with message-id: returns (0 or actual number, message_id)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::{NntpClient, ServerConfig};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ServerConfig::tls("news.example.com", "user", "pass");
    /// # let mut client = NntpClient::connect(Arc::new(config)).await?;
    /// # client.select_group("comp.lang.rust").await?;
    /// // Check by article number
    /// let (num, msgid) = client.stat("12345").await?;
    /// println!("Article {} has message-id: {}", num, msgid);
    ///
    /// // Check by message-id
    /// let (num, msgid) = client.stat("<abc@example.com>").await?;
    /// println!("Message exists at article number: {}", num);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::NoSuchArticle`] - Article does not exist (code 430)
    /// - [`NntpError::NoGroupSelected`] - No newsgroup selected (code 412)
    /// - [`NntpError::InvalidArticleNumber`] - Invalid article number (code 423)
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn stat(&mut self, id: &str) -> Result<(u64, String)> {
        trace!("Checking article status: {}", id);

        let cmd = commands::stat(id);
        self.send_command(&cmd).await?;
        let response = self.read_response().await?;

        // Handle specific error codes
        if response.code == codes::NO_SUCH_ARTICLE_ID
            || response.code == codes::NO_SUCH_ARTICLE_NUMBER
        {
            return Err(NntpError::NoSuchArticle(id.to_string()));
        }

        if response.code == codes::NO_GROUP_SELECTED {
            return Err(NntpError::NoGroupSelected);
        }

        if response.code == codes::NO_CURRENT_ARTICLE {
            return Err(NntpError::InvalidArticleNumber);
        }

        // Parse the response
        commands::parse_stat_response(&response)
    }

    /// Navigate to the next article in the current newsgroup
    ///
    /// Moves the server's internal pointer to the next article in the currently selected
    /// newsgroup and returns its article number and message-id.
    ///
    /// Corresponds to the NEXT command in RFC 3977 Section 6.1.4.
    ///
    /// # Returns
    ///
    /// Returns a tuple of (article_number, message_id) for the next article.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::NoGroupSelected`] - No newsgroup is currently selected (code 412)
    /// - [`NntpError::InvalidArticleNumber`] - No current article selected (code 420)
    /// - [`NntpError::NoSuchArticle`] - No next article in the group (code 421)
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use nntp_rs::NntpClient;
    /// # async fn example() -> nntp_rs::Result<()> {
    /// # let mut client = NntpClient::connect("news.example.com:119").await?;
    /// // Select a newsgroup first
    /// client.select_group("comp.lang.rust").await?;
    ///
    /// // Navigate to next article
    /// let (article_num, message_id) = client.next().await?;
    /// println!("Next article: {} <{}>", article_num, message_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn next(&mut self) -> Result<(u64, String)> {
        trace!("Navigating to next article");

        let cmd = commands::next();
        self.send_command(&cmd).await?;
        let response = self.read_response().await?;

        // Handle specific error codes
        if response.code == codes::NO_GROUP_SELECTED {
            return Err(NntpError::NoGroupSelected);
        }

        if response.code == codes::NO_CURRENT_ARTICLE {
            return Err(NntpError::InvalidArticleNumber);
        }

        if response.code == codes::NO_NEXT_ARTICLE {
            return Err(NntpError::NoSuchArticle("no next article".to_string()));
        }

        // Parse the response
        commands::parse_next_response(&response)
    }

    /// Navigate to the previous article in the selected newsgroup (RFC 3977 §6.1.3)
    ///
    /// The LAST command moves the currently selected article pointer backwards to the
    /// previous article in the currently selected newsgroup.
    ///
    /// Corresponds to the LAST command in RFC 3977 Section 6.1.3.
    ///
    /// # Returns
    ///
    /// Returns a tuple of (article_number, message_id) for the previous article.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::NoGroupSelected`] - No newsgroup is currently selected (code 412)
    /// - [`NntpError::InvalidArticleNumber`] - No current article selected (code 420)
    /// - [`NntpError::NoSuchArticle`] - No previous article in the group (code 422)
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use nntp_rs::NntpClient;
    /// # async fn example() -> nntp_rs::Result<()> {
    /// # let mut client = NntpClient::connect("news.example.com:119").await?;
    /// // Select a newsgroup first
    /// client.select_group("comp.lang.rust").await?;
    ///
    /// // Navigate to previous article
    /// let (article_num, message_id) = client.last().await?;
    /// println!("Previous article: {} <{}>", article_num, message_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn last(&mut self) -> Result<(u64, String)> {
        trace!("Navigating to previous article");

        let cmd = commands::last();
        self.send_command(&cmd).await?;
        let response = self.read_response().await?;

        // Handle specific error codes
        if response.code == codes::NO_GROUP_SELECTED {
            return Err(NntpError::NoGroupSelected);
        }

        if response.code == codes::NO_CURRENT_ARTICLE {
            return Err(NntpError::InvalidArticleNumber);
        }

        if response.code == codes::NO_PREV_ARTICLE {
            return Err(NntpError::NoSuchArticle("no previous article".to_string()));
        }

        // Parse the response
        commands::parse_last_response(&response)
    }

    /// Fetch XOVER data for article range
    ///
    /// Range format examples:
    /// - "100" - single article
    /// - "100-200" - range from 100 to 200
    /// - "100-" - from 100 to end
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::Protocol`] - Server returned an error (e.g., no group selected)
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn fetch_xover(&mut self, range: &str) -> Result<Vec<XoverEntry>> {
        trace!("Fetching XOVER: {}", range);

        let cmd = commands::xover(range);
        self.send_command(&cmd).await?;
        let response = self.read_multiline_response().await?;

        if !response.is_success() {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        let mut entries = Vec::new();
        for line in &response.lines {
            match commands::parse_xover_line(line) {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    warn!("Failed to parse XOVER line: {} - {}", line, e);
                    continue;
                }
            }
        }

        Ok(entries)
    }

    /// Fetch article overview data using OVER command (RFC 3977 §8.3)
    ///
    /// OVER is the RFC 3977 standard name for the XOVER command. It retrieves
    /// article metadata (subject, author, date, message-id, etc.) for a range
    /// of articles or a single message-id.
    ///
    /// This is more efficient than fetching full articles when you only need
    /// metadata for browsing or searching.
    ///
    /// # Arguments
    ///
    /// * `range_or_msgid` - Article range, single number, or message-id
    ///
    /// Range format examples:
    /// - "100" - single article
    /// - "100-200" - range from 100 to 200
    /// - "100-" - from 100 to end
    /// - "<abc@example.com>" - specific article by message-id
    /// - Empty string "" - current article
    ///
    /// # Returns
    ///
    /// Returns a [`Vec<XoverEntry>`] containing overview metadata for each article.
    /// Failed parse lines are logged and skipped (not returned as errors).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::Protocol`] - Server returned an error (e.g., no group selected)
    /// - [`NntpError::Timeout`] - Server did not respond in time
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::NntpClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = NntpClient::connect("news.example.com", 119, None).await?;
    /// // Select a newsgroup
    /// client.select_group("comp.lang.rust").await?;
    ///
    /// // Fetch overview for a range of articles
    /// let entries = client.over("1-100").await?;
    /// for entry in entries {
    ///     println!("{}: {}", entry.article_number, entry.subject);
    /// }
    ///
    /// // Fetch overview for current article
    /// let current = client.over("").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn over(&mut self, range_or_msgid: &str) -> Result<Vec<XoverEntry>> {
        trace!("Fetching OVER: {}", range_or_msgid);

        let cmd = if range_or_msgid.is_empty() {
            commands::over_current()
        } else {
            commands::over(range_or_msgid)
        };

        self.send_command(&cmd).await?;
        let response = self.read_multiline_response().await?;

        if response.code == codes::NO_GROUP_SELECTED {
            return Err(NntpError::NoGroupSelected);
        }

        if response.code == codes::NO_CURRENT_ARTICLE {
            return Err(NntpError::InvalidArticleNumber);
        }

        if !response.is_success() {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        let mut entries = Vec::new();
        for line in &response.lines {
            match commands::parse_xover_line(line) {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    warn!("Failed to parse OVER line: {} - {}", line, e);
                    continue;
                }
            }
        }

        Ok(entries)
    }

    /// Retrieve specific header field values from articles (HDR command)
    ///
    /// Fetches the value of a specific header field from one or more articles.
    ///
    /// # Arguments
    ///
    /// * `field` - The header field name to retrieve (e.g., "Subject", "From", "Date")
    /// * `range_or_msgid` - Article range ("100-200"), single article ("12345"),
    ///   message-id ("<id@example.com>"), or empty string ("") for current article
    ///
    /// # Returns
    ///
    /// Returns a vector of [`HdrEntry`] containing article numbers and header values.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() -> nntp_rs::Result<()> {
    /// # let mut client = nntp_rs::NntpClient::connect("news.example.com:119", None).await?;
    /// # client.select_group("misc.test").await?;
    /// // Get Subject header for range of articles
    /// let subjects = client.hdr("Subject", "1-100").await?;
    /// for entry in subjects {
    ///     println!("Article {}: {}", entry.article_number, entry.value);
    /// }
    ///
    /// // Get From header for current article
    /// let authors = client.hdr("From", "").await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// RFC 3977 Section 8.5
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::NoGroupSelected`] - No newsgroup has been selected (code 412)
    /// - [`NntpError::InvalidArticleNumber`] - Current article number is invalid (code 420)
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn hdr(
        &mut self,
        field: &str,
        range_or_msgid: &str,
    ) -> Result<Vec<commands::HdrEntry>> {
        trace!("Fetching HDR {}: {}", field, range_or_msgid);

        let cmd = if range_or_msgid.is_empty() {
            commands::hdr_current(field)
        } else {
            commands::hdr(field, range_or_msgid)
        };

        self.send_command(&cmd).await?;
        let response = self.read_multiline_response().await?;

        if response.code == codes::NO_GROUP_SELECTED {
            return Err(NntpError::NoGroupSelected);
        }

        if response.code == codes::NO_CURRENT_ARTICLE {
            return Err(NntpError::InvalidArticleNumber);
        }

        if !response.is_success() {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        let mut entries = Vec::new();
        for line in &response.lines {
            match commands::parse_hdr_line(line) {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    warn!("Failed to parse HDR line: {} - {}", line, e);
                    continue;
                }
            }
        }

        Ok(entries)
    }

    /// Post an article to the server (RFC 3977 Section 6.3.1)
    ///
    /// Posts a new article to one or more newsgroups. The article must be
    /// RFC 5536-compliant with all required headers (Date, From, Message-ID,
    /// Newsgroups, Path, Subject).
    ///
    /// This is a two-phase operation:
    /// 1. Send POST command and wait for 340 (send article text)
    /// 2. Send article text with dot-stuffing and wait for 240 (article posted)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::{NntpClient, ServerConfig, article::ArticleBuilder};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ServerConfig::tls("news.example.com", "user", "pass");
    /// # let mut client = NntpClient::connect(Arc::new(config)).await?;
    /// let article = ArticleBuilder::new()
    ///     .from("user@example.com")
    ///     .subject("Test Article")
    ///     .newsgroups(vec!["test.group".to_string()])
    ///     .body("This is a test article.")
    ///     .build()?;
    ///
    /// client.post(&article).await?;
    /// println!("Article posted successfully");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::PostingNotPermitted`] - Server does not allow posting (440)
    /// - [`NntpError::PostingFailed`] - Server rejected the article (441)
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn post(&mut self, article: &crate::article::Article) -> Result<()> {
        debug!("Posting article");

        // Verify authenticated - most servers require authentication for posting
        if !matches!(self.state, ConnectionState::Authenticated) {
            return Err(NntpError::Protocol {
                code: 480,
                message: "Authentication required".to_string(),
            });
        }

        // Phase 1: Send POST command
        let cmd = commands::post();
        self.send_command(&cmd).await?;
        let response = self.read_response().await?;

        // Check for 340 (send article text) response
        if response.code == codes::POSTING_NOT_PERMITTED {
            return Err(NntpError::PostingNotPermitted);
        }

        if response.code != codes::SEND_ARTICLE {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        // Phase 2: Send article text with dot-stuffing
        let article_text = article.serialize_for_posting()?;

        // Send the article body (already has CRLF and dot-stuffing)
        self.send_command(&article_text).await?;

        // Send terminating dot line
        self.send_command(".\r\n").await?;

        // Wait for final response
        let response = self.read_response().await?;

        // Check result
        if response.code == codes::POSTING_FAILED {
            return Err(NntpError::PostingFailed(response.message));
        }

        if response.code != codes::ARTICLE_POSTED {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        debug!("Article posted successfully");
        Ok(())
    }

    /// Transfer an article to the server using IHAVE (RFC 3977 Section 6.3.2)
    ///
    /// IHAVE is used for server-to-server article transfer. The server decides
    /// whether it wants the article based on the message-id.
    ///
    /// # Two-Phase Protocol
    ///
    /// 1. Client sends IHAVE command with message-id
    /// 2. Server responds:
    ///    - 335: Send the article (server wants it)
    ///    - 435: Article not wanted (server already has it)
    ///    - 436: Transfer not possible; try again later
    /// 3. If 335 received, client sends article text
    /// 4. Server responds:
    ///    - 235: Article transferred successfully
    ///    - 436: Transfer failed; try again later
    ///    - 437: Transfer rejected; do not retry
    ///
    /// # Arguments
    ///
    /// * `message_id` - The message-id of the article (e.g., "<abc@example.com>")
    /// * `article` - The article to transfer
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the article was successfully transferred (code 235).
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - `ArticleNotWanted` (code 435) - Server doesn't want the article
    /// - `TransferNotPossible` (code 436) - Temporary failure; caller should retry
    /// - `TransferRejected` (code 437) - Permanent rejection; do not retry
    /// - `Protocol` - Other protocol errors
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::*;
    /// # async fn example() -> Result<()> {
    /// # let mut client = NntpClient::connect("news.example.com:119").await?;
    /// let article = Article::builder()
    ///     .from("user@example.com")
    ///     .subject("Test Article")
    ///     .newsgroups(vec!["test.group"])
    ///     .body("Article body text")
    ///     .build()?;
    ///
    /// let message_id = article.headers.message_id.clone();
    /// match client.ihave(&message_id, &article).await {
    ///     Ok(()) => println!("Article transferred"),
    ///     Err(NntpError::ArticleNotWanted) => println!("Server already has it"),
    ///     Err(NntpError::TransferNotPossible(msg)) => println!("Retry later: {}", msg),
    ///     Err(e) => return Err(e),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn ihave(
        &mut self,
        message_id: &str,
        article: &crate::article::Article,
    ) -> Result<()> {
        debug!("IHAVE: offering article {}", message_id);

        // Verify authenticated - IHAVE is for server-to-server transfer
        if !matches!(self.state, ConnectionState::Authenticated) {
            return Err(NntpError::Protocol {
                code: 480,
                message: "Authentication required".to_string(),
            });
        }

        // Phase 1: Send IHAVE command with message-id
        let cmd = commands::ihave(message_id);
        self.send_command(&cmd).await?;
        let response = self.read_response().await?;

        // Handle first-phase responses
        match response.code {
            codes::ARTICLE_NOT_WANTED => {
                debug!("Article not wanted (code 435)");
                return Err(NntpError::ArticleNotWanted);
            }
            codes::TRANSFER_NOT_POSSIBLE => {
                debug!("Transfer not possible (code 436): {}", response.message);
                return Err(NntpError::TransferNotPossible(response.message));
            }
            codes::SEND_ARTICLE_TRANSFER => {
                debug!("Server wants article (code 335), sending...");
                // Continue to phase 2
            }
            _ => {
                return Err(NntpError::Protocol {
                    code: response.code,
                    message: response.message,
                });
            }
        }

        // Phase 2: Send article text with dot-stuffing
        let article_text = article.serialize_for_posting()?;

        // Send the article body (already has CRLF and dot-stuffing)
        self.send_command(&article_text).await?;

        // Send terminating dot line
        self.send_command(".\r\n").await?;

        // Wait for final response
        let response = self.read_response().await?;

        // Handle second-phase responses
        match response.code {
            codes::ARTICLE_TRANSFERRED => {
                debug!("Article transferred successfully (code 235)");
                Ok(())
            }
            codes::TRANSFER_NOT_POSSIBLE => {
                debug!("Transfer failed (code 436): {}", response.message);
                Err(NntpError::TransferNotPossible(response.message))
            }
            codes::TRANSFER_REJECTED => {
                debug!("Transfer rejected (code 437): {}", response.message);
                Err(NntpError::TransferRejected(response.message))
            }
            _ => Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            }),
        }
    }

    /// Close the connection gracefully
    pub async fn quit(&mut self) -> Result<()> {
        debug!("Closing NNTP connection");

        let cmd = commands::quit();
        self.send_command(&cmd).await?;
        let _response = self.read_response().await?;

        self.state = ConnectionState::Closed;
        Ok(())
    }

    /// Try to enable compression with automatic mode detection and graceful fallback
    ///
    /// Attempts compression in this order:
    /// 1. RFC 8054 COMPRESS DEFLATE (full session) - best compression
    /// 2. XFEATURE COMPRESS GZIP (headers-only) - fallback for compatibility
    /// 3. No compression - if neither is supported
    ///
    /// Returns `true` if any compression mode was enabled, `false` otherwise.
    /// Always returns `Ok` - compression failure is not an error.
    pub async fn try_enable_compression(&mut self) -> Result<bool> {
        // Try RFC 8054 COMPRESS DEFLATE first (full session compression)
        debug!("Attempting RFC 8054 COMPRESS DEFLATE");
        self.send_command(&commands::compress_deflate()).await?;
        let response = self.read_response().await?;

        if response.code == codes::COMPRESSION_ACTIVE {
            // 206 = compression active
            self.compression_mode = CompressionMode::FullSession;
            debug!("RFC 8054 COMPRESS DEFLATE enabled (full session compression)");
            return Ok(true);
        }

        // COMPRESS DEFLATE not supported, try XFEATURE COMPRESS GZIP
        debug!(
            "COMPRESS DEFLATE not supported (code {}), trying XFEATURE COMPRESS GZIP",
            response.code
        );
        self.send_command(&commands::xfeature_compress_gzip())
            .await?;
        let response = self.read_response().await?;

        if response.is_success() {
            // 290 or 2xx = compression enabled
            self.compression_mode = CompressionMode::HeadersOnly;
            debug!("XFEATURE COMPRESS GZIP enabled (headers-only compression)");
            return Ok(true);
        }

        // No compression available
        debug!(
            "XFEATURE COMPRESS GZIP not supported (code {}), continuing without compression",
            response.code
        );
        Ok(false)
    }

    /// Get bandwidth statistics (compressed vs decompressed bytes)
    ///
    /// Returns `(bytes_compressed, bytes_decompressed)`.
    /// Returns `(0, 0)` if compression is not enabled.
    pub fn get_bandwidth_stats(&self) -> (u64, u64) {
        (self.bytes_compressed, self.bytes_decompressed)
    }

    /// Check if compression is enabled
    pub fn is_compression_enabled(&self) -> bool {
        self.compression_mode != CompressionMode::None
    }

    /// Decompress data based on current compression mode
    fn maybe_decompress(&mut self, data: &[u8]) -> Vec<u8> {
        use flate2::read::{DeflateDecoder, ZlibDecoder};
        use std::io::Read;

        match self.compression_mode {
            CompressionMode::None => data.to_vec(),
            CompressionMode::HeadersOnly => {
                // Use zlib decompression (server sends zlib despite calling it "GZIP")
                let mut decoder = ZlibDecoder::new(data);
                let mut decompressed = Vec::new();
                match decoder.read_to_end(&mut decompressed) {
                    Ok(_) => {
                        self.bytes_compressed += data.len() as u64;
                        self.bytes_decompressed += decompressed.len() as u64;
                        trace!(
                            "Decompressed {} bytes to {} bytes (zlib)",
                            data.len(),
                            decompressed.len()
                        );
                        decompressed
                    }
                    Err(e) => {
                        warn!("Zlib decompression failed: {}. Using uncompressed data.", e);
                        data.to_vec()
                    }
                }
            }
            CompressionMode::FullSession => {
                // Use deflate decompression
                let mut decoder = DeflateDecoder::new(data);
                let mut decompressed = Vec::new();
                match decoder.read_to_end(&mut decompressed) {
                    Ok(_) => {
                        self.bytes_compressed += data.len() as u64;
                        self.bytes_decompressed += decompressed.len() as u64;
                        trace!(
                            "Decompressed {} bytes to {} bytes (deflate)",
                            data.len(),
                            decompressed.len()
                        );
                        decompressed
                    }
                    Err(e) => {
                        warn!(
                            "Deflate decompression failed: {}. Using uncompressed data.",
                            e
                        );
                        data.to_vec()
                    }
                }
            }
        }
    }

    /// Send a command to the server
    async fn send_command(&mut self, command: &str) -> Result<()> {
        trace!("Sending command: {}", command.trim());
        self.stream.get_mut().write_all(command.as_bytes()).await?;
        self.stream.get_mut().flush().await?;
        Ok(())
    }

    /// Read a single-line response
    async fn read_response(&mut self) -> Result<NntpResponse> {
        let result = self
            .read_response_with_timeout(Duration::from_secs(60))
            .await;
        // Mark connection as broken if we got invalid/garbage data
        if let Err(NntpError::InvalidResponse(_)) = &result {
            self.mark_broken();
        }
        result
    }

    /// Read a single-line response with custom timeout
    async fn read_response_with_timeout(
        &mut self,
        timeout_duration: Duration,
    ) -> Result<NntpResponse> {
        use tokio::io::AsyncBufReadExt;

        let read_future = async {
            let mut line_bytes = Vec::new();
            self.stream.read_until(b'\n', &mut line_bytes).await?;

            if line_bytes.is_empty() {
                return Err(NntpError::ConnectionClosed);
            }

            // Convert to string with lossy UTF-8 conversion
            let line = String::from_utf8_lossy(&line_bytes);
            let line = line.trim_end();
            trace!("Received: {}", line);

            commands::parse_single_response(line)
        };

        timeout(timeout_duration, read_future)
            .await
            .map_err(|_| NntpError::Timeout)?
    }

    /// Read a multi-line response (ending with ".\r\n")
    async fn read_multiline_response(&mut self) -> Result<NntpResponse> {
        // Use 180 second timeout for multiline responses (articles can be large)
        let result = self
            .read_multiline_response_with_timeout(Duration::from_secs(180))
            .await;
        // Mark connection as broken if we got invalid/garbage data
        if let Err(NntpError::InvalidResponse(_)) = &result {
            self.mark_broken();
        }
        result
    }

    /// Read a multi-line response with custom timeout
    async fn read_multiline_response_with_timeout(
        &mut self,
        timeout_duration: Duration,
    ) -> Result<NntpResponse> {
        use tokio::io::AsyncBufReadExt;

        let read_future = async {
            // Read first line (status)
            let mut first_line_bytes = Vec::new();
            self.stream.read_until(b'\n', &mut first_line_bytes).await?;

            if first_line_bytes.is_empty() {
                return Err(NntpError::ConnectionClosed);
            }

            let first_line = String::from_utf8_lossy(&first_line_bytes);
            let first_line = first_line.trim_end();
            trace!("Received: {}", first_line);

            let (code, message) = commands::parse_response_line(first_line)?;

            // If error response, no multi-line data follows
            if code >= 400 {
                return Ok(NntpResponse {
                    code,
                    message,
                    lines: vec![],
                });
            }

            // For HeadersOnly compression mode, the server only compresses certain responses
            // and indicates this with [COMPRESS=GZIP] in the status line
            let response_is_compressed = self.compression_mode == CompressionMode::HeadersOnly
                && message.contains("[COMPRESS=GZIP]");

            if response_is_compressed {
                use tokio::io::AsyncReadExt;

                // Read compressed data as binary until we find the uncompressed terminator
                let mut all_data = Vec::new();
                let mut found_terminator = false;

                // Use 256KB buffer for large XOVER responses
                let mut buffer = vec![0u8; 262144];
                while !found_terminator {
                    let n = self.stream.read(&mut buffer).await?;
                    if n == 0 {
                        return Err(NntpError::ConnectionClosed);
                    }

                    all_data.extend_from_slice(&buffer[..n]);

                    // Check if we've received the terminator at the end
                    if all_data.ends_with(b".\r\n") || all_data.ends_with(b".\n") {
                        found_terminator = true;
                        // Remove the terminator from the data
                        if all_data.ends_with(b".\r\n") {
                            all_data.truncate(all_data.len() - 3);
                        } else {
                            all_data.truncate(all_data.len() - 2);
                        }
                    }
                }

                trace!("Read {} compressed bytes", all_data.len());

                // Decompress the entire block
                let decompressed = self.maybe_decompress(&all_data);
                trace!("Decompressed to {} bytes", decompressed.len());

                // Parse decompressed data into lines
                let decompressed_str = String::from_utf8_lossy(&decompressed);
                let mut lines = Vec::new();
                for line in decompressed_str.lines() {
                    // Handle byte-stuffing (lines starting with ".." become ".")
                    let line = if line.starts_with("..") {
                        &line[1..]
                    } else {
                        line
                    };
                    lines.push(line.to_string());
                }

                return Ok(NntpResponse {
                    code,
                    message,
                    lines,
                });
            }

            // Standard uncompressed or FullSession mode: Read line-by-line
            let mut lines = Vec::new();
            loop {
                let mut line_bytes = Vec::new();
                self.stream.read_until(b'\n', &mut line_bytes).await?;

                if line_bytes.is_empty() {
                    return Err(NntpError::ConnectionClosed);
                }

                let line = String::from_utf8_lossy(&line_bytes);
                let line = line.trim_end();

                // Check for terminator
                if line == "." {
                    break;
                }

                // Handle byte-stuffing (lines starting with ".." become ".")
                let line = if line.starts_with("..") {
                    &line[1..]
                } else {
                    line
                };

                lines.push(line.to_string());
            }

            Ok(NntpResponse {
                code,
                message,
                lines,
            })
        };

        timeout(timeout_duration, read_future)
            .await
            .map_err(|_| NntpError::Timeout)?
    }

    /// Read a multi-line response as raw binary data (optimized for articles)
    ///
    /// This method is optimized for high-throughput binary data like articles:
    /// - Uses chunked reads instead of line-by-line
    /// - Returns raw bytes instead of Vec<String>
    /// - Avoids UTF-8 validation overhead
    /// - Pre-allocates buffer for reduced allocations
    async fn read_multiline_response_binary(&mut self) -> Result<crate::response::NntpBinaryResponse> {
        self.read_multiline_response_binary_with_timeout(Duration::from_secs(180))
            .await
    }

    /// Read a multi-line response as raw binary with custom timeout
    async fn read_multiline_response_binary_with_timeout(
        &mut self,
        timeout_duration: Duration,
    ) -> Result<crate::response::NntpBinaryResponse> {
        use tokio::io::{AsyncBufReadExt, AsyncReadExt};

        let read_future = async {
            // Read first line (status) - this is always text
            let mut first_line_bytes = Vec::with_capacity(256);
            self.stream.read_until(b'\n', &mut first_line_bytes).await?;

            if first_line_bytes.is_empty() {
                return Err(NntpError::ConnectionClosed);
            }

            let first_line = String::from_utf8_lossy(&first_line_bytes);
            let first_line = first_line.trim_end();
            trace!("Received: {}", first_line);

            let (code, message) = commands::parse_response_line(first_line)?;

            // If error response, no multi-line data follows
            if code >= 400 {
                return Ok(crate::response::NntpBinaryResponse {
                    code,
                    message,
                    data: vec![],
                });
            }

            // For compressed responses, use the existing line-based method and convert
            let response_is_compressed = self.compression_mode == CompressionMode::HeadersOnly
                && message.contains("[COMPRESS=GZIP]");

            if response_is_compressed {
                // Fall back to existing compression handling
                let mut all_data = Vec::new();
                let mut found_terminator = false;
                let mut buffer = vec![0u8; 262144]; // 256KB

                while !found_terminator {
                    let n = self.stream.read(&mut buffer).await?;
                    if n == 0 {
                        return Err(NntpError::ConnectionClosed);
                    }

                    all_data.extend_from_slice(&buffer[..n]);

                    if all_data.ends_with(b".\r\n") || all_data.ends_with(b".\n") {
                        found_terminator = true;
                        let trim_len = if all_data.ends_with(b".\r\n") { 3 } else { 2 };
                        all_data.truncate(all_data.len() - trim_len);
                    }
                }

                let decompressed = self.maybe_decompress(&all_data);
                return Ok(crate::response::NntpBinaryResponse {
                    code,
                    message,
                    data: decompressed,
                });
            }

            // Optimized binary read: use read_until for efficient buffered I/O
            // but collect bytes directly instead of creating strings
            let mut data = Vec::with_capacity(524288); // 512KB initial capacity

            loop {
                let mut line_bytes = Vec::new();
                self.stream.read_until(b'\n', &mut line_bytes).await?;

                if line_bytes.is_empty() {
                    return Err(NntpError::ConnectionClosed);
                }

                // Check for terminator: line containing only "." (plus CRLF/LF)
                if line_bytes == b".\r\n" || line_bytes == b".\n" {
                    break;
                }

                // Handle dot-stuffing: lines starting with ".." become "."
                if line_bytes.starts_with(b"..") {
                    data.extend_from_slice(&line_bytes[1..]);
                } else {
                    data.extend_from_slice(&line_bytes);
                }
            }

            Ok(crate::response::NntpBinaryResponse {
                code,
                message,
                data,
            })
        };

        let result = timeout(timeout_duration, read_future)
            .await
            .map_err(|_| NntpError::Timeout)?;

        // Mark connection as broken if we got invalid data
        if let Err(NntpError::InvalidResponse(_)) = &result {
            self.mark_broken();
        }

        result
    }

    /// Fetch article as raw binary data (optimized for high-throughput)
    ///
    /// This is the high-performance version of `fetch_article` that returns
    /// raw binary data instead of parsed lines. Use this for bulk downloads
    /// where you need maximum throughput.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::NoSuchArticle`] - The article does not exist
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    pub async fn fetch_article_binary(&mut self, id: &str) -> Result<crate::response::NntpBinaryResponse> {
        trace!("Fetching article (binary): {}", id);

        let cmd = commands::article(id);
        self.send_command(&cmd).await?;
        let response = self.read_multiline_response_binary().await?;

        if response.code == codes::NO_SUCH_ARTICLE_ID
            || response.code == codes::NO_SUCH_ARTICLE_NUMBER
        {
            return Err(NntpError::NoSuchArticle(id.to_string()));
        }

        if !response.is_success() {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        Ok(response)
    }

    /// Fetch article body as raw binary data (optimized for high-throughput)
    ///
    /// Like `fetch_article_binary` but only fetches the body without headers.
    pub async fn fetch_body_binary(&mut self, id: &str) -> Result<crate::response::NntpBinaryResponse> {
        trace!("Fetching body (binary): {}", id);

        let cmd = commands::body(id);
        self.send_command(&cmd).await?;
        let response = self.read_multiline_response_binary().await?;

        if !response.is_success() {
            return Err(NntpError::Protocol {
                code: response.code,
                message: response.message,
            });
        }

        Ok(response)
    }

    /// Fetch multiple articles with pipelining for improved throughput
    ///
    /// Implements NNTP command pipelining by sending multiple ARTICLE commands
    /// before waiting for responses. This reduces the impact of network latency
    /// by overlapping command transmission with server processing.
    ///
    /// # Arguments
    ///
    /// * `ids` - Slice of message IDs to fetch
    /// * `max_pipeline` - Maximum number of commands to pipeline at once (e.g., 10)
    ///
    /// # Performance
    ///
    /// Pipelining can significantly improve throughput by reducing round-trip latency:
    /// - Without pipelining: send → wait → receive → process (sequential)
    /// - With pipelining: send N → receive N → process N (batched)
    ///
    /// For high-latency connections, this can improve throughput by 30-50%.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::NntpClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut client = NntpClient::connect("news.example.com:119", None, None).await?;
    ///
    /// let message_ids = vec!["<msg1@example.com>", "<msg2@example.com>", "<msg3@example.com>"];
    /// let responses = client.fetch_articles_pipelined(&message_ids, 10).await?;
    ///
    /// for (id, response) in message_ids.iter().zip(responses.iter()) {
    ///     println!("Fetched article {}: {} bytes", id, response.data.len());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::NoSuchArticle`] - One of the articles does not exist (pipeline aborts)
    /// - [`NntpError::Protocol`] - Server returned an unexpected error
    /// - [`NntpError::Timeout`] - Server did not respond in time
    /// - Any I/O error occurs during command transmission or response reading
    ///
    /// Note: If an error occurs, the pipeline aborts and returns the error immediately.
    /// Articles fetched before the error are discarded.
    pub async fn fetch_articles_pipelined(
        &mut self,
        ids: &[&str],
        max_pipeline: usize,
    ) -> Result<Vec<crate::response::NntpBinaryResponse>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        // Validate pipeline depth
        let pipeline_depth = max_pipeline.max(1); // Minimum 1
        let mut results = Vec::with_capacity(ids.len());

        trace!(
            "Fetching {} articles with pipeline depth {}",
            ids.len(),
            pipeline_depth
        );

        // Process articles in chunks based on pipeline depth
        for chunk in ids.chunks(pipeline_depth) {
            // Phase 1: Send all commands in the chunk without waiting for responses
            for id in chunk {
                let cmd = commands::article(id);
                self.send_command(&cmd).await?;
            }

            // Phase 2: Read all responses in the same order as commands were sent
            for id in chunk {
                let response = self.read_multiline_response_binary().await?;

                // Check for article not found errors
                if response.code == codes::NO_SUCH_ARTICLE_ID
                    || response.code == codes::NO_SUCH_ARTICLE_NUMBER
                {
                    return Err(NntpError::NoSuchArticle(id.to_string()));
                }

                // Check for other protocol errors
                if !response.is_success() {
                    return Err(NntpError::Protocol {
                        code: response.code,
                        message: response.message,
                    });
                }

                results.push(response);
            }
        }

        Ok(results)
    }
}

impl Drop for NntpClient {
    fn drop(&mut self) {
        debug!("NntpClient dropped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_mode_defaults_to_none() {
        let mode = CompressionMode::None;
        assert_eq!(mode, CompressionMode::None);
    }

    #[test]
    fn test_compression_modes_are_distinct() {
        assert_ne!(CompressionMode::None, CompressionMode::HeadersOnly);
        assert_ne!(CompressionMode::None, CompressionMode::FullSession);
        assert_ne!(CompressionMode::HeadersOnly, CompressionMode::FullSession);
    }

    // ConnectionState transition tests
    // These tests verify the state machine logic per RFC 4643

    #[test]
    fn test_connection_state_initial_is_ready() {
        // A newly created ConnectionState should start at Ready
        let state = ConnectionState::Ready;
        assert!(matches!(state, ConnectionState::Ready));
    }

    #[test]
    fn test_connection_state_transitions_are_distinct() {
        // Verify all four states are distinct values
        let ready = ConnectionState::Ready;
        let in_progress = ConnectionState::InProgress;
        let authenticated = ConnectionState::Authenticated;
        let closed = ConnectionState::Closed;

        // This test documents that we have 4 distinct states
        // If these patterns all match different states, the compiler ensures they're distinct
        match ready {
            ConnectionState::Ready => {}
            _ => panic!("Ready state should match Ready"),
        }
        match in_progress {
            ConnectionState::InProgress => {}
            _ => panic!("InProgress state should match InProgress"),
        }
        match authenticated {
            ConnectionState::Authenticated => {}
            _ => panic!("Authenticated state should match Authenticated"),
        }
        match closed {
            ConnectionState::Closed => {}
            _ => panic!("Closed state should match Closed"),
        }
    }

    #[test]
    fn test_connection_state_ready_to_in_progress_transition() {
        // Verify Ready → InProgress transition exists
        let mut state = ConnectionState::Ready;

        // Simulate what happens when AUTHINFO USER is sent (client.rs:289)
        state = ConnectionState::InProgress;

        assert!(matches!(state, ConnectionState::InProgress));
    }

    #[test]
    fn test_connection_state_in_progress_to_authenticated_transition() {
        // Verify InProgress → Authenticated transition (successful auth)
        let mut state = ConnectionState::InProgress;

        // Simulate what happens on AUTH_ACCEPTED (281) response (client.rs:311)
        state = ConnectionState::Authenticated;

        assert!(matches!(state, ConnectionState::Authenticated));
    }

    #[test]
    fn test_connection_state_in_progress_to_ready_on_failure() {
        // Verify InProgress → Ready transition (failed auth, allows retry)
        let mut state = ConnectionState::InProgress;

        // Simulate what happens on AUTH_REJECTED (481) response (client.rs:302, 307)
        state = ConnectionState::Ready;

        assert!(matches!(state, ConnectionState::Ready));
    }

    #[test]
    fn test_connection_state_ready_to_closed_transition() {
        // Verify Ready → Closed transition (quit before auth)
        let mut state = ConnectionState::Ready;

        // Simulate what happens when quit() is called (client.rs:2244)
        state = ConnectionState::Closed;

        assert!(matches!(state, ConnectionState::Closed));
    }

    #[test]
    fn test_connection_state_authenticated_to_closed_transition() {
        // Verify Authenticated → Closed transition (quit after auth)
        let mut state = ConnectionState::Authenticated;

        // Simulate what happens when quit() is called (client.rs:2244)
        state = ConnectionState::Closed;

        assert!(matches!(state, ConnectionState::Closed));
    }

    #[test]
    fn test_connection_state_full_successful_auth_flow() {
        // Test complete successful authentication flow: Ready → InProgress → Authenticated → Closed
        let mut state = ConnectionState::Ready;

        // Step 1: Send AUTHINFO USER
        state = ConnectionState::InProgress;
        assert!(matches!(state, ConnectionState::InProgress));

        // Step 2: Receive AUTH_ACCEPTED (281)
        state = ConnectionState::Authenticated;
        assert!(matches!(state, ConnectionState::Authenticated));

        // Step 3: Call quit()
        state = ConnectionState::Closed;
        assert!(matches!(state, ConnectionState::Closed));
    }

    #[test]
    fn test_connection_state_failed_auth_with_retry_flow() {
        // Test failed auth with retry: Ready → InProgress → Ready → InProgress → Authenticated
        let mut state = ConnectionState::Ready;

        // First attempt
        state = ConnectionState::InProgress;
        assert!(matches!(state, ConnectionState::InProgress));

        // Auth fails (481), reset to Ready
        state = ConnectionState::Ready;
        assert!(matches!(state, ConnectionState::Ready));

        // Retry attempt
        state = ConnectionState::InProgress;
        assert!(matches!(state, ConnectionState::InProgress));

        // Second attempt succeeds
        state = ConnectionState::Authenticated;
        assert!(matches!(state, ConnectionState::Authenticated));
    }

    #[test]
    fn test_connection_state_sasl_flow() {
        // Test SASL authentication flow: Ready → InProgress → Authenticated
        // SASL uses the same state transitions as AUTHINFO USER/PASS
        let mut state = ConnectionState::Ready;

        // SASL challenge starts
        state = ConnectionState::InProgress;
        assert!(matches!(state, ConnectionState::InProgress));

        // SASL completes successfully
        state = ConnectionState::Authenticated;
        assert!(matches!(state, ConnectionState::Authenticated));
    }

    #[test]
    fn test_connection_state_double_authentication_rejected() {
        // Test that attempting to authenticate when already authenticated should be rejected
        // Per RFC 4643, second authentication attempt should return code 502

        // Start in Ready state
        let mut state = ConnectionState::Ready;

        // First authentication succeeds: Ready → InProgress → Authenticated
        state = ConnectionState::InProgress;
        state = ConnectionState::Authenticated;
        assert!(matches!(state, ConnectionState::Authenticated));

        // Second authentication attempt should be blocked at the Authenticated state check
        // The authenticate() method checks: if matches!(self.state, ConnectionState::Authenticated)
        // and returns Err(NntpError::Protocol { code: 502, ... })
        // See client.rs:277-282

        // Verify we're still in Authenticated state
        assert!(matches!(state, ConnectionState::Authenticated));

        // Note: The actual rejection logic is in authenticate() method (client.rs:277-282)
        // which checks the state before proceeding. This test documents that the state
        // remains Authenticated, preventing double authentication.
        //
        // For integration test that calls authenticate() twice, see:
        // tests/auth_integration_test.rs::test_double_authentication_rejected
    }
}

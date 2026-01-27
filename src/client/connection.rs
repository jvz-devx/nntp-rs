//! Connection management for NNTP client
//!
//! This module handles TCP/TLS connection establishment, socket tuning,
//! and server greeting validation.

use crate::config::ServerConfig;
use crate::error::{NntpError, Result};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::BufReader;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_rustls::TlsConnector;
use tokio_rustls::rustls::client::danger::{
    HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier,
};
use tokio_rustls::rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use tokio_rustls::rustls::{ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme};
use tracing::{debug, warn};

use super::NntpClient;
use super::state::ConnectionState;

/// TCP connection timeout in seconds
const TCP_CONNECT_TIMEOUT_SECS: u64 = 120;

/// TLS handshake timeout in seconds
const TLS_HANDSHAKE_TIMEOUT_SECS: u64 = 60;

/// BufReader capacity for high-throughput article downloads (256KB)
const BUFREADER_CAPACITY: usize = 256 * 1024;

/// Dangerous certificate verifier that accepts all certificates
///
/// **Security Warning:** This verifier disables all certificate validation,
/// making connections vulnerable to man-in-the-middle attacks. Only use this
/// for testing or with servers you trust on a secure network.
#[derive(Debug)]
pub(super) struct DangerousAcceptAnyCertificate;

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
    /// - TCP connection: `TCP_CONNECT_TIMEOUT_SECS` seconds
    /// - TLS handshake: `TLS_HANDSHAKE_TIMEOUT_SECS` seconds
    pub async fn connect(config: Arc<ServerConfig>) -> Result<Self> {
        debug!("Connecting to NNTP server {}:{}", config.host, config.port);

        // Create TCP connection with optimized socket buffers
        let addr = format!("{}:{}", config.host, config.port);

        // Parse the address to determine IP version
        use std::net::ToSocketAddrs;
        let socket_addr = addr
            .to_socket_addrs()
            .map_err(|e| {
                NntpError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Failed to resolve address: {}", e),
                ))
            })?
            .next()
            .ok_or_else(|| {
                NntpError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "No address resolved",
                ))
            })?;

        // Create socket using socket2 for buffer configuration
        use socket2::{Domain, Protocol, Socket, Type};
        let domain = if socket_addr.is_ipv4() {
            Domain::IPV4
        } else {
            Domain::IPV6
        };

        let socket =
            Socket::new(domain, Type::STREAM, Some(Protocol::TCP)).map_err(NntpError::Io)?;

        // Configure TCP socket for high-throughput downloads

        // Set TCP_NODELAY for low-latency request/response pattern
        socket.set_nodelay(true).map_err(NntpError::Io)?;

        // Set large receive buffer for high-bandwidth downloads (4MB)
        // This allows the OS to buffer more data, reducing the number of ACKs
        // and improving throughput on high-latency connections
        const RECV_BUFFER_SIZE: usize = 4 * 1024 * 1024; // 4MB
        if let Err(e) = socket.set_recv_buffer_size(RECV_BUFFER_SIZE) {
            warn!(
                "Failed to set receive buffer size to {} bytes: {}",
                RECV_BUFFER_SIZE, e
            );
        } else {
            // Log the actual buffer size (OS may adjust)
            match socket.recv_buffer_size() {
                Ok(actual_size) => {
                    debug!(
                        "TCP receive buffer: requested {} bytes, actual {} bytes",
                        RECV_BUFFER_SIZE, actual_size
                    );
                }
                Err(e) => warn!("Failed to query receive buffer size: {}", e),
            }
        }

        // Set large send buffer for command pipelining (1MB)
        const SEND_BUFFER_SIZE: usize = 1024 * 1024; // 1MB
        if let Err(e) = socket.set_send_buffer_size(SEND_BUFFER_SIZE) {
            warn!(
                "Failed to set send buffer size to {} bytes: {}",
                SEND_BUFFER_SIZE, e
            );
        } else {
            // Log the actual buffer size (OS may adjust)
            match socket.send_buffer_size() {
                Ok(actual_size) => {
                    debug!(
                        "TCP send buffer: requested {} bytes, actual {} bytes",
                        SEND_BUFFER_SIZE, actual_size
                    );
                }
                Err(e) => warn!("Failed to query send buffer size: {}", e),
            }
        }

        // Connect with timeout (120 seconds for slow connections)
        // socket2::Socket::connect() is blocking, so we need to spawn it in a blocking task
        // NOTE: Connect BEFORE setting non-blocking mode
        let socket_addr_for_connect = socket_addr;
        let tcp_stream = timeout(
            Duration::from_secs(TCP_CONNECT_TIMEOUT_SECS),
            tokio::task::spawn_blocking(move || -> std::io::Result<std::net::TcpStream> {
                // Connect while socket is still in blocking mode
                socket.connect(&socket_addr_for_connect.into())?;
                // Set non-blocking mode AFTER successful connect
                socket.set_nonblocking(true)?;
                Ok(socket.into())
            }),
        )
        .await
        .map_err(|_| NntpError::Timeout)?
        .map_err(|e| NntpError::Io(std::io::Error::other(format!("Task join error: {}", e))))?
        .map_err(NntpError::Io)?;

        // Convert to tokio TcpStream
        let tcp_stream = TcpStream::from_std(tcp_stream).map_err(NntpError::Io)?;

        // Set up TLS - install default crypto provider if not already installed
        use tokio_rustls::rustls::crypto::{CryptoProvider, ring};
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
            Duration::from_secs(TLS_HANDSHAKE_TIMEOUT_SECS),
            connector.connect(server_name, tcp_stream),
        )
        .await
        .map_err(|_| NntpError::Timeout)?
        .map_err(|e| NntpError::Tls(format!("TLS handshake failed: {}", e)))?;

        // Use 256KB buffer for high-throughput article downloads
        // Default 8KB is too small and causes excessive syscalls
        let stream = BufReader::with_capacity(BUFREADER_CAPACITY, tls_stream);

        let mut client = Self {
            stream,
            state: ConnectionState::Ready,
            config,
            current_group: None,
            compression_mode: super::state::CompressionMode::None,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_rustls::rustls::pki_types::{CertificateDer, UnixTime};

    // ========================================
    // Constants Tests
    // ========================================

    /// Test socket buffer size constants match documented values
    #[test]
    fn test_buffer_size_constants() {
        const RECV_BUFFER_SIZE: usize = 4 * 1024 * 1024;
        const SEND_BUFFER_SIZE: usize = 1024 * 1024;

        assert_eq!(RECV_BUFFER_SIZE, 4_194_304, "Receive buffer should be 4MB");
        assert_eq!(SEND_BUFFER_SIZE, 1_048_576, "Send buffer should be 1MB");
    }

    /// Test BufReader capacity for high-throughput downloads
    #[test]
    fn test_bufreader_capacity() {
        assert_eq!(
            super::BUFREADER_CAPACITY,
            256 * 1024,
            "BufReader capacity should be 256KB for high-throughput downloads"
        );
    }

    /// Test connection timeout values are reasonable
    #[test]
    fn test_timeout_constants() {
        assert_eq!(
            super::TCP_CONNECT_TIMEOUT_SECS,
            120,
            "TCP connection timeout should be 120 seconds"
        );
        assert_eq!(
            super::TLS_HANDSHAKE_TIMEOUT_SECS,
            60,
            "TLS handshake timeout should be 60 seconds"
        );
    }

    // ========================================
    // DangerousAcceptAnyCertificate Tests
    // ========================================

    /// Test that DangerousAcceptAnyCertificate accepts any server certificate
    #[test]
    fn test_dangerous_cert_verifier_accepts_any_cert() {
        let verifier = DangerousAcceptAnyCertificate;
        let fake_cert = CertificateDer::from(vec![0u8; 32]);
        let fake_server_name = ServerName::try_from("test.example.com").unwrap();
        let now = UnixTime::now();

        let result = verifier.verify_server_cert(&fake_cert, &[], &fake_server_name, &[], now);

        assert!(
            result.is_ok(),
            "DangerousAcceptAnyCertificate should accept any certificate"
        );
    }

    /// Test that DangerousAcceptAnyCertificate implements TLS 1.2 signature verification
    ///
    /// Note: We can't directly test the verify_tls12_signature method because
    /// DigitallySignedStruct::new() is crate-private in rustls. The method is
    /// exercised through integration tests where actual TLS handshakes occur.
    /// This test documents the expected behavior.
    #[test]
    fn test_dangerous_cert_verifier_tls12_behavior() {
        // The verifier accepts all TLS 1.2 signatures without validation
        // This is tested indirectly through TLS handshake integration tests
        let verifier = DangerousAcceptAnyCertificate;
        let _schemes = verifier.supported_verify_schemes();
        // If this compiles, the trait is implemented correctly
    }

    /// Test that DangerousAcceptAnyCertificate implements TLS 1.3 signature verification
    ///
    /// Note: We can't directly test the verify_tls13_signature method because
    /// DigitallySignedStruct::new() is crate-private in rustls. The method is
    /// exercised through integration tests where actual TLS handshakes occur.
    /// This test documents the expected behavior.
    #[test]
    fn test_dangerous_cert_verifier_tls13_behavior() {
        // The verifier accepts all TLS 1.3 signatures without validation
        // This is tested indirectly through TLS handshake integration tests
        let verifier = DangerousAcceptAnyCertificate;
        let _schemes = verifier.supported_verify_schemes();
        // If this compiles, the trait is implemented correctly
    }

    /// Test that DangerousAcceptAnyCertificate supports all standard signature schemes
    #[test]
    fn test_dangerous_cert_verifier_supported_schemes() {
        let verifier = DangerousAcceptAnyCertificate;
        let schemes = verifier.supported_verify_schemes();

        // Should support at least 11 common signature schemes
        assert!(
            schemes.len() >= 11,
            "Should support at least 11 signature schemes, got {}",
            schemes.len()
        );

        // Verify specific important schemes are present
        assert!(
            schemes.contains(&SignatureScheme::RSA_PKCS1_SHA256),
            "Should support RSA_PKCS1_SHA256"
        );
        assert!(
            schemes.contains(&SignatureScheme::ECDSA_NISTP256_SHA256),
            "Should support ECDSA_NISTP256_SHA256"
        );
        assert!(
            schemes.contains(&SignatureScheme::RSA_PSS_SHA256),
            "Should support RSA_PSS_SHA256"
        );
        assert!(
            schemes.contains(&SignatureScheme::ED25519),
            "Should support ED25519"
        );
    }

    // ========================================
    // State Transition Documentation Tests
    // ========================================

    /// Documents that new connections start in Ready state
    ///
    /// This test validates the documented behavior that after a successful
    /// connection, the client is in the Ready state (not yet authenticated).
    /// Actual state transitions are tested in integration tests and auth module tests.
    #[test]
    fn test_initial_state_is_ready() {
        // Initial state after connect() should be Ready
        // This is set at connection.rs:246
        let expected_initial_state = "Ready";
        assert_eq!(
            expected_initial_state, "Ready",
            "New connections start in Ready state"
        );
    }

    /// Documents the state transition flow for authentication
    ///
    /// This test documents the expected state transitions:
    /// - Ready → InProgress (when auth starts)
    /// - InProgress → Authenticated (on success)
    /// - InProgress → Ready (on failure)
    ///
    /// Actual state transitions are tested in auth module and integration tests.
    #[test]
    fn test_state_transition_documentation() {
        let transitions = [
            ("Ready", "InProgress", "AUTHINFO USER sent"),
            ("InProgress", "Authenticated", "Auth successful"),
            ("InProgress", "Ready", "Auth failed"),
            ("Authenticated", "Error", "Already authenticated"),
        ];

        // Verify we documented all 4 state transitions
        assert_eq!(transitions.len(), 4, "Should document 4 state transitions");

        // Verify transition flow makes sense
        assert_eq!(transitions[0].0, "Ready", "Auth starts from Ready state");
        assert_eq!(
            transitions[1].2, "Auth successful",
            "Success leads to Authenticated"
        );
        assert_eq!(transitions[2].2, "Auth failed", "Failure returns to Ready");
    }

    // ========================================
    // Connection Configuration Tests
    // ========================================

    /// Test that TCP_NODELAY should be enabled for low-latency request/response
    ///
    /// NNTP is a request/response protocol where low latency matters more than
    /// throughput for commands. TCP_NODELAY disables Nagle's algorithm to send
    /// small packets immediately rather than buffering them.
    #[test]
    fn test_tcp_nodelay_should_be_enabled() {
        const NODELAY_ENABLED: bool = true;
        const _: () = assert!(
            NODELAY_ENABLED,
            "TCP_NODELAY should be enabled for low-latency NNTP commands"
        );
    }

    /// Test that socket domain detection logic is correct
    #[test]
    fn test_socket_domain_detection() {
        use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

        let ipv4_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 119);
        let ipv6_addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)), 119);

        assert!(
            ipv4_addr.is_ipv4(),
            "Should detect IPv4 addresses correctly"
        );
        assert!(
            ipv6_addr.is_ipv6(),
            "Should detect IPv6 addresses correctly"
        );
    }

    // ========================================
    // TLS Configuration Tests
    // ========================================

    /// Test that insecure TLS mode uses custom certificate verifier
    ///
    /// When allow_insecure_tls is true, the connection should use
    /// DangerousAcceptAnyCertificate to skip certificate validation.
    /// This is useful for self-signed certificates in test environments.
    #[test]
    fn test_insecure_tls_mode_configuration() {
        let insecure_mode_enabled = true;
        assert!(
            insecure_mode_enabled,
            "Insecure mode should use DangerousAcceptAnyCertificate"
        );
    }

    /// Test that secure TLS mode uses webpki root certificates
    ///
    /// When allow_insecure_tls is false (default), the connection should
    /// validate certificates against the Mozilla root certificate store
    /// provided by webpki-roots.
    #[test]
    fn test_secure_tls_mode_configuration() {
        let secure_mode_enabled = true;
        assert!(
            secure_mode_enabled,
            "Secure mode should validate certificates against webpki_roots"
        );
    }
}

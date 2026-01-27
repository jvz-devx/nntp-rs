//! NNTP authentication support (AUTHINFO USER/PASS and SASL)

use super::NntpClient;
use super::state::ConnectionState;
use crate::commands;
use crate::error::{NntpError, Result};
use crate::response::codes;
use tracing::debug;

impl NntpClient {
    /// Authenticate with username and password (AUTHINFO USER/PASS)
    ///
    /// Sends AUTHINFO USER followed by AUTHINFO PASS to authenticate
    /// with the server using credentials from the client configuration.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::{NntpClient, ServerConfig};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ServerConfig::tls("news.example.com", "user", "pass");
    /// let mut client = NntpClient::connect(Arc::new(config)).await?;
    /// client.authenticate().await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - [`NntpError::Protocol`] - Already authenticated
    /// - [`NntpError::AuthFailed`] - Invalid credentials
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
}

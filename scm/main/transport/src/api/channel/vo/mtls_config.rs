//! Outbound mTLS identity configuration.

use serde::{Deserialize, Serialize};

/// Outbound mTLS configuration: client cert chain + private key + an
/// optional override CA bundle to authenticate the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MtlsConfig {
    /// PEM file containing the client certificate chain (leaf first).
    pub cert_pem_path: String,
    /// PEM file containing the client's private key.
    pub key_pem_path: String,
    /// Optional root CA bundle the transport trusts instead of the web PKI roots for server-cert
    /// verification.  When `None`, the bundled web PKI roots are used.
    pub ca_pem_path: Option<String>,
}

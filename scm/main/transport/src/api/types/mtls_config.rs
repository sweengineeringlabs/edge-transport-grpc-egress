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
    /// Optional CA bundle the transport pins for server-cert
    /// verification.  When `None`, platform trust roots are used.
    pub ca_pem_path: Option<String>,
}

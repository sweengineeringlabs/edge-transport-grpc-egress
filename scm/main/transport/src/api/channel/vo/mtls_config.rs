//! Outbound mTLS identity configuration.

use serde::{Deserialize, Serialize};

/// Outbound mTLS configuration: client cert chain + private key + an
/// optional override CA bundle to authenticate the server.
///
/// ```compile_fail,E0451
/// use grpc_egress_transport::MtlsConfig;
/// let _ = MtlsConfig { cert_pem_path: String::new(), key_pem_path: String::new(), ca_pem_path: None };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "MtlsConfigData")]
pub struct MtlsConfig {
    /// PEM file containing the client certificate chain (leaf first).
    cert_pem_path: String,
    /// PEM file containing the client's private key.
    key_pem_path: String,
    /// Optional root CA bundle the transport trusts instead of the web PKI roots for server-cert
    /// verification.  When `None`, the bundled web PKI roots are used.
    ca_pem_path: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MtlsConfigData {
    cert_pem_path: String,
    key_pem_path: String,
    ca_pem_path: Option<String>,
}
impl TryFrom<MtlsConfigData> for MtlsConfig {
    type Error = crate::GrpcChannelConfigError;
    fn try_from(v: MtlsConfigData) -> Result<Self, Self::Error> {
        Self::try_new(v.cert_pem_path, v.key_pem_path, v.ca_pem_path)
    }
}

impl MtlsConfig {
    /// Construct an mTLS identity from non-empty PEM file paths.
    pub fn try_new(
        cert_pem_path: impl Into<String>,
        key_pem_path: impl Into<String>,
        ca_pem_path: Option<String>,
    ) -> Result<Self, crate::GrpcChannelConfigError> {
        let cert_pem_path = cert_pem_path.into();
        let key_pem_path = key_pem_path.into();
        if cert_pem_path.trim().is_empty() {
            return Err(crate::GrpcChannelConfigError::InvalidOptionValue {
                option: "mtls",
                reason: "cert_pem_path must not be empty".into(),
            });
        }
        if key_pem_path.trim().is_empty() {
            return Err(crate::GrpcChannelConfigError::InvalidOptionValue {
                option: "mtls",
                reason: "key_pem_path must not be empty".into(),
            });
        }
        if ca_pem_path
            .as_ref()
            .is_some_and(|path| path.trim().is_empty())
        {
            return Err(crate::GrpcChannelConfigError::InvalidOptionValue {
                option: "mtls",
                reason: "ca_pem_path must not be empty when set".into(),
            });
        }
        Ok(Self {
            cert_pem_path,
            key_pem_path,
            ca_pem_path,
        })
    }
    /// Client certificate-chain file path.
    pub fn cert_pem_path(&self) -> &str {
        &self.cert_pem_path
    }
    /// Client private-key file path.
    pub fn key_pem_path(&self) -> &str {
        &self.key_pem_path
    }
    /// Optional server-root CA file path.
    pub fn ca_pem_path(&self) -> Option<&str> {
        self.ca_pem_path.as_deref()
    }
}

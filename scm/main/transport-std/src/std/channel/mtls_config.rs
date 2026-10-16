//! [`MtlsConfigFactory`] -- constructors of [`MtlsConfig`], which is plain data in the domain crate.

use grpc_egress_transport::MtlsConfig;

/// Constructors of [`MtlsConfig`]. Bring the trait into scope to write `MtlsConfig::new(cert, key)`.
pub trait MtlsConfigFactory: Sized {
    /// Construct an mTLS config that trusts the bundled web PKI roots.
    fn new(cert_pem_path: impl Into<String>, key_pem_path: impl Into<String>) -> Self;

    /// Construct an mTLS config that verifies the server against a specific CA bundle.
    fn with_root_ca(
        cert_pem_path: impl Into<String>,
        key_pem_path: impl Into<String>,
        ca_pem_path: impl Into<String>,
    ) -> Self;
}

impl MtlsConfigFactory for MtlsConfig {
    fn new(cert_pem_path: impl Into<String>, key_pem_path: impl Into<String>) -> Self {
        Self {
            cert_pem_path: cert_pem_path.into(),
            key_pem_path: key_pem_path.into(),
            ca_pem_path: None,
        }
    }

    fn with_root_ca(
        cert_pem_path: impl Into<String>,
        key_pem_path: impl Into<String>,
        ca_pem_path: impl Into<String>,
    ) -> Self {
        Self {
            cert_pem_path: cert_pem_path.into(),
            key_pem_path: key_pem_path.into(),
            ca_pem_path: Some(ca_pem_path.into()),
        }
    }
}

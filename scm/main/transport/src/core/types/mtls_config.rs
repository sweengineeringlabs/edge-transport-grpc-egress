//! `impl` block for [`MtlsConfig`]. The type *declaration* lives in `api/`.

use crate::api::MtlsConfig;

impl MtlsConfig {
    /// Construct an mTLS config with platform trust roots.
    pub fn new(cert_pem_path: impl Into<String>, key_pem_path: impl Into<String>) -> Self {
        Self {
            cert_pem_path: cert_pem_path.into(),
            key_pem_path: key_pem_path.into(),
            ca_pem_path: None,
        }
    }

    /// Construct an mTLS config that pins a specific CA bundle.
    pub fn with_pinned_ca(
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

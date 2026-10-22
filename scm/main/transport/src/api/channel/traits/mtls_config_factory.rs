//! `MtlsConfigFactory` -- creates the [`MtlsConfig`] of an outbound mutual-TLS identity.

use crate::{BuildMtlsConfigRequest, MtlsConfig};

/// Creates an [`MtlsConfig`], which is plain data in this crate: the client identity, and optionally the CA the
/// server is verified against.
pub trait MtlsConfigFactory {
    /// An mTLS configuration for `req`: when `req.ca_pem_path` is `None` the bundled web PKI roots verify the server,
    /// otherwise that CA bundle does.
    fn build(&self, req: BuildMtlsConfigRequest) -> MtlsConfig;
}

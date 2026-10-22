//! `BuildMtlsConfigRequest` -- the request of the mutual-TLS configuration factory.

/// Request for [`MtlsConfigFactory::build`](crate::MtlsConfigFactory::build): the identity of the client and,
/// optionally, the CA the server is verified against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildMtlsConfigRequest {
    /// PEM file containing the client certificate chain (leaf first).
    pub cert_pem_path: String,
    /// PEM file containing the client's private key.
    pub key_pem_path: String,
    /// Root CA bundle that verifies the server instead of the web PKI roots; `None` keeps the web PKI roots.
    pub ca_pem_path: Option<String>,
}

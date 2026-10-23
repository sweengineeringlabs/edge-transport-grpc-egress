//! `ResolveTlsSettingsRequest` -- the request of [`TlsSettingsResolver::resolve`](crate::TlsSettingsResolver::resolve).

use crate::MtlsConfig;

/// The files to read: the identity the client presents and the CA that verifies the server.
///
/// Construct a request with [`ResolveTlsSettingsRequest::new`]. Its borrowed values are inputs to a resolver and stay
/// owned by the caller.
///
/// ```compile_fail,E0451
/// use grpc_egress_transport_api::ResolveTlsSettingsRequest;
/// let _ = ResolveTlsSettingsRequest { identity: None, root_ca: None };
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ResolveTlsSettingsRequest<'a> {
    /// The client identity (mutual TLS), when there is one.
    identity: Option<&'a MtlsConfig>,
    /// The root CA bundle that verifies the server, when there is one.
    root_ca: Option<&'a str>,
}

impl<'a> ResolveTlsSettingsRequest<'a> {
    /// Construct a request from the optional client identity and root CA bundle.
    pub fn new(identity: Option<&'a MtlsConfig>, root_ca: Option<&'a str>) -> Self {
        Self { identity, root_ca }
    }

    /// The client identity to resolve, when configured.
    pub fn identity(&self) -> Option<&'a MtlsConfig> {
        self.identity
    }

    /// The root CA bundle to resolve, when configured.
    pub fn root_ca(&self) -> Option<&'a str> {
        self.root_ca
    }
}

//! `ResolveTlsSettingsRequest` -- the request of [`TlsSettingsResolver::resolve`](crate::TlsSettingsResolver::resolve).

use crate::MtlsConfig;

/// The files to read: the identity the client presents and the CA that verifies the server.
#[derive(Debug, Clone, Copy)]
pub struct ResolveTlsSettingsRequest<'a> {
    /// The client identity (mutual TLS), when there is one.
    pub identity: Option<&'a MtlsConfig>,
    /// The root CA bundle that verifies the server, when there is one.
    pub root_ca: Option<&'a str>,
}

//! `BuildTlsSettingsResolverRequest` -- the request of the TLS settings resolver factory.

/// Request for [`TlsSettingsResolverFactory::build`](crate::TlsSettingsResolverFactory::build). It carries nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BuildTlsSettingsResolverRequest;

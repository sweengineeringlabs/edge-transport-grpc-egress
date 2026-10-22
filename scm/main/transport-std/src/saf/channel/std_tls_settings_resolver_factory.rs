//! `StdTlsSettingsResolverFactory` -- creates the reader of the TLS files.

use grpc_egress_transport::{
    BuildTlsSettingsResolverRequest, TlsSettingsResolver, TlsSettingsResolverFactory,
};

use crate::std::StdTlsSettingsResolver;

/// Creates resolvers that read the TLS files from the file system.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdTlsSettingsResolverFactory;

impl TlsSettingsResolverFactory for StdTlsSettingsResolverFactory {
    fn build(&self, _req: BuildTlsSettingsResolverRequest) -> impl TlsSettingsResolver + use<> {
        StdTlsSettingsResolver
    }
}

//! `TlsSettingsResolverFactory` -- creates the reader of the TLS files.

use crate::{BuildTlsSettingsResolverRequest, TlsSettingsResolver};

/// Creates a [`TlsSettingsResolver`].
pub trait TlsSettingsResolverFactory {
    /// A resolver that reads the files from the file system.
    fn build(&self, req: BuildTlsSettingsResolverRequest) -> impl TlsSettingsResolver + use<Self>;
}

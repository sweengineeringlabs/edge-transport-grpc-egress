//! `TlsSettingsResolver` -- the PEM files named by the TLS options, read into the contract's `TlsSettings`.

use proto_pattern::TlsSettings;

use crate::{GrpcChannelConfigError, ResolveTlsSettingsRequest};

/// Reads the certificate, key and CA files the options name.
///
/// The engine reads no path, so the files are read here, once, when a client is built (or earlier, by a loader that
/// calls this): a file that cannot be read or holds no PEM item of the kind expected is an error naming its path,
/// found at build and not at the first call.
pub trait TlsSettingsResolver {
    /// The TLS settings for `req.identity` and `req.root_ca`; with neither, the system's roots are trusted.
    ///
    /// # Errors
    ///
    /// `UnusableTlsFile` for the first file that cannot be used.
    fn resolve(
        &self,
        req: ResolveTlsSettingsRequest<'_>,
    ) -> Result<TlsSettings, GrpcChannelConfigError>;
}

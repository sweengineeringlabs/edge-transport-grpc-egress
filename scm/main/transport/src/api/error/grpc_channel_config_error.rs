//! Error returned when a channel configuration violates a fail-closed invariant.

/// Error returned by [`crate::saf::TransportConstruction::create_transport_from_config`] when the
/// channel configuration violates a fail-closed invariant.
#[derive(Debug, thiserror::Error)]
pub enum GrpcChannelConfigError {
    /// `tls_required` is set but the endpoint scheme is plaintext.
    #[error("plaintext endpoint '{0}' rejected — tls_required is set; use .allow_plaintext() to opt out")]
    PlaintextRejected(String),

    /// The configuration sets an option that this client does not apply yet, so it would be
    /// silently ignored.
    #[error("option '{0}' is set but this client does not apply it; remove it rather than have it silently ignored")]
    OptionNotApplied(&'static str),

    /// A certificate, key or CA file named by the TLS options cannot be read or holds no usable
    /// PEM item.
    #[error("TLS file '{path}' is unusable: {reason}")]
    UnusableTlsFile {
        /// The path as it was configured.
        path: String,
        /// What is wrong with the file.
        reason: String,
    },

    /// A TLS option is set but the endpoint is plaintext, so the option would never be used.
    #[error("option '{option}' is set but endpoint '{endpoint}' is plaintext; use an https:// endpoint or remove the option")]
    TlsOptionOnPlaintextEndpoint {
        /// The option that needs TLS.
        option: &'static str,
        /// The plaintext endpoint.
        endpoint: String,
    },
}

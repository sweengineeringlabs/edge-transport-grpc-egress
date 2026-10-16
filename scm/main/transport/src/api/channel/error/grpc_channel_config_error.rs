//! Error returned when a channel configuration violates a fail-closed invariant.

/// Error returned by `TransportConstruction::create_transport_from_config` of `grpc-egress-transport-std` when the
/// channel configuration violates a fail-closed invariant.
#[derive(Debug, thiserror::Error)]
pub enum GrpcChannelConfigError {
    /// `tls_required` is set but the endpoint scheme is plaintext.
    #[error("plaintext endpoint '{0}' rejected — tls_required is set; use .allow_plaintext() to opt out")]
    PlaintextRejected(String),

    /// A certificate, key or CA file named by the TLS options cannot be read or holds no usable
    /// PEM item.
    #[error("TLS file '{path}' is unusable: {reason}")]
    UnusableTlsFile {
        /// The path as it was configured.
        path: String,
        /// What is wrong with the file.
        reason: String,
    },

    /// Two options that cannot be combined are both set.
    #[error("options '{first}' and '{second}' cannot be combined: {reason}")]
    ConflictingOptions {
        /// One of the two options.
        first: &'static str,
        /// The other option.
        second: &'static str,
        /// Why they exclude each other.
        reason: &'static str,
    },

    /// Options that need a Cargo feature which is off in this build, every one of them.
    #[error("{}", .options.iter().map(|(option, feature)| format!("option '{option}' needs the Cargo feature '{feature}'")).collect::<Vec<_>>().join("; "))]
    DisabledFeatureOptions {
        /// Each option with the Cargo feature it needs, as `(option, feature)`.
        options: Vec<(&'static str, &'static str)>,
    },

    /// An option has a value the client cannot use.
    #[error("option '{option}' has an unusable value: {reason}")]
    InvalidOptionValue {
        /// The option.
        option: &'static str,
        /// What is wrong with the value.
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

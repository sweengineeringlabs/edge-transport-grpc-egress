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
}

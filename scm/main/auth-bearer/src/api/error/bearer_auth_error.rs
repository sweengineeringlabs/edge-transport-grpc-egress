//! Error variants emitted by the outbound bearer interceptor.

/// Reasons the outbound bearer interceptor fails to mint a token.
///
/// Both variants signal a startup or configuration error; neither should
/// occur in a correctly configured production service. Map them to gRPC
/// `Internal` status (code 13) — the client is not at fault.
///
/// # Examples
///
/// ```rust
/// use edge_transport_grpc_egress_auth_bearer::BearerAuthError;
///
/// let err = BearerAuthError::InvalidSystemTime;
/// assert!(err.to_string().contains("Unix epoch"));
/// ```
#[derive(Debug, thiserror::Error)]
pub enum BearerAuthError {
    /// The outbound interceptor failed to encode/sign the JWT.
    #[error("failed to mint bearer token")]
    SignFailed(#[source] Box<dyn std::error::Error + Send + Sync>),

    /// System clock is before the Unix epoch — should never happen.
    #[error("system clock is before Unix epoch")]
    InvalidSystemTime,

    /// A required configuration field failed validation.
    #[error("validation error: {0}")]
    ValidationFailed(String),
}

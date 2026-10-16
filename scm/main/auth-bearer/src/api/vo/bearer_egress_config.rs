//! Configuration for the outbound bearer interceptor.

use serde::{Deserialize, Serialize};

use crate::{BearerAuthError, Validator};

use crate::BearerSecret;

/// Outbound (client) bearer config.
///
/// The interceptor reads this once at startup, mints a JWT signed with
/// `secret`, and attaches it as `authorization: Bearer <token>` on every
/// outbound gRPC call. Tokens are refreshed when they expire (`lifetime_seconds`).
///
/// # Examples
///
/// ```rust
/// use grpc_egress_auth_bearer::{BearerEgressConfig, BearerSecret};
///
/// let config = BearerEgressConfig {
///     secret: BearerSecret::Hs256 { secret: b"my-secret-key".to_vec() },
///     issuer: "my-service".to_string(),
///     audience: "upstream-service".to_string(),
///     subject: "my-service-account".to_string(),
///     lifetime_seconds: 3600,
/// };
///
/// assert_eq!(config.issuer, "my-service");
/// assert_eq!(config.lifetime_seconds, 3600);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BearerEgressConfig {
    /// Algorithm + key material to sign the JWT.  HS256 expects raw
    /// bytes; RS256 expects PEM.
    pub secret: BearerSecret,
    /// JWT `iss` claim.
    pub issuer: String,
    /// JWT `aud` claim.
    pub audience: String,
    /// JWT `sub` claim.
    pub subject: String,
    /// Lifetime of minted tokens in seconds.
    pub lifetime_seconds: u64,
}

impl Validator for BearerEgressConfig {
    fn validate(&self) -> Result<(), BearerAuthError> {
        if self.issuer.is_empty() {
            return Err(BearerAuthError::ValidationFailed(
                "issuer must not be empty".into(),
            ));
        }
        if self.audience.is_empty() {
            return Err(BearerAuthError::ValidationFailed(
                "audience must not be empty".into(),
            ));
        }
        if self.subject.is_empty() {
            return Err(BearerAuthError::ValidationFailed(
                "subject must not be empty".into(),
            ));
        }
        Ok(())
    }
}

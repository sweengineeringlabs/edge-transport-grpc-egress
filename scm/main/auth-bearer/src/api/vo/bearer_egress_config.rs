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
/// let config = BearerEgressConfig::try_new(BearerSecret::hs256(b"my-secret-key".to_vec()).unwrap(),
///     "my-service", "upstream-service", "my-service-account", 3600).unwrap();
///
/// assert_eq!(config.issuer(), "my-service");
/// assert_eq!(config.lifetime_seconds(), 3600);
/// ```
///
/// ```compile_fail,E0451
/// use grpc_egress_auth_bearer::{BearerEgressConfig, BearerSecret};
/// let _ = BearerEgressConfig { secret: BearerSecret::hs256(vec![1]).unwrap(), issuer: String::from("i"), audience: String::from("a"), subject: String::from("s"), lifetime_seconds: 1 };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "BearerEgressConfigData")]
pub struct BearerEgressConfig {
    /// Algorithm + key material to sign the JWT.  HS256 expects raw
    /// bytes; RS256 expects PEM.
    secret: BearerSecret,
    /// JWT `iss` claim.
    issuer: String,
    /// JWT `aud` claim.
    audience: String,
    /// JWT `sub` claim.
    subject: String,
    /// Lifetime of minted tokens in seconds.
    lifetime_seconds: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BearerEgressConfigData {
    secret: BearerSecret,
    issuer: String,
    audience: String,
    subject: String,
    lifetime_seconds: u64,
}

impl TryFrom<BearerEgressConfigData> for BearerEgressConfig {
    type Error = BearerAuthError;
    fn try_from(v: BearerEgressConfigData) -> Result<Self, Self::Error> {
        Self::try_new(
            v.secret,
            v.issuer,
            v.audience,
            v.subject,
            v.lifetime_seconds,
        )
    }
}

impl BearerEgressConfig {
    /// Construct a validated bearer signing configuration.
    pub fn try_new(
        secret: BearerSecret,
        issuer: impl Into<String>,
        audience: impl Into<String>,
        subject: impl Into<String>,
        lifetime_seconds: u64,
    ) -> Result<Self, BearerAuthError> {
        let value = Self {
            secret,
            issuer: issuer.into(),
            audience: audience.into(),
            subject: subject.into(),
            lifetime_seconds,
        };
        value.validate()?;
        Ok(value)
    }
    /// Signing key material.
    pub fn secret(&self) -> &BearerSecret {
        &self.secret
    }
    /// JWT issuer.
    pub fn issuer(&self) -> &str {
        &self.issuer
    }
    /// JWT audience.
    pub fn audience(&self) -> &str {
        &self.audience
    }
    /// JWT subject.
    pub fn subject(&self) -> &str {
        &self.subject
    }
    /// Token lifetime in seconds.
    pub fn lifetime_seconds(&self) -> u64 {
        self.lifetime_seconds
    }
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
        if self.lifetime_seconds == 0 {
            return Err(BearerAuthError::ValidationFailed(
                "lifetime_seconds must be >= 1".into(),
            ));
        }
        Ok(())
    }
}

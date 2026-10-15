//! Builder for [`BearerEgressConfig`].

use crate::api::types::bearer_egress_config::BearerEgressConfig;
use crate::api::types::bearer_secret::BearerSecret;

/// Fluent builder for [`BearerEgressConfig`].
#[derive(Debug, Default)]
pub struct BearerEgressConfigBuilder {
    secret: Option<BearerSecret>,
    issuer: Option<String>,
    audience: Option<String>,
    subject: Option<String>,
    lifetime_seconds: Option<u64>,
}

impl BearerEgressConfigBuilder {
    /// Create a new empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the algorithm and key material.
    pub fn secret(mut self, secret: BearerSecret) -> Self {
        self.secret = Some(secret);
        self
    }

    /// Set the JWT `iss` claim.
    pub fn issuer(mut self, issuer: impl Into<String>) -> Self {
        self.issuer = Some(issuer.into());
        self
    }

    /// Set the JWT `aud` claim.
    pub fn audience(mut self, audience: impl Into<String>) -> Self {
        self.audience = Some(audience.into());
        self
    }

    /// Set the JWT `sub` claim.
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// Set the token lifetime in seconds.
    pub fn lifetime_seconds(mut self, secs: u64) -> Self {
        self.lifetime_seconds = Some(secs);
        self
    }

    /// Build the [`BearerEgressConfig`].
    ///
    /// Returns `Err` if any required field has not been set.
    pub fn build(self) -> Result<BearerEgressConfig, String> {
        Ok(BearerEgressConfig {
            secret: self.secret.ok_or("secret is required")?,
            issuer: self.issuer.ok_or("issuer is required")?,
            audience: self.audience.ok_or("audience is required")?,
            subject: self.subject.ok_or("subject is required")?,
            lifetime_seconds: self
                .lifetime_seconds
                .ok_or("lifetime_seconds is required")?,
        })
    }
}

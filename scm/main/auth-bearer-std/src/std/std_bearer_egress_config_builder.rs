//! `StdBearerEgressConfigBuilder` -- the standard [`BearerEgressConfigBuilder`].

use grpc_egress_auth_bearer_api::{BearerEgressConfig, BearerEgressConfigBuilder, BearerSecret};

/// Fluent builder for [`BearerEgressConfig`]. Every field is required.
#[derive(Debug, Default)]
pub(crate) struct StdBearerEgressConfigBuilder {
    secret: Option<BearerSecret>,
    issuer: Option<String>,
    audience: Option<String>,
    subject: Option<String>,
    lifetime_seconds: Option<u64>,
}

impl BearerEgressConfigBuilder for StdBearerEgressConfigBuilder {
    fn secret(mut self, secret: BearerSecret) -> Self {
        self.secret = Some(secret);
        self
    }

    fn issuer(mut self, issuer: impl Into<String>) -> Self {
        self.issuer = Some(issuer.into());
        self
    }

    fn audience(mut self, audience: impl Into<String>) -> Self {
        self.audience = Some(audience.into());
        self
    }

    fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    fn lifetime_seconds(mut self, secs: u64) -> Self {
        self.lifetime_seconds = Some(secs);
        self
    }

    fn build(self) -> Result<BearerEgressConfig, String> {
        BearerEgressConfig::try_new(
            self.secret.ok_or("secret is required")?,
            self.issuer.ok_or("issuer is required")?,
            self.audience.ok_or("audience is required")?,
            self.subject.ok_or("subject is required")?,
            self.lifetime_seconds
                .ok_or("lifetime_seconds is required")?,
        )
        .map_err(|error| error.to_string())
    }
}

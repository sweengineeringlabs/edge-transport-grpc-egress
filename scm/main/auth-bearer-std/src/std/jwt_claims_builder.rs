//! Builder for [`JwtClaims`].

use swe_edge_security::JwtClaims;

/// Fluent builder for [`JwtClaims`].
#[derive(Debug, Default)]
pub struct JwtClaimsBuilder {
    iss: Option<String>,
    aud: Option<String>,
    sub: Option<String>,
    exp: Option<u64>,
    iat: Option<u64>,
}

impl JwtClaimsBuilder {
    /// Create a new empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the `iss` (issuer) claim.
    pub fn iss(mut self, iss: impl Into<String>) -> Self {
        self.iss = Some(iss.into());
        self
    }

    /// Set the `aud` (audience) claim.
    pub fn aud(mut self, aud: impl Into<String>) -> Self {
        self.aud = Some(aud.into());
        self
    }

    /// Set the `sub` (subject) claim.
    #[allow(clippy::should_implement_trait)]
    pub fn sub(mut self, sub: impl Into<String>) -> Self {
        self.sub = Some(sub.into());
        self
    }

    /// Set the `exp` (expiration) claim — seconds since Unix epoch.
    pub fn exp(mut self, exp: u64) -> Self {
        self.exp = Some(exp);
        self
    }

    /// Set the `iat` (issued-at) claim — seconds since Unix epoch.
    pub fn iat(mut self, iat: u64) -> Self {
        self.iat = Some(iat);
        self
    }

    /// Build the [`JwtClaims`].
    ///
    /// Returns `Err` if any required field has not been set.
    pub fn build(self) -> Result<JwtClaims, String> {
        Ok(JwtClaims {
            iss: self.iss.ok_or("iss is required")?,
            aud: self.aud.ok_or("aud is required")?,
            sub: self.sub.ok_or("sub is required")?,
            exp: self.exp.ok_or("exp is required")?,
            iat: self.iat.ok_or("iat is required")?,
        })
    }
}

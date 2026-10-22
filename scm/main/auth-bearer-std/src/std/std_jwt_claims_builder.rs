//! `StdJwtClaimsBuilder` -- the standard [`JwtClaimsBuilder`].

use grpc_egress_auth_bearer::{JwtClaims, JwtClaimsBuilder};

/// Fluent builder for [`JwtClaims`]. Every claim is required.
#[derive(Debug, Default)]
pub(crate) struct StdJwtClaimsBuilder {
    iss: Option<String>,
    aud: Option<String>,
    sub: Option<String>,
    exp: Option<u64>,
    iat: Option<u64>,
}

impl JwtClaimsBuilder for StdJwtClaimsBuilder {
    fn iss(mut self, iss: impl Into<String>) -> Self {
        self.iss = Some(iss.into());
        self
    }

    fn aud(mut self, aud: impl Into<String>) -> Self {
        self.aud = Some(aud.into());
        self
    }

    fn subject(mut self, sub: impl Into<String>) -> Self {
        self.sub = Some(sub.into());
        self
    }

    fn exp(mut self, exp: u64) -> Self {
        self.exp = Some(exp);
        self
    }

    fn iat(mut self, iat: u64) -> Self {
        self.iat = Some(iat);
        self
    }

    fn build(self) -> Result<JwtClaims, String> {
        Ok(JwtClaims {
            iss: self.iss.ok_or("iss is required")?,
            aud: self.aud.ok_or("aud is required")?,
            sub: self.sub.ok_or("sub is required")?,
            exp: self.exp.ok_or("exp is required")?,
            iat: self.iat.ok_or("iat is required")?,
        })
    }
}

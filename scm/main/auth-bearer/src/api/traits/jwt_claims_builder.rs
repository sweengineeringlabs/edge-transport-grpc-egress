//! `JwtClaimsBuilder` -- the fluent construction of [`JwtClaims`].

use crate::JwtClaims;

/// Collects the claims of a token and builds the [`JwtClaims`]. Every claim is required.
pub trait JwtClaimsBuilder: Sized {
    /// Set the `iss` (issuer) claim.
    fn iss(self, iss: impl Into<String>) -> Self;

    /// Set the `aud` (audience) claim.
    fn aud(self, aud: impl Into<String>) -> Self;

    /// Set the `sub` (subject) claim.
    fn subject(self, sub: impl Into<String>) -> Self;

    /// Set the `exp` (expiration) claim -- seconds since Unix epoch.
    fn exp(self, exp: u64) -> Self;

    /// Set the `iat` (issued-at) claim -- seconds since Unix epoch.
    fn iat(self, iat: u64) -> Self;

    /// Build the [`JwtClaims`].
    ///
    /// # Errors
    ///
    /// A text naming the first required claim that has not been set.
    fn build(self) -> Result<JwtClaims, String>;
}

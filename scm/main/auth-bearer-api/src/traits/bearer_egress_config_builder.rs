//! `BearerEgressConfigBuilder` -- the fluent construction of a [`BearerEgressConfig`].

use crate::{BearerEgressConfig, BearerSecret};

/// Collects the fields of the outbound bearer config and builds the [`BearerEgressConfig`], which is plain data in
/// this crate. Every field is required.
pub trait BearerEgressConfigBuilder: Sized {
    /// Set the algorithm and key material.
    fn secret(self, secret: BearerSecret) -> Self;

    /// Set the JWT `iss` claim.
    fn issuer(self, issuer: impl Into<String>) -> Self;

    /// Set the JWT `aud` claim.
    fn audience(self, audience: impl Into<String>) -> Self;

    /// Set the JWT `sub` claim.
    fn subject(self, subject: impl Into<String>) -> Self;

    /// Set the token lifetime in seconds.
    fn lifetime_seconds(self, secs: u64) -> Self;

    /// Build the [`BearerEgressConfig`].
    ///
    /// # Errors
    ///
    /// A text naming the first required field that has not been set.
    fn build(self) -> Result<BearerEgressConfig, String>;
}

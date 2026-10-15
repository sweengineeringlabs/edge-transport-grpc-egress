//! Validation contract for bearer auth types.

use super::super::error::BearerAuthError;
use super::super::types::BearerEgressConfig;

/// Validation contract — implementors check their own invariants.
///
/// Implemented by [`BearerEgressConfig`] to verify that required fields
/// (issuer, audience, subject, secret) are populated before the interceptor is used.
pub trait Validator: Send + Sync {
    /// Validate the receiver's invariants.
    ///
    /// Returns `Ok(())` when all invariants hold, or an `Err(BearerAuthError)` describing the
    /// first violation found.
    fn validate(&self) -> Result<(), BearerAuthError>;

    /// Create a validator instance from a config.
    ///
    /// This method ensures [`BearerEgressConfig`] appears in the trait signature.
    fn from_config(config: BearerEgressConfig) -> Self
    where
        Self: Sized;
}

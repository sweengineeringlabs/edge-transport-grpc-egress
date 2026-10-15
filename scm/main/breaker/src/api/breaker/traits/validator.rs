//! `Validator` trait — configuration validation contract.

use crate::api::BreakerDomainError;
use crate::api::ConfigValidationRequest;

/// Validation contract for circuit-breaker configuration.
pub trait Validator: Send + Sync {
    /// Validate the breaker configuration.
    fn validate(&self, req: ConfigValidationRequest) -> Result<(), BreakerDomainError>;
}

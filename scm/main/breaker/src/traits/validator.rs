//! `Validator` trait — configuration validation contract.

use crate::BreakerDomainError;
use crate::ConfigValidationRequest;

/// Validation contract for circuit-breaker configuration.
pub trait Validator: Send + Sync {
    /// Validate the breaker configuration.
    fn validate(&self, req: ConfigValidationRequest) -> Result<(), BreakerDomainError>;
}

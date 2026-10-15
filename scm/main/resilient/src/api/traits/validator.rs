//! `Validator` trait — configuration validation contract.

use crate::api::ConfigValidationRequest;
use crate::api::ResilientTransportError;

/// Configuration validation contract for the resilient transport.
pub trait Validator: Send + Sync {
    /// Validate the given resilience policy.
    fn validate(&self, req: ConfigValidationRequest) -> Result<(), ResilientTransportError>;
}

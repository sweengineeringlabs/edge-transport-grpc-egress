//! `Validator` trait — configuration validation contract.

use crate::api::Error;
use crate::api::ValidationRequest;

/// Validates a [`crate::api::GrpcRetryConfig`] for correctness.
///
/// Implemented by `DefaultProcessor`
/// in `core/`.
pub trait Validator {
    /// Check that all numeric fields are within their valid ranges.
    fn validate_config(&self, req: ValidationRequest) -> Result<(), Error>;
}

//! Request for [`crate::Validator::validate`].

use crate::GrpcBreakerConfig;

/// Input to [`crate::Validator::validate`] — the config to check.
pub struct ConfigValidationRequest {
    /// The breaker policy to validate.
    pub config: GrpcBreakerConfig,
}

//! Request for [`crate::api::Validator::validate`].

use crate::api::GrpcBreakerConfig;

/// Input to [`crate::api::Validator::validate`] — the config to check.
pub struct ConfigValidationRequest {
    /// The breaker policy to validate.
    pub config: GrpcBreakerConfig,
}

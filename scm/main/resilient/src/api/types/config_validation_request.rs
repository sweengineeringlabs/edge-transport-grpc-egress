//! Request for [`crate::api::Validator::validate`].

use crate::api::ResilienceConfig;

/// Input to [`crate::api::Validator::validate`] — the resilience policy to check.
pub struct ConfigValidationRequest {
    /// The resilience policy to validate.
    pub config: ResilienceConfig,
}

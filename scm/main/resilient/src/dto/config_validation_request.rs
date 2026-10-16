//! Request for [`crate::Validator::validate`].

use crate::ResilienceConfig;

/// Input to [`crate::Validator::validate`] — the resilience policy to check.
pub struct ConfigValidationRequest {
    /// The resilience policy to validate.
    pub config: ResilienceConfig,
}

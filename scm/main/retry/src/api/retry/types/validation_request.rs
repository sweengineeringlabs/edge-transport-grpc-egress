//! Request for [`crate::api::Validator::validate_config`].

use crate::api::GrpcRetryConfig;

/// Input to [`crate::api::Validator::validate_config`] — the config to check.
pub struct ValidationRequest {
    /// The retry policy to validate.
    pub config: GrpcRetryConfig,
}

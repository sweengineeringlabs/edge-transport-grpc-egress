//! Request for [`crate::Validator::validate_config`].

use crate::GrpcRetryConfig;

/// Input to [`crate::Validator::validate_config`] — the config to check.
pub struct ValidationRequest {
    /// The retry policy to validate.
    pub config: GrpcRetryConfig,
}

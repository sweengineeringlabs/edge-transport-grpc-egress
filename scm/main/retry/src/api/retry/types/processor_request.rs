//! Request for [`crate::api::Processor::validate`].

use crate::api::GrpcRetryConfig;

/// Input to [`crate::api::Processor::validate`] — the config to check.
pub struct ProcessorRequest {
    /// The retry policy to validate.
    pub config: GrpcRetryConfig,
}

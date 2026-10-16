//! Request for [`crate::Processor::validate`].

use crate::GrpcRetryConfig;

/// Input to [`crate::Processor::validate`] — the config to check.
pub struct ProcessorRequest {
    /// The retry policy to validate.
    pub config: GrpcRetryConfig,
}

//! Request for [`crate::RetryDecorator::describe_policy`].

use crate::GrpcRetryConfig;

/// Input to [`crate::RetryDecorator::describe_policy`] — the config to summarize.
pub struct DescribePolicyRequest {
    /// The retry policy to describe.
    pub config: GrpcRetryConfig,
}

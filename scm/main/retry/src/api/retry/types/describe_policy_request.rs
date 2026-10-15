//! Request for [`crate::api::RetryDecorator::describe_policy`].

use crate::api::GrpcRetryConfig;

/// Input to [`crate::api::RetryDecorator::describe_policy`] — the config to summarize.
pub struct DescribePolicyRequest {
    /// The retry policy to describe.
    pub config: GrpcRetryConfig,
}

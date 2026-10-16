//! `RetryDecorator` trait — retry-client construction contract.

use crate::DescribePolicyRequest;
use crate::DescribePolicyResponse;
use crate::Error;

/// Interface for producing a [`GrpcRetryClient`] decorator around an inner egress.
///
/// Implemented by `DefaultRetryDecorator`
/// in `core/`.
pub trait RetryDecorator: Send + Sync {
    /// Summarize the configured retry policy for logging/observability.
    fn describe_policy(&self, req: DescribePolicyRequest) -> Result<DescribePolicyResponse, Error>;
}

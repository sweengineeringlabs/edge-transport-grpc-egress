//! `RetryDecorator` trait — retry-client construction contract.

use crate::api::DescribePolicyRequest;
use crate::api::DescribePolicyResponse;
use crate::api::Error;
use crate::api::GrpcRetryClient;
use crate::api::GrpcRetryConfig;
use crate::api::GrpcRetryConfigBuilder;

/// Interface for producing a [`GrpcRetryClient`] decorator around an inner egress.
///
/// Implemented by `DefaultRetryDecorator`
/// in `core/`.
pub trait RetryDecorator: Send + Sync {
    /// Summarize the configured retry policy for logging/observability.
    fn describe_policy(&self, req: DescribePolicyRequest) -> Result<DescribePolicyResponse, Error>;

    /// Construct a [`GrpcRetryClient`] wrapping `inner` — gives it a
    /// genuine role in this trait's signature set, not just an impl-site
    /// concrete type built elsewhere. `Self: Sized` keeps this trait
    /// dyn-compatible for `Box<dyn Trait>`.
    fn default_client<T>(inner: T, config: GrpcRetryConfig) -> GrpcRetryClient<T>
    where
        Self: Sized,
    {
        GrpcRetryClient::new(inner, config)
    }

    /// Start a fluent [`GrpcRetryConfigBuilder`] for programmatic policy
    /// construction — gives it a genuine role in this trait's signature
    /// set, not just an impl-site helper.
    fn default_config_builder() -> GrpcRetryConfigBuilder
    where
        Self: Sized,
    {
        GrpcRetryConfigBuilder::new()
    }
}

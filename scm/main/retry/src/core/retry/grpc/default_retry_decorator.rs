//! `impl RetryDecorator for DefaultRetryDecorator`.

use tracing::trace;

use crate::api::DescribePolicyRequest;
use crate::api::DescribePolicyResponse;
use crate::api::Error;
use crate::api::RetryDecorator;
use crate::api::GRPC_RETRY_CLIENT_LOG_TARGET;

/// Default [`RetryDecorator`] implementation.
pub(crate) struct DefaultRetryDecorator;

impl RetryDecorator for DefaultRetryDecorator {
    fn describe_policy(&self, req: DescribePolicyRequest) -> Result<DescribePolicyResponse, Error> {
        let summary = format!(
            "max_attempts={} initial_backoff_ms={} rate_limit_max_attempts={}",
            req.config.max_attempts,
            req.config.initial_backoff_ms,
            req.config.rate_limit_max_attempts,
        );
        trace!(
            target: GRPC_RETRY_CLIENT_LOG_TARGET,
            %summary,
            "grpc-retry: policy described",
        );
        Ok(DescribePolicyResponse { summary })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::GrpcRetryConfig;

    /// @covers: describe_policy
    #[test]
    fn test_describe_policy_includes_configured_max_attempts() {
        let cfg = GrpcRetryConfig {
            max_attempts: 9,
            ..GrpcRetryConfig::default()
        };
        let resp = DefaultRetryDecorator
            .describe_policy(DescribePolicyRequest { config: cfg })
            .expect("infallible");
        assert!(resp.summary.contains("max_attempts=9"));
        // Sibling negative-shape check: a differently-configured input
        // must not produce the same summary string.
        let other = DefaultRetryDecorator
            .describe_policy(DescribePolicyRequest {
                config: GrpcRetryConfig::default(),
            })
            .expect("infallible");
        assert_ne!(resp.summary, other.summary);
    }
}

#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`DescribePolicyRequest`].

use edge_transport_grpc_egress_retry::{
    DescribePolicyRequest, GrpcRetryConfig, RetryDecoratorFactory,
};

/// @covers: DescribePolicyRequest
#[test]
fn test_describe_policy_request_used_by_real_decorator_happy() {
    let decorator = RetryDecoratorFactory::create();
    let resp = decorator
        .describe_policy(DescribePolicyRequest {
            config: GrpcRetryConfig::default(),
        })
        .expect("real decorator must accept this request type");
    assert!(resp.summary.contains("max_attempts"));
}

/// @covers: DescribePolicyRequest
#[test]
fn test_describe_policy_request_reflects_configured_value_error() {
    // "error"-flavored scenario for an infallible summary: prove the
    // request's config actually drives the output, not a hardcoded string.
    let decorator = RetryDecoratorFactory::create();
    let cfg = GrpcRetryConfig {
        max_attempts: 6,
        ..GrpcRetryConfig::default()
    };
    let resp = decorator
        .describe_policy(DescribePolicyRequest { config: cfg })
        .expect("real decorator must accept this request type");
    assert!(resp.summary.contains("max_attempts=6"));
}

/// @covers: DescribePolicyRequest
#[test]
fn test_describe_policy_request_reusable_edge() {
    let decorator = RetryDecoratorFactory::create();
    let resp1 = decorator
        .describe_policy(DescribePolicyRequest {
            config: GrpcRetryConfig::default(),
        })
        .expect("first request");
    let resp2 = decorator
        .describe_policy(DescribePolicyRequest {
            config: GrpcRetryConfig::default(),
        })
        .expect("second request");
    assert_eq!(resp1.summary, resp2.summary);
}

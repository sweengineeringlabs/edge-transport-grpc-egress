#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`RetryDecoratorFactory`].

use edge_transport_grpc_egress_retry::{
    DescribePolicyRequest, GrpcRetryConfig, RetryDecoratorFactory,
};

/// @covers: create
#[test]
fn test_create_produces_a_working_decorator_happy() {
    let decorator = RetryDecoratorFactory::create();
    let resp = decorator
        .describe_policy(DescribePolicyRequest {
            config: GrpcRetryConfig::default(),
        })
        .expect("factory-produced decorator must succeed");
    assert!(!resp.summary.is_empty());
}

/// @covers: create
#[test]
fn test_create_produces_independent_instances_error() {
    let first = RetryDecoratorFactory::create();
    let second = RetryDecoratorFactory::create();
    let resp1 = first
        .describe_policy(DescribePolicyRequest {
            config: GrpcRetryConfig::default(),
        })
        .expect("first must succeed");
    let resp2 = second
        .describe_policy(DescribePolicyRequest {
            config: GrpcRetryConfig::default(),
        })
        .expect("second must succeed");
    assert_eq!(resp1.summary, resp2.summary);
}

/// @covers: create
#[test]
fn test_create_repeated_requests_edge() {
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
        .expect("second request on same decorator instance");
    assert_eq!(resp1.summary, resp2.summary);
}

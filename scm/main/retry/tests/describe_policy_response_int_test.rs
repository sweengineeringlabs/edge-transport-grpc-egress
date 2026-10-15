#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`DescribePolicyResponse`].

use edge_transport_grpc_egress_retry::{
    DescribePolicyRequest, GrpcRetryConfig, RetryDecoratorFactory,
};

/// @covers: DescribePolicyResponse
#[test]
fn test_describe_policy_response_produced_by_real_decorator_happy() {
    let decorator = RetryDecoratorFactory::create();
    let resp = decorator
        .describe_policy(DescribePolicyRequest {
            config: GrpcRetryConfig::default(),
        })
        .expect("real decorator must produce this response type");
    assert!(!resp.summary.is_empty());
}

/// @covers: DescribePolicyResponse
#[test]
fn test_describe_policy_response_differs_for_different_configs_error() {
    let decorator = RetryDecoratorFactory::create();
    let cfg_a = GrpcRetryConfig {
        max_attempts: 2,
        ..GrpcRetryConfig::default()
    };
    let cfg_b = GrpcRetryConfig {
        max_attempts: 8,
        ..GrpcRetryConfig::default()
    };
    let resp_a = decorator
        .describe_policy(DescribePolicyRequest { config: cfg_a })
        .expect("first config");
    let resp_b = decorator
        .describe_policy(DescribePolicyRequest { config: cfg_b })
        .expect("second config");
    assert_ne!(resp_a.summary, resp_b.summary);
}

/// @covers: DescribePolicyResponse
#[test]
fn test_describe_policy_response_equality_is_by_value_edge() {
    let a = edge_transport_grpc_egress_retry::DescribePolicyResponse {
        summary: "same".to_string(),
    };
    let b = edge_transport_grpc_egress_retry::DescribePolicyResponse {
        summary: "same".to_string(),
    };
    assert_eq!(a, b);
}

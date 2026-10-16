#![allow(clippy::unwrap_used, clippy::expect_used)]
//! End-to-end tests for [`RetryDecorator`] via a test-double implementation.

use grpc_egress_retry::{DescribePolicyRequest, Error, GrpcRetryConfig, RetryDecorator};

struct MockDecorator {
    fail: bool,
}

impl RetryDecorator for MockDecorator {
    fn describe_policy(
        &self,
        req: DescribePolicyRequest,
    ) -> Result<grpc_egress_retry::DescribePolicyResponse, Error> {
        if self.fail {
            return Err(Error::InvalidConfig("mock decorator forced failure".into()));
        }
        Ok(grpc_egress_retry::DescribePolicyResponse {
            summary: format!("max_attempts={}", req.config.max_attempts),
        })
    }
}

/// @covers: describe_policy
#[test]
fn test_describe_policy_includes_max_attempts_happy() {
    let decorator = MockDecorator { fail: false };
    let cfg = GrpcRetryConfig {
        max_attempts: 4,
        ..GrpcRetryConfig::default()
    };
    let resp = decorator
        .describe_policy(DescribePolicyRequest { config: cfg })
        .expect("happy path");
    assert_eq!(resp.summary, "max_attempts=4");
}

/// @covers: describe_policy
#[test]
fn test_describe_policy_propagates_failure_error() {
    let decorator = MockDecorator { fail: true };
    let err = decorator
        .describe_policy(DescribePolicyRequest {
            config: GrpcRetryConfig::default(),
        })
        .expect_err("forced failure must surface");
    assert!(err.to_string().contains("mock decorator forced failure"));
}

/// @covers: describe_policy
#[test]
fn test_describe_policy_zero_max_attempts_edge() {
    let decorator = MockDecorator { fail: false };
    let cfg = GrpcRetryConfig {
        max_attempts: 0,
        ..GrpcRetryConfig::default()
    };
    let resp = decorator
        .describe_policy(DescribePolicyRequest { config: cfg })
        .expect("happy path");
    assert_eq!(resp.summary, "max_attempts=0");
}

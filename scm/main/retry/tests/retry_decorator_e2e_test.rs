#![allow(clippy::unwrap_used, clippy::expect_used)]
//! End-to-end tests for [`RetryDecorator`] via a test-double implementation.

use edge_transport_grpc_egress_retry::{
    DescribePolicyRequest, Error, GrpcRetryConfig, RetryDecorator,
};

struct MockDecorator {
    fail: bool,
}

impl RetryDecorator for MockDecorator {
    fn describe_policy(
        &self,
        req: DescribePolicyRequest,
    ) -> Result<edge_transport_grpc_egress_retry::DescribePolicyResponse, Error> {
        if self.fail {
            return Err(Error::InvalidConfig("mock decorator forced failure".into()));
        }
        Ok(edge_transport_grpc_egress_retry::DescribePolicyResponse {
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

/// @covers: default_client
#[test]
fn test_default_client_wraps_inner_with_given_config_happy() {
    let client =
        <MockDecorator as RetryDecorator>::default_client("inner", GrpcRetryConfig::default());
    assert_eq!(
        client.config().max_attempts,
        GrpcRetryConfig::default().max_attempts
    );
}

/// @covers: default_client
#[test]
fn test_default_client_applies_supplied_config_error() {
    // "error"-flavored scenario for an infallible constructor: prove it
    // does NOT silently substitute a default config when one is supplied.
    let cfg = GrpcRetryConfig {
        max_attempts: 17,
        ..GrpcRetryConfig::default()
    };
    let client = <MockDecorator as RetryDecorator>::default_client("inner", cfg);
    assert_eq!(client.config().max_attempts, 17);
}

/// @covers: default_client
#[test]
fn test_default_client_zero_max_attempts_config_edge() {
    let cfg = GrpcRetryConfig {
        max_attempts: 0,
        ..GrpcRetryConfig::default()
    };
    let client = <MockDecorator as RetryDecorator>::default_client("inner", cfg);
    assert_eq!(client.config().max_attempts, 0);
}

/// @covers: default_config_builder
#[test]
fn test_default_config_builder_builds_valid_default_config_happy() {
    let builder = <MockDecorator as RetryDecorator>::default_config_builder();
    let cfg = builder.build().expect("SWE baseline must be valid");
    assert_eq!(cfg.max_attempts, GrpcRetryConfig::default().max_attempts);
}

/// @covers: default_config_builder
#[test]
fn test_default_config_builder_rejects_invalid_override_error() {
    let builder = <MockDecorator as RetryDecorator>::default_config_builder().max_attempts(0);
    let err = builder.build().expect_err("zero max_attempts is invalid");
    assert!(err.to_string().contains("max_attempts"));
}

/// @covers: default_config_builder
#[test]
fn test_default_config_builder_applies_fluent_override_edge() {
    let cfg = <MockDecorator as RetryDecorator>::default_config_builder()
        .max_attempts(9)
        .build()
        .expect("valid override");
    assert_eq!(cfg.max_attempts, 9);
}

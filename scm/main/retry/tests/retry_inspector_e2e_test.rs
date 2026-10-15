#![allow(clippy::unwrap_used, clippy::expect_used)]
//! End-to-end tests for [`RetryInspector`] via a test-double implementation.

use edge_transport_grpc_egress_retry::{
    Error, ResourceExhaustedContext, RetryDecision, RetryInspectRequest, RetryInspector,
};

struct MockInspector {
    fail: bool,
}

impl RetryInspector for MockInspector {
    fn describe(
        &self,
        _req: RetryInspectRequest,
    ) -> Result<edge_transport_grpc_egress_retry::RetryInspectResponse, Error> {
        if self.fail {
            return Err(Error::InvalidConfig("mock inspector forced failure".into()));
        }
        Ok(edge_transport_grpc_egress_retry::RetryInspectResponse {
            label: "mock-inspector",
        })
    }
}

/// @covers: describe
#[test]
fn test_describe_returns_configured_label_happy() {
    let inspector = MockInspector { fail: false };
    let resp = inspector.describe(RetryInspectRequest).expect("happy path");
    assert_eq!(resp.label, "mock-inspector");
}

/// @covers: describe
#[test]
fn test_describe_propagates_failure_error() {
    let inspector = MockInspector { fail: true };
    let err = inspector
        .describe(RetryInspectRequest)
        .expect_err("forced failure must surface");
    assert!(err.to_string().contains("mock inspector forced failure"));
}

/// @covers: describe
#[test]
fn test_describe_repeated_calls_are_independent_edge() {
    let inspector = MockInspector { fail: false };
    let first = inspector.describe(RetryInspectRequest).expect("first call");
    let second = inspector
        .describe(RetryInspectRequest)
        .expect("second call");
    assert_eq!(first.label, second.label);
}

/// @covers: should_retry
#[test]
fn test_should_retry_true_for_retry_variant_happy() {
    assert!(<MockInspector as RetryInspector>::should_retry(
        RetryDecision::Retry
    ));
}

/// @covers: should_retry
#[test]
fn test_should_retry_false_for_terminal_variant_error() {
    // "error"-flavored scenario for a bool predicate: prove it doesn't
    // just always return true regardless of input.
    assert!(!<MockInspector as RetryInspector>::should_retry(
        RetryDecision::Terminal
    ));
}

/// @covers: should_retry
#[test]
fn test_should_retry_true_for_rate_limit_variant_edge() {
    assert!(<MockInspector as RetryInspector>::should_retry(
        RetryDecision::RetryRateLimit
    ));
}

/// @covers: classify_resource_exhausted
#[test]
fn test_classify_resource_exhausted_hard_quota_happy() {
    assert_eq!(
        <MockInspector as RetryInspector>::classify_resource_exhausted("billing quota exceeded"),
        ResourceExhaustedContext::HardQuota
    );
}

/// @covers: classify_resource_exhausted
#[test]
fn test_classify_resource_exhausted_unknown_message_error() {
    // "error"-flavored scenario: a message with none of the known
    // keywords must fall back to the safe default (Capacity, which
    // triggers a retry) rather than misclassifying as HardQuota (which
    // would silently stop retries).
    assert_eq!(
        <MockInspector as RetryInspector>::classify_resource_exhausted(
            "unrecognized backend error"
        ),
        ResourceExhaustedContext::Capacity
    );
}

/// @covers: classify_resource_exhausted
#[test]
fn test_classify_resource_exhausted_empty_message_edge() {
    assert_eq!(
        <MockInspector as RetryInspector>::classify_resource_exhausted(""),
        ResourceExhaustedContext::Capacity
    );
}

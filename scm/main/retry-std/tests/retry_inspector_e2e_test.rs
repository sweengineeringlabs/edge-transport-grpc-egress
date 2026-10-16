#![allow(clippy::unwrap_used, clippy::expect_used)]
//! End-to-end tests for [`RetryInspector`] via a test-double implementation.

use grpc_egress_retry::{Error, RetryInspectRequest, RetryInspector};

struct MockInspector {
    fail: bool,
}

impl RetryInspector for MockInspector {
    fn describe(
        &self,
        _req: RetryInspectRequest,
    ) -> Result<grpc_egress_retry::RetryInspectResponse, Error> {
        if self.fail {
            return Err(Error::InvalidConfig("mock inspector forced failure".into()));
        }
        Ok(grpc_egress_retry::RetryInspectResponse {
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

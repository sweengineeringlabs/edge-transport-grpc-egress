#![allow(clippy::unwrap_used, clippy::expect_used)]
//! End-to-end tests for [`Processor`] via a test-double implementation.

use edge_transport_grpc_egress_resilient::{
    DescribeRequest, DescribeResponse, Processor, ResilientTransportError,
};

struct MockProcessor {
    label: &'static str,
    fail: bool,
}

impl Processor for MockProcessor {
    fn describe(&self, _req: DescribeRequest) -> Result<DescribeResponse, ResilientTransportError> {
        if self.fail {
            return Err(ResilientTransportError::InvalidResilience(
                "mock processor forced failure".into(),
            ));
        }
        Ok(DescribeResponse { label: self.label })
    }
}

/// @covers: describe
#[test]
fn test_describe_returns_configured_label_happy() {
    let processor = MockProcessor {
        label: "mock-processor",
        fail: false,
    };
    let resp = processor.describe(DescribeRequest).expect("happy path");
    assert_eq!(resp.label, "mock-processor");
}

/// @covers: describe
#[test]
fn test_describe_propagates_failure_error() {
    let processor = MockProcessor {
        label: "mock-processor",
        fail: true,
    };
    let err = processor
        .describe(DescribeRequest)
        .expect_err("forced failure must surface");
    assert!(err.to_string().contains("mock processor forced failure"));
}

/// @covers: describe
#[test]
fn test_describe_empty_label_edge() {
    let processor = MockProcessor {
        label: "",
        fail: false,
    };
    let resp = processor
        .describe(DescribeRequest)
        .expect("empty label is valid");
    assert_eq!(resp.label, "");
}

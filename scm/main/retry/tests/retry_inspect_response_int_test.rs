#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`RetryInspectResponse`].

use edge_transport_grpc_egress_retry::{RetryInspectRequest, RetryInspectorFactory};

/// @covers: RetryInspectResponse
#[test]
fn test_retry_inspect_response_produced_by_real_inspector_happy() {
    let inspector = RetryInspectorFactory::create();
    let resp = inspector
        .describe(RetryInspectRequest)
        .expect("real inspector must produce this response type");
    assert!(!resp.label.is_empty());
}

/// @covers: RetryInspectResponse
#[test]
fn test_retry_inspect_response_deterministic_across_instances_error() {
    let a = RetryInspectorFactory::create();
    let b = RetryInspectorFactory::create();
    let resp_a = a.describe(RetryInspectRequest).expect("first instance");
    let resp_b = b.describe(RetryInspectRequest).expect("second instance");
    assert_eq!(resp_a.label, resp_b.label);
}

/// @covers: RetryInspectResponse
#[test]
fn test_retry_inspect_response_equality_is_by_value_edge() {
    let a = edge_transport_grpc_egress_retry::RetryInspectResponse { label: "same" };
    let b = edge_transport_grpc_egress_retry::RetryInspectResponse { label: "same" };
    assert_eq!(a, b);
}

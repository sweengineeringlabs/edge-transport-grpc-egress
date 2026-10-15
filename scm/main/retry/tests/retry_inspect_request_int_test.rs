#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`RetryInspectRequest`].

use edge_transport_grpc_egress_retry::{RetryInspectRequest, RetryInspectorFactory};

/// @covers: RetryInspectRequest
#[test]
fn test_retry_inspect_request_is_constructible_happy() {
    let req = RetryInspectRequest;
    assert_eq!(std::mem::size_of_val(&req), 0);
}

/// @covers: RetryInspectRequest
#[test]
fn test_retry_inspect_request_used_by_real_inspector_error() {
    let inspector = RetryInspectorFactory::create();
    let resp = inspector
        .describe(RetryInspectRequest)
        .expect("real inspector must accept this request type");
    assert!(!resp.label.is_empty());
}

/// @covers: RetryInspectRequest
#[test]
fn test_retry_inspect_request_reusable_edge() {
    let a = RetryInspectRequest;
    let b = RetryInspectRequest;
    assert_eq!(std::mem::size_of_val(&a), std::mem::size_of_val(&b));
}

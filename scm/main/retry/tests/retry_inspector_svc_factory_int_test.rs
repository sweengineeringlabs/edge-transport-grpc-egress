#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`RetryInspectorFactory`].

use edge_transport_grpc_egress_retry::{RetryInspectRequest, RetryInspectorFactory};

/// @covers: create
#[test]
fn test_create_produces_a_working_inspector_happy() {
    let inspector = RetryInspectorFactory::create();
    let resp = inspector
        .describe(RetryInspectRequest)
        .expect("factory-produced inspector must succeed");
    assert!(!resp.label.is_empty());
}

/// @covers: create
#[test]
fn test_create_produces_independent_instances_error() {
    let first = RetryInspectorFactory::create();
    let second = RetryInspectorFactory::create();
    let resp1 = first
        .describe(RetryInspectRequest)
        .expect("first must succeed");
    let resp2 = second
        .describe(RetryInspectRequest)
        .expect("second must succeed");
    assert_eq!(resp1.label, resp2.label);
}

/// @covers: create
#[test]
fn test_create_repeated_requests_edge() {
    let inspector = RetryInspectorFactory::create();
    let resp1 = inspector
        .describe(RetryInspectRequest)
        .expect("first request");
    let resp2 = inspector
        .describe(RetryInspectRequest)
        .expect("second request on same inspector instance");
    assert_eq!(resp1.label, resp2.label);
}

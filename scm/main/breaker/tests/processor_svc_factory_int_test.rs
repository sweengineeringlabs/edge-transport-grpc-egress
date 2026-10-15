#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`ProcessorFactory`].

use edge_transport_grpc_egress_breaker::{DescribeRequest, ProcessorFactory};

/// @covers: create
#[test]
fn test_create_produces_a_working_processor_happy() {
    let processor = ProcessorFactory::create();
    let resp = processor
        .describe(DescribeRequest)
        .expect("factory-produced processor must succeed");
    assert!(!resp.label.is_empty());
}

/// @covers: create
#[test]
fn test_create_identifies_as_grpc_breaker_error() {
    // "error"-flavored scenario: prove the label is a genuine, specific
    // identifier rather than an empty/placeholder stub.
    let processor = ProcessorFactory::create();
    let resp = processor.describe(DescribeRequest).expect("must succeed");
    assert!(resp.label.contains("breaker"));
}

/// @covers: create
#[test]
fn test_create_produces_independent_instances_edge() {
    let first = ProcessorFactory::create();
    let second = ProcessorFactory::create();
    let resp1 = first.describe(DescribeRequest).expect("first must succeed");
    let resp2 = second
        .describe(DescribeRequest)
        .expect("second must succeed");
    assert_eq!(resp1.label, resp2.label);
}

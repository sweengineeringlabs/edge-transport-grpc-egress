#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`DescribeRequest`].

use edge_transport_grpc_egress_resilient::{DescribeRequest, GrpcResilientSvcProcessor, Processor};

/// @covers: DescribeRequest
#[test]
fn test_describe_request_is_constructible_happy() {
    let req = DescribeRequest;
    assert_eq!(std::mem::size_of_val(&req), 0);
}

/// @covers: DescribeRequest
#[test]
fn test_describe_request_used_by_real_processor_error() {
    let resp = GrpcResilientSvcProcessor
        .describe(DescribeRequest)
        .expect("real processor must accept this request type");
    assert!(!resp.label.is_empty());
}

/// @covers: DescribeRequest
#[test]
fn test_describe_request_reusable_edge() {
    let a = DescribeRequest;
    let b = DescribeRequest;
    assert_eq!(std::mem::size_of_val(&a), std::mem::size_of_val(&b));
}

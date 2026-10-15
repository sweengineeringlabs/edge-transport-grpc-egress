#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for the `Processor` trait contract on `GrpcBreakerSvc`.

use edge_transport_grpc_egress_breaker::{DescribeRequest, GrpcBreakerSvc, Processor};

/// @covers: Processor — trait is object-safe
#[test]
fn breaker_trait_processor_is_object_safe_int_test() {
    fn _assert(_: &dyn Processor) {}
}

/// @covers: Processor::describe — returns expected label
#[test]
fn breaker_struct_grpc_breaker_svc_describe_returns_label_int_test() {
    let svc = GrpcBreakerSvc;
    let resp = svc.describe(DescribeRequest).expect("infallible");
    assert_eq!(resp.label, "grpc-breaker");
}

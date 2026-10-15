#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for the `Processor` trait contract on `GrpcResilientSvcProcessor`.

use edge_transport_grpc_egress_resilient::{DescribeRequest, GrpcResilientSvcProcessor, Processor};

/// @covers: Processor — trait is object-safe
#[test]
fn resilient_trait_processor_is_object_safe_int_test() {
    fn _assert(_: &dyn Processor) {}
}

/// @covers: Processor::describe — returns expected label
#[test]
fn resilient_struct_grpc_resilient_svc_describe_returns_label_int_test() {
    let resp = GrpcResilientSvcProcessor
        .describe(DescribeRequest)
        .expect("infallible");
    assert_eq!(resp.label, "grpc-resilient");
}

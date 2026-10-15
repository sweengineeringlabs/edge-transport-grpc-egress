//! Integration tests for [`DescribeResponse`].

use edge_transport_grpc_egress_resilient::DescribeResponse;

/// @covers: DescribeResponse
#[test]
fn test_describe_response_preserves_label_happy() {
    let resp = DescribeResponse {
        label: "grpc-resilient",
    };
    assert_eq!(resp.label, "grpc-resilient");
}

/// @covers: DescribeResponse
#[test]
fn test_describe_response_empty_label_error() {
    let resp = DescribeResponse { label: "" };
    assert_eq!(resp.label, "");
}

/// @covers: DescribeResponse
#[test]
fn test_describe_response_equality_edge() {
    let a = DescribeResponse { label: "same" };
    let b = DescribeResponse { label: "same" };
    assert_eq!(a, b);
}

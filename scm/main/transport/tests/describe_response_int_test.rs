//! Integration tests for `DescribeResponse`.

use edge_transport_grpc_egress_transport::DescribeResponse;

/// @covers: DescribeResponse
#[test]
fn test_describe_response_carries_label_happy() {
    let resp = DescribeResponse { label: "transport" };
    assert_eq!(resp.label, "transport");
}

/// @covers: DescribeResponse
#[test]
fn test_describe_response_distinguishes_labels_error() {
    let a = DescribeResponse { label: "a" };
    let b = DescribeResponse { label: "b" };
    assert_ne!(a, b);
}

/// @covers: DescribeResponse
#[test]
fn test_describe_response_empty_label_edge() {
    let resp = DescribeResponse { label: "" };
    assert_eq!(resp.label, "");
}

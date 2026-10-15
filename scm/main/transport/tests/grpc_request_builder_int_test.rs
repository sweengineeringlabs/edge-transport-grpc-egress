#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for `GrpcRequestBuilder`.

use edge_transport_grpc_egress_transport::GrpcRequestBuilder;
use std::collections::HashMap;
use std::time::Duration;

/// @covers: GrpcRequestBuilder::build — valid request returns Ok
#[test]
fn transport_struct_grpc_request_builder_build_valid_request_returns_ok_int_test() {
    let req = GrpcRequestBuilder::new()
        .method("svc/Method")
        .deadline(Duration::from_secs(5))
        .build()
        .expect("method and deadline set, build must succeed");
    assert_eq!(req.method, "svc/Method");
    assert_eq!(req.deadline, Duration::from_secs(5));
}

/// @covers: GrpcRequestBuilder::build — missing method returns Err
#[test]
fn transport_struct_grpc_request_builder_build_missing_method_returns_err_int_test() {
    assert!(GrpcRequestBuilder::new()
        .deadline(Duration::from_secs(1))
        .build()
        .is_err());
}

/// @covers: GrpcRequestBuilder::build — missing deadline returns Err
#[test]
fn transport_struct_grpc_request_builder_build_missing_deadline_returns_err_int_test() {
    assert!(GrpcRequestBuilder::new().method("svc/M").build().is_err());
}

/// @covers: GrpcRequestBuilder::body — stores bytes in built request
#[test]
fn transport_struct_grpc_request_builder_body_setter_stores_bytes_int_test() {
    let req = GrpcRequestBuilder::new()
        .method("svc/M")
        .deadline(Duration::from_secs(1))
        .body(vec![1, 2, 3])
        .build()
        .expect("build must succeed");
    assert_eq!(req.body, vec![1u8, 2, 3]);
}

/// @covers: GrpcRequestBuilder::deadline — stores deadline in built request
#[test]
fn transport_struct_grpc_request_builder_deadline_setter_stores_value_int_test() {
    let req = GrpcRequestBuilder::new()
        .method("svc/M")
        .deadline(Duration::from_secs(10))
        .build()
        .expect("build must succeed");
    assert_eq!(req.deadline, Duration::from_secs(10));
}

/// @covers: GrpcRequestBuilder::metadata — stores headers in built request
#[test]
fn transport_struct_grpc_request_builder_metadata_setter_stores_headers_int_test() {
    let mut meta = HashMap::new();
    meta.insert("x-test".into(), "value".into());
    let req = GrpcRequestBuilder::new()
        .method("svc/M")
        .deadline(Duration::from_secs(1))
        .metadata(meta)
        .build()
        .expect("build must succeed");
    assert_eq!(
        req.metadata.get("x-test").map(String::as_str),
        Some("value")
    );
}

/// @covers: GrpcRequestBuilder::cancellation_token — attaches token to built request
#[test]
fn transport_struct_grpc_request_builder_cancellation_token_setter_attaches_token_int_test() {
    use tokio_util::sync::CancellationToken;
    let token = CancellationToken::new();
    let req = GrpcRequestBuilder::new()
        .method("svc/M")
        .deadline(Duration::from_secs(1))
        .cancellation_token(token)
        .build()
        .expect("build must succeed");
    assert!(req.cancellation.is_some());
}

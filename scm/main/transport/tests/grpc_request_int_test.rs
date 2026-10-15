#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for `api/value/grpc/grpc_request.rs`.

use std::collections::HashMap;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use edge_transport_grpc_egress_transport::GrpcRequest;

#[test]
fn transport_struct_new_stores_method_body_and_deadline_int_test() {
    let d = Duration::from_secs(5);
    let req = GrpcRequest::new("pkg.Svc/Method", vec![0xAB], d);
    assert_eq!(req.method, "pkg.Svc/Method");
    assert_eq!(req.body, vec![0xAB]);
    assert_eq!(req.deadline, d);
    assert!(req.metadata.is_empty());
    assert!(req.cancellation.is_none());
}

#[test]
fn transport_struct_with_header_inserts_single_metadata_entry_int_test() {
    let req = GrpcRequest::new("svc/M", vec![], Duration::from_secs(1))
        .with_header("authorization", "Bearer tok");
    assert_eq!(
        req.metadata.get("authorization").map(String::as_str),
        Some("Bearer tok")
    );
}

#[test]
fn transport_struct_with_metadata_replaces_metadata_entirely_int_test() {
    let meta: HashMap<String, String> = [("k".to_string(), "v".to_string())].into_iter().collect();
    let req = GrpcRequest::new("svc/M", vec![], Duration::from_secs(1)).with_metadata(meta);
    assert_eq!(req.metadata.get("k").map(String::as_str), Some("v"));
}

#[test]
fn transport_struct_with_cancellation_attaches_token_int_test() {
    let token = CancellationToken::new();
    let req =
        GrpcRequest::new("svc/M", vec![], Duration::from_secs(1)).with_cancellation(token.clone());
    let stored = req.cancellation.as_ref().expect("token should be Some");
    assert!(!stored.is_cancelled());
    token.cancel();
    assert!(
        stored.is_cancelled(),
        "stored token must observe cancellation"
    );
}

#[test]
fn transport_struct_grpc_request_holds_method_body_and_deadline_via_struct_init_int_test() {
    let req = GrpcRequest {
        method: "svc/Method".into(),
        body: vec![1, 2],
        metadata: HashMap::new(),
        deadline: Duration::from_millis(250),
        cancellation: None,
    };
    assert_eq!(req.method, "svc/Method");
    assert_eq!(req.body, vec![1, 2]);
    assert_eq!(req.deadline, Duration::from_millis(250));
}

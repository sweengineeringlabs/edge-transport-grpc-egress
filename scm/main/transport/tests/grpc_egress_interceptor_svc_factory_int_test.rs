//! Integration tests for `GrpcEgressInterceptorFactory`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use edge_transport_grpc_egress_transport::{GrpcEgressInterceptorFactory, GrpcRequest};

/// @covers: create
#[test]
fn test_create_returns_pass_through_interceptor_happy() {
    let interceptor = GrpcEgressInterceptorFactory::create();
    let mut req = GrpcRequest::new("svc/Method", Vec::new(), Duration::from_secs(1));
    // A pass-through interceptor must not inject a traceparent when the
    // caller supplied none — proves this is the real pass-through variant,
    // not an arbitrary stub that always succeeds.
    interceptor
        .before_call(&mut req)
        .expect("pass-through interceptor must not reject a bare request");
    assert!(
        !req.metadata.contains_key("traceparent"),
        "pass-through must not inject traceparent metadata"
    );
}

/// @covers: create
#[test]
fn test_create_each_call_returns_independent_instance_edge() {
    let a = GrpcEgressInterceptorFactory::create();
    let b = GrpcEgressInterceptorFactory::create();
    // Two independently-created interceptors must both behave identically
    // (both pass-through) without sharing state.
    let mut req_a = GrpcRequest::new("svc/A", Vec::new(), Duration::from_secs(1));
    let mut req_b = GrpcRequest::new("svc/B", Vec::new(), Duration::from_secs(1));
    a.before_call(&mut req_a)
        .expect("interceptor a must accept the request");
    b.before_call(&mut req_b)
        .expect("interceptor b must accept the request");
    assert!(
        !req_a.metadata.contains_key("traceparent") && !req_b.metadata.contains_key("traceparent"),
        "both independently-created interceptors must be pass-through, injecting nothing"
    );
}

/// @covers: create
#[test]
fn test_create_preserves_upstream_traceparent_edge() {
    let interceptor = GrpcEgressInterceptorFactory::create();
    let mut req = GrpcRequest::new("svc/Method", Vec::new(), Duration::from_secs(1));
    req.metadata
        .insert("traceparent".to_string(), "existing-value".to_string());
    interceptor
        .before_call(&mut req)
        .expect("pass-through interceptor must accept a request with an upstream traceparent");
    assert_eq!(
        req.metadata.get("traceparent"),
        Some(&"existing-value".to_string()),
        "pass-through must not overwrite an upstream traceparent"
    );
}

#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for `TraceContextGrpcEgressInterceptor`.
//!
//! Tests that access the `pub(crate)` field `interceptor.source` are omitted.

use edge_transport_grpc_egress_transport::GrpcEgressInterceptor;
use edge_transport_grpc_egress_transport::{
    AfterCallRequest, GrpcRequest, GrpcResponse, TraceContextGrpcEgressInterceptor,
};
use std::collections::HashMap;
use std::time::Duration;

fn req() -> GrpcRequest {
    GrpcRequest::new("svc/M", vec![], Duration::from_secs(1))
}

/// @covers: TraceContextGrpcEgressInterceptor::pass_through — preserves an existing upstream traceparent unchanged
#[test]
fn transport_struct_trace_context_interceptor_pass_through_constructs_without_panic_int_test() {
    let upstream = "00-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa1-bbbbbbbbbbbbbbbb-01";
    let ic = TraceContextGrpcEgressInterceptor::pass_through();
    let mut r = req();
    r.metadata.insert("traceparent".into(), upstream.into());
    ic.before_call(&mut r).expect("before_call must not fail");
    assert_eq!(
        r.metadata.get("traceparent").map(String::as_str),
        Some(upstream),
        "pass_through must leave an existing upstream traceparent untouched"
    );
}

/// @covers: TraceContextGrpcEgressInterceptor::pass_through — does not inject traceparent when absent
#[test]
fn transport_struct_trace_context_interceptor_pass_through_does_not_inject_traceparent_int_test() {
    let ic = TraceContextGrpcEgressInterceptor::pass_through();
    let mut r = req();
    ic.before_call(&mut r).expect("before_call must not fail");
    assert!(
        !r.metadata.contains_key("traceparent"),
        "pass_through must not inject a traceparent header"
    );
}

/// @covers: TraceContextGrpcEgressInterceptor::with_static — injects traceparent when absent
#[test]
fn transport_struct_trace_context_interceptor_with_static_injects_traceparent_when_absent_int_test()
{
    let tp = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
    let ic = TraceContextGrpcEgressInterceptor::with_static(tp, None);
    let mut r = req();
    ic.before_call(&mut r).expect("before_call must not fail");
    assert_eq!(
        r.metadata.get("traceparent").map(String::as_str),
        Some(tp),
        "with_static must inject traceparent when absent"
    );
}

/// @covers: TraceContextGrpcEgressInterceptor::with_static — injects tracestate when configured
#[test]
fn transport_struct_trace_context_interceptor_with_static_injects_tracestate_when_configured_int_test(
) {
    let tp = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
    let ic = TraceContextGrpcEgressInterceptor::with_static(tp, Some("v=1".into()));
    let mut r = req();
    ic.before_call(&mut r).expect("before_call must not fail");
    assert_eq!(
        r.metadata.get("tracestate").map(String::as_str),
        Some("v=1"),
        "with_static must inject tracestate when configured"
    );
}

/// @covers: TraceContextGrpcEgressInterceptor::with_static — upstream traceparent is preserved
#[test]
fn transport_struct_trace_context_interceptor_upstream_traceparent_is_not_overwritten_int_test() {
    let upstream = "00-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa1-bbbbbbbbbbbbbbbb-01";
    let injected = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
    let ic = TraceContextGrpcEgressInterceptor::with_static(injected, None);
    let mut r = req();
    r.metadata.insert("traceparent".into(), upstream.into());
    ic.before_call(&mut r).expect("before_call must not fail");
    assert_eq!(
        r.metadata.get("traceparent").map(String::as_str),
        Some(upstream),
        "existing traceparent must not be overwritten"
    );
}

/// @covers: TraceContextGrpcEgressInterceptor::after_call — does not modify response
#[test]
fn transport_struct_trace_context_interceptor_after_call_does_not_modify_response_int_test() {
    let ic = TraceContextGrpcEgressInterceptor::pass_through();
    let mut resp = GrpcResponse {
        body: vec![],
        metadata: HashMap::new(),
    };
    ic.after_call(AfterCallRequest {
        response: &mut resp,
    })
    .expect("after_call must not fail");
    assert!(resp.metadata.is_empty());
}

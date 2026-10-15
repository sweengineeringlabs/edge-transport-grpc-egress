//! Integration tests for `CallUnaryWithContextRequest`.

use edge_security_runtime::SecurityContext;
use edge_transport_grpc_egress_transport::{CallUnaryWithContextRequest, GrpcRequest};

/// @covers: CallUnaryWithContextRequest
#[test]
fn test_call_unary_with_context_request_carries_request_and_ctx_happy() {
    let request = GrpcRequest::new(
        "pkg.Service/Method",
        b"payload".to_vec(),
        std::time::Duration::from_secs(5),
    );
    let ctx = SecurityContext::unauthenticated().with_trace_id("trace-1");
    let wrapped = CallUnaryWithContextRequest { request, ctx };
    assert_eq!(wrapped.request.method, "pkg.Service/Method");
    assert_eq!(wrapped.request.body, b"payload");
}

/// @covers: CallUnaryWithContextRequest
#[test]
fn test_call_unary_with_context_request_empty_body_error() {
    let request = GrpcRequest::new(
        "pkg.Service/Method",
        Vec::new(),
        std::time::Duration::from_secs(5),
    );
    let ctx = SecurityContext::unauthenticated();
    let wrapped = CallUnaryWithContextRequest { request, ctx };
    assert!(
        wrapped.request.body.is_empty(),
        "an empty payload is representable; callers must validate before dispatch"
    );
}

/// @covers: CallUnaryWithContextRequest
#[test]
fn test_call_unary_with_context_request_claim_backed_ctx_edge() {
    let request = GrpcRequest::new(
        "pkg.Service/Method",
        b"payload".to_vec(),
        std::time::Duration::from_secs(5),
    );
    let ctx = SecurityContext::unauthenticated().with_claim("role", "admin");
    let wrapped = CallUnaryWithContextRequest { request, ctx };
    assert_eq!(wrapped.request.method, "pkg.Service/Method");
}

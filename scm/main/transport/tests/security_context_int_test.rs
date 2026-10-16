//! The security context a unary call carries: `CallUnaryWithContextRequest` holds the request and the
//! `SecurityContext` of `edge-security-runtime` side by side, unchanged.

use std::collections::HashMap;
use std::time::Duration;

use edge_security_runtime::SecurityContext;
use grpc_egress_transport::{CallUnaryWithContextRequest, GrpcRequest};

fn request(method: &str) -> GrpcRequest {
    GrpcRequest {
        method: method.to_string(),
        body: b"payload".to_vec(),
        metadata: HashMap::new(),
        deadline: Duration::from_secs(5),
        cancellation: None,
    }
}

/// @covers: CallUnaryWithContextRequest
#[test]
fn test_new_request_with_context_keeps_the_trace_id_and_the_request() {
    let wrapped = CallUnaryWithContextRequest {
        request: request("pkg.Service/Method"),
        ctx: SecurityContext::unauthenticated().with_trace_id("trace-1"),
    };

    assert_eq!(wrapped.ctx.trace_id.as_deref(), Some("trace-1"));
    assert!(!wrapped.ctx.authenticated);
    assert_eq!(wrapped.request.method, "pkg.Service/Method");
    assert_eq!(wrapped.request.body, b"payload");
}

/// @covers: CallUnaryWithContextRequest
#[test]
fn test_new_request_with_context_distinct_contexts_stay_distinct() {
    let first = CallUnaryWithContextRequest {
        request: request("pkg.Service/A"),
        ctx: SecurityContext::unauthenticated().with_trace_id("trace-a"),
    };
    let second = CallUnaryWithContextRequest {
        request: request("pkg.Service/B"),
        ctx: SecurityContext::unauthenticated().with_trace_id("trace-b"),
    };

    assert_ne!(first.ctx.trace_id, second.ctx.trace_id);
    assert_ne!(first.request.method, second.request.method);
}

//! Integration tests for `api/interceptor/grpc/grpc_egress_interceptor.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use edge_transport_grpc_egress_transport::{
    AfterCallRequest, GrpcClientBuilder, GrpcEgressError, GrpcEgressInterceptor,
    GrpcEgressInterceptorChain, GrpcRequest, GrpcResponse, TraceContextSource,
};

#[test]
fn transport_trait_grpc_egress_interceptor_is_object_safe_int_test() {
    fn _assert(_: &dyn GrpcEgressInterceptor) {}
}

/// Test-double satisfying the two abstract methods. `before_call` injects a
/// metadata header (or fails, if `fail_before`); `after_call` injects a
/// response header (or fails, if `fail_after`) — used both to exercise the
/// trait's default (`Self: Sized`) methods and `before_call`/`after_call`
/// themselves from outside the crate.
// @allow: no_mocks_in_integration — hand-rolled test double, not a mock library.
#[derive(Default)]
struct StubInterceptor {
    fail_before: bool,
    fail_after: bool,
}

impl GrpcEgressInterceptor for StubInterceptor {
    fn before_call(&self, req: &mut GrpcRequest) -> Result<(), GrpcEgressError> {
        if self.fail_before {
            return Err(GrpcEgressError::Internal(
                "before_call forced failure".into(),
            ));
        }
        req.metadata
            .insert("x-stub-injected".to_string(), "1".to_string());
        Ok(())
    }

    fn after_call(&self, req: AfterCallRequest<'_>) -> Result<(), GrpcEgressError> {
        if self.fail_after {
            return Err(GrpcEgressError::Internal(
                "after_call forced failure".into(),
            ));
        }
        req.response
            .metadata
            .insert("x-stub-observed".to_string(), "1".to_string());
        Ok(())
    }
}

// ── default_request_builder ──────────────────────────────────────────────────

/// @covers: default_request_builder
#[test]
fn test_default_request_builder_missing_method_is_error() {
    let err = <StubInterceptor as GrpcEgressInterceptor>::default_request_builder()
        .build()
        .expect_err("a builder with no method set must fail to build");
    assert!(!err.is_empty(), "error message must be non-empty");
}

/// @covers: default_request_builder
#[test]
fn test_default_request_builder_with_method_and_deadline_builds_valid_request_happy() {
    let req = <StubInterceptor as GrpcEgressInterceptor>::default_request_builder()
        .method("/pkg.Svc/Method")
        .deadline(std::time::Duration::from_secs(5))
        .build()
        .expect("a builder with method and deadline set must build");
    assert_eq!(req.method, "/pkg.Svc/Method");
}

/// @covers: default_request_builder
#[test]
fn test_default_request_builder_repeated_calls_are_independent_edge() {
    let a = <StubInterceptor as GrpcEgressInterceptor>::default_request_builder()
        .method("/pkg.Svc/A")
        .deadline(std::time::Duration::from_secs(1))
        .build()
        .expect("first builder");
    let b = <StubInterceptor as GrpcEgressInterceptor>::default_request_builder()
        .method("/pkg.Svc/B")
        .deadline(std::time::Duration::from_secs(1))
        .build()
        .expect("second builder");
    assert_ne!(
        a.method, b.method,
        "each builder must be independently configured"
    );
}

// ── describe_chain_len ────────────────────────────────────────────────────────

/// @covers: describe_chain_len
#[test]
fn test_describe_chain_len_empty_chain_is_zero_happy() {
    let chain = GrpcEgressInterceptorChain::new();
    assert_eq!(
        <StubInterceptor as GrpcEgressInterceptor>::describe_chain_len(&chain),
        0
    );
}

/// @covers: describe_chain_len
#[test]
fn test_describe_chain_len_registered_interceptor_counts_one_error() {
    use std::sync::Arc;
    let chain = GrpcEgressInterceptorChain::new().push(Arc::new(StubInterceptor::default()));
    assert_eq!(
        <StubInterceptor as GrpcEgressInterceptor>::describe_chain_len(&chain),
        1
    );
}

/// @covers: describe_chain_len
#[test]
fn test_describe_chain_len_multiple_interceptors_edge() {
    use std::sync::Arc;
    let chain = GrpcEgressInterceptorChain::new()
        .push(Arc::new(StubInterceptor::default()))
        .push(Arc::new(StubInterceptor::default()))
        .push(Arc::new(StubInterceptor::default()));
    assert_eq!(
        <StubInterceptor as GrpcEgressInterceptor>::describe_chain_len(&chain),
        3
    );
}

// ── describe_trace_source ────────────────────────────────────────────────────

/// @covers: describe_trace_source
#[test]
fn test_describe_trace_source_pass_through_is_not_static_happy() {
    assert!(
        !<StubInterceptor as GrpcEgressInterceptor>::describe_trace_source(
            TraceContextSource::PassThrough
        )
    );
}

/// @covers: describe_trace_source
#[test]
fn test_describe_trace_source_static_without_tracestate_is_static_error() {
    assert!(
        <StubInterceptor as GrpcEgressInterceptor>::describe_trace_source(
            TraceContextSource::Static {
                traceparent: "00-0-0-00".to_string(),
                tracestate: None,
            }
        )
    );
}

/// @covers: describe_trace_source
#[test]
fn test_describe_trace_source_static_with_tracestate_is_static_edge() {
    assert!(
        <StubInterceptor as GrpcEgressInterceptor>::describe_trace_source(
            TraceContextSource::Static {
                traceparent: "00-0-0-00".to_string(),
                tracestate: Some("vendor=value".to_string()),
            }
        )
    );
}

// ── default_client_builder ────────────────────────────────────────────────────

/// @covers: default_client_builder
#[test]
fn test_default_client_builder_returns_zero_sized_marker_happy() {
    let builder: GrpcClientBuilder =
        <StubInterceptor as GrpcEgressInterceptor>::default_client_builder();
    assert_eq!(std::mem::size_of_val(&builder), 0);
}

/// @covers: default_client_builder
#[test]
fn test_default_client_builder_repeated_calls_are_independent_edge() {
    let a = <StubInterceptor as GrpcEgressInterceptor>::default_client_builder();
    let b = <StubInterceptor as GrpcEgressInterceptor>::default_client_builder();
    assert_eq!(std::mem::size_of_val(&a), std::mem::size_of_val(&b));
}

/// @covers: default_client_builder
#[test]
fn test_default_client_builder_is_the_real_marker_type_error() {
    fn assert_type(builder: GrpcClientBuilder) -> usize {
        std::mem::size_of_val(&builder)
    }
    let size = assert_type(<StubInterceptor as GrpcEgressInterceptor>::default_client_builder());
    assert_eq!(
        size, 0,
        "the returned value must unify with the genuine zero-sized GrpcClientBuilder, not a look-alike"
    );
}

// ── before_call ───────────────────────────────────────────────────────────────

/// @covers: before_call
#[test]
fn test_before_call_injects_metadata_happy() {
    let interceptor = StubInterceptor::default();
    let mut req = GrpcRequest::new("svc/Method", Vec::new(), std::time::Duration::from_secs(5));
    interceptor
        .before_call(&mut req)
        .expect("before_call should succeed");
    assert_eq!(req.metadata.get("x-stub-injected"), Some(&"1".to_string()));
}

/// @covers: before_call
#[test]
fn test_before_call_propagates_forced_failure_error() {
    let interceptor = StubInterceptor {
        fail_before: true,
        fail_after: false,
    };
    let mut req = GrpcRequest::new("svc/Method", Vec::new(), std::time::Duration::from_secs(5));
    let err = interceptor
        .before_call(&mut req)
        .expect_err("before_call must propagate the forced failure");
    assert!(matches!(err, GrpcEgressError::Internal(_)));
}

/// @covers: before_call
#[test]
fn test_before_call_does_not_overwrite_existing_metadata_edge() {
    let interceptor = StubInterceptor::default();
    let mut req = GrpcRequest::new("svc/Method", Vec::new(), std::time::Duration::from_secs(5));
    req.metadata
        .insert("x-caller-set".to_string(), "original".to_string());
    interceptor
        .before_call(&mut req)
        .expect("before_call should succeed");
    assert_eq!(
        req.metadata.get("x-caller-set"),
        Some(&"original".to_string()),
        "before_call must not clobber metadata set by the caller"
    );
    assert_eq!(req.metadata.get("x-stub-injected"), Some(&"1".to_string()));
}

// ── after_call ────────────────────────────────────────────────────────────────

/// @covers: after_call
#[test]
fn test_after_call_injects_response_metadata_happy() {
    let interceptor = StubInterceptor::default();
    let mut response = GrpcResponse {
        body: Vec::new(),
        metadata: std::collections::HashMap::new(),
    };
    interceptor
        .after_call(AfterCallRequest {
            response: &mut response,
        })
        .expect("after_call should succeed");
    assert_eq!(
        response.metadata.get("x-stub-observed"),
        Some(&"1".to_string())
    );
}

/// @covers: after_call
#[test]
fn test_after_call_propagates_forced_failure_error() {
    let interceptor = StubInterceptor {
        fail_before: false,
        fail_after: true,
    };
    let mut response = GrpcResponse {
        body: Vec::new(),
        metadata: std::collections::HashMap::new(),
    };
    let err = interceptor
        .after_call(AfterCallRequest {
            response: &mut response,
        })
        .expect_err("after_call must propagate the forced failure");
    assert!(matches!(err, GrpcEgressError::Internal(_)));
}

/// @covers: after_call
#[test]
fn test_after_call_preserves_response_body_edge() {
    let interceptor = StubInterceptor::default();
    let mut response = GrpcResponse {
        body: b"original-body".to_vec(),
        metadata: std::collections::HashMap::new(),
    };
    interceptor
        .after_call(AfterCallRequest {
            response: &mut response,
        })
        .expect("after_call should succeed");
    assert_eq!(
        response.body, b"original-body",
        "after_call must not mutate the response body, only metadata"
    );
}

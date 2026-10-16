//! Integration tests for `StdGrpcEgressInterceptorFactory`.

use std::time::Duration;

use grpc_egress_transport::{
    BuildGrpcEgressInterceptorRequest, GrpcEgressInterceptor, GrpcEgressInterceptorFactory,
    GrpcRequest, TraceContextSource,
};
use grpc_egress_transport_std::{GrpcRequestFactory, StdGrpcEgressInterceptorFactory};

fn bare_request() -> GrpcRequest {
    GrpcRequest::new("svc/Method", Vec::new(), Duration::from_secs(1))
}

/// @covers: build
#[test]
fn test_build_pass_through_source_injects_nothing() -> Result<(), Box<dyn std::error::Error>> {
    let factory = StdGrpcEgressInterceptorFactory::new(TraceContextSource::PassThrough);
    let interceptor = factory.build(BuildGrpcEgressInterceptorRequest);
    let mut req = bare_request();

    interceptor.before_call(&mut req)?;

    assert!(
        !req.metadata.contains_key("traceparent"),
        "pass-through must not inject traceparent metadata"
    );
    Ok(())
}

/// @covers: build
#[test]
fn test_build_static_source_injects_traceparent_and_tracestate(
) -> Result<(), Box<dyn std::error::Error>> {
    let factory = StdGrpcEgressInterceptorFactory::new(TraceContextSource::Static {
        traceparent: "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string(),
        tracestate: Some("vendor=1".to_string()),
    });
    let interceptor = factory.build(BuildGrpcEgressInterceptorRequest);
    let mut req = bare_request();

    interceptor.before_call(&mut req)?;

    assert_eq!(
        req.metadata.get("traceparent").map(String::as_str),
        Some("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01")
    );
    assert_eq!(
        req.metadata.get("tracestate").map(String::as_str),
        Some("vendor=1")
    );
    Ok(())
}

/// @covers: build
#[test]
fn test_build_static_source_keeps_upstream_traceparent() -> Result<(), Box<dyn std::error::Error>> {
    let factory = StdGrpcEgressInterceptorFactory::new(TraceContextSource::Static {
        traceparent: "injected".to_string(),
        tracestate: None,
    });
    let interceptor = factory.build(BuildGrpcEgressInterceptorRequest);
    let mut req = bare_request();
    req.metadata
        .insert("traceparent".to_string(), "existing-value".to_string());

    interceptor.before_call(&mut req)?;

    assert_eq!(
        req.metadata.get("traceparent").map(String::as_str),
        Some("existing-value"),
        "an upstream traceparent must not be overwritten"
    );
    Ok(())
}

/// @covers: build
#[test]
fn test_build_called_twice_returns_independent_interceptors(
) -> Result<(), Box<dyn std::error::Error>> {
    let factory = StdGrpcEgressInterceptorFactory::new(TraceContextSource::Static {
        traceparent: "injected".to_string(),
        tracestate: None,
    });
    let a = factory.build(BuildGrpcEgressInterceptorRequest);
    let b = factory.build(BuildGrpcEgressInterceptorRequest);
    let mut req_a = bare_request();
    let mut req_b = bare_request();

    a.before_call(&mut req_a)?;

    assert!(req_a.metadata.contains_key("traceparent"));
    assert!(
        !req_b.metadata.contains_key("traceparent"),
        "b has not run, so its request must be untouched"
    );
    b.before_call(&mut req_b)?;
    assert!(req_b.metadata.contains_key("traceparent"));
    Ok(())
}

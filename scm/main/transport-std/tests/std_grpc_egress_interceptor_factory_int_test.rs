//! Integration tests for `StdGrpcEgressInterceptorFactory`.

use grpc_egress_test_support::OutboundCallFixture;
use grpc_egress_transport::{
    BuildGrpcEgressInterceptorRequest, GrpcEgressInterceptor, GrpcEgressInterceptorFactory,
    OutboundCall, TraceContextSource,
};
use grpc_egress_transport_std::StdGrpcEgressInterceptorFactory;
use proto_pattern::Metadata;

const TRACEPARENT: &str = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";

fn bare_call() -> OutboundCallFixture {
    OutboundCallFixture::create("/svc/Method")
        .with_timeout(None)
        .with_metadata(Metadata::default())
}

/// @covers: build
#[test]
fn test_build_pass_through_source_injects_nothing() -> Result<(), Box<dyn std::error::Error>> {
    let factory = StdGrpcEgressInterceptorFactory::new(TraceContextSource::PassThrough);
    let interceptor = factory.build(BuildGrpcEgressInterceptorRequest);
    let mut call = bare_call();

    interceptor.before_call(&mut call)?;

    assert!(
        !call.has_header("traceparent"),
        "pass-through must not inject traceparent metadata"
    );
    assert_eq!(call.get_metadata().get_count(), 0);
    Ok(())
}

/// @covers: build
#[test]
fn test_build_static_source_injects_traceparent_and_tracestate(
) -> Result<(), Box<dyn std::error::Error>> {
    let factory = StdGrpcEgressInterceptorFactory::new(TraceContextSource::Static {
        traceparent: TRACEPARENT.to_string(),
        tracestate: Some("vendor=1".to_string()),
    });
    let interceptor = factory.build(BuildGrpcEgressInterceptorRequest);
    let mut call = bare_call();

    interceptor.before_call(&mut call)?;

    assert_eq!(call.get_header_values("traceparent"), [TRACEPARENT]);
    assert_eq!(call.get_header_values("tracestate"), ["vendor=1"]);
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
    let mut call = bare_call();
    call.add_header("traceparent", "existing-value")?;

    interceptor.before_call(&mut call)?;

    assert_eq!(
        call.get_header_values("traceparent"),
        ["existing-value"],
        "an upstream traceparent must not be overwritten, nor repeated"
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
    let mut call_a = bare_call();
    let mut call_b = bare_call();

    a.before_call(&mut call_a)?;

    assert!(call_a.has_header("traceparent"));
    assert!(
        !call_b.has_header("traceparent"),
        "b has not run, so its call must be untouched"
    );
    b.before_call(&mut call_b)?;
    assert!(call_b.has_header("traceparent"));
    Ok(())
}

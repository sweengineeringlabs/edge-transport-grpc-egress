//! Integration tests for `TraceContextGrpcEgressInterceptor`.
//!
//! Tests that access the `pub(crate)` field `interceptor.source` are omitted.

use grpc_egress_test_support::OutboundCallFixture;
use grpc_egress_transport_api::{GrpcEgressInterceptor, InboundReply, OutboundCall};
use proto_pattern::{Metadata, MetadataEntry};

const UPSTREAM: &str = "00-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa1-bbbbbbbbbbbbbbbb-01";
const INJECTED: &str = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";

fn call() -> OutboundCallFixture {
    OutboundCallFixture::create("/svc/M")
        .with_timeout(None)
        .with_metadata(Metadata::default())
}

fn call_with_upstream() -> Result<OutboundCallFixture, Box<dyn std::error::Error>> {
    Ok(OutboundCallFixture::create("/svc/M")
        .with_timeout(None)
        .with_metadata(Metadata::create(vec![MetadataEntry::create(
            "traceparent",
            UPSTREAM,
        )?])))
}

/// @covers: TraceContextGrpcEgressInterceptor::pass_through
#[test]
fn test_pass_through_keeps_an_existing_upstream_traceparent(
) -> Result<(), Box<dyn std::error::Error>> {
    let interceptor = trace_interceptor(grpc_egress_transport_api::TraceContextSource::PassThrough);
    let mut call = call_with_upstream()?;

    interceptor.before_call(&mut call)?;

    assert_eq!(call.header_values("traceparent"), [UPSTREAM]);
    Ok(())
}

/// @covers: TraceContextGrpcEgressInterceptor::pass_through
#[test]
fn test_pass_through_does_not_inject_a_traceparent() -> Result<(), Box<dyn std::error::Error>> {
    let interceptor = trace_interceptor(grpc_egress_transport_api::TraceContextSource::PassThrough);
    let mut call = call();

    interceptor.before_call(&mut call)?;

    assert!(!call.has_header("traceparent"));
    assert!(!call.has_header("tracestate"));
    Ok(())
}

/// @covers: TraceContextGrpcEgressInterceptor::with_static
#[test]
fn test_with_static_injects_the_traceparent_when_absent() -> Result<(), Box<dyn std::error::Error>>
{
    let interceptor = static_trace_interceptor(INJECTED, None);
    let mut call = call();

    interceptor.before_call(&mut call)?;

    assert_eq!(call.header_values("traceparent"), [INJECTED]);
    assert!(
        !call.has_header("tracestate"),
        "no tracestate was configured, so none is sent"
    );
    Ok(())
}

/// @covers: TraceContextGrpcEgressInterceptor::with_static
#[test]
fn test_with_static_injects_the_tracestate_when_configured(
) -> Result<(), Box<dyn std::error::Error>> {
    let interceptor = static_trace_interceptor(INJECTED, Some("v=1".to_string()));
    let mut call = call();

    interceptor.before_call(&mut call)?;

    assert_eq!(call.header_values("tracestate"), ["v=1"]);
    Ok(())
}

/// @covers: TraceContextGrpcEgressInterceptor::with_static
#[test]
fn test_with_static_keeps_an_upstream_traceparent_and_adds_no_tracestate(
) -> Result<(), Box<dyn std::error::Error>> {
    let interceptor = static_trace_interceptor(INJECTED, Some("v=1".to_string()));
    let mut call = call_with_upstream()?;

    interceptor.before_call(&mut call)?;

    assert_eq!(
        call.header_values("traceparent"),
        [UPSTREAM],
        "an upstream traceparent is not overwritten, nor repeated"
    );
    assert!(
        !call.has_header("tracestate"),
        "a call that carries a trace context gets none of ours"
    );
    Ok(())
}

/// @covers: TraceContextGrpcEgressInterceptor::after_call
#[test]
fn test_after_call_accepts_any_reply() {
    let interceptor = trace_interceptor(grpc_egress_transport_api::TraceContextSource::PassThrough);
    let reply = InboundReply::new("/svc/M", Metadata::default(), Metadata::default());

    assert!(interceptor.after_call(&reply).is_ok());
}

fn trace_interceptor(
    source: grpc_egress_transport_api::TraceContextSource,
) -> impl grpc_egress_transport_api::GrpcEgressInterceptor {
    use grpc_egress_transport_api::GrpcEgressInterceptorFactory;
    grpc_egress_transport_std::StdGrpcEgressInterceptorFactory::new(source)
        .build(grpc_egress_transport_api::BuildGrpcEgressInterceptorRequest)
}

fn static_trace_interceptor(
    traceparent: impl Into<String>,
    tracestate: Option<String>,
) -> impl grpc_egress_transport_api::GrpcEgressInterceptor {
    trace_interceptor(grpc_egress_transport_api::TraceContextSource::Static {
        traceparent: traceparent.into(),
        tracestate,
    })
}

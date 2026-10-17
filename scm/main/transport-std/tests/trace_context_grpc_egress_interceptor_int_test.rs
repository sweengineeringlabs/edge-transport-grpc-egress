//! Integration tests for `TraceContextGrpcEgressInterceptor`.
//!
//! Tests that access the `pub(crate)` field `interceptor.source` are omitted.

use grpc_egress_transport::{GrpcEgressInterceptor, InboundReply, OutboundCall};
use grpc_egress_transport_std::{OutboundCallHeaders, TraceContextGrpcEgressInterceptor};
use proto_pattern::{Metadata, MetadataEntry};

const UPSTREAM: &str = "00-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa1-bbbbbbbbbbbbbbbb-01";
const INJECTED: &str = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";

fn call() -> OutboundCall<'static> {
    OutboundCall {
        path: "/svc/M",
        timeout: None,
        metadata: Metadata::default(),
    }
}

fn call_with_upstream() -> Result<OutboundCall<'static>, Box<dyn std::error::Error>> {
    Ok(OutboundCall {
        path: "/svc/M",
        timeout: None,
        metadata: Metadata::create(vec![MetadataEntry::create("traceparent", UPSTREAM)?]),
    })
}

/// @covers: TraceContextGrpcEgressInterceptor::pass_through
#[test]
fn test_pass_through_keeps_an_existing_upstream_traceparent(
) -> Result<(), Box<dyn std::error::Error>> {
    let interceptor = TraceContextGrpcEgressInterceptor::pass_through();
    let mut call = call_with_upstream()?;

    interceptor.before_call(&mut call)?;

    assert_eq!(call.get_header_values("traceparent"), [UPSTREAM]);
    Ok(())
}

/// @covers: TraceContextGrpcEgressInterceptor::pass_through
#[test]
fn test_pass_through_does_not_inject_a_traceparent() -> Result<(), Box<dyn std::error::Error>> {
    let interceptor = TraceContextGrpcEgressInterceptor::pass_through();
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
    let interceptor = TraceContextGrpcEgressInterceptor::with_static(INJECTED, None);
    let mut call = call();

    interceptor.before_call(&mut call)?;

    assert_eq!(call.get_header_values("traceparent"), [INJECTED]);
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
    let interceptor =
        TraceContextGrpcEgressInterceptor::with_static(INJECTED, Some("v=1".to_string()));
    let mut call = call();

    interceptor.before_call(&mut call)?;

    assert_eq!(call.get_header_values("tracestate"), ["v=1"]);
    Ok(())
}

/// @covers: TraceContextGrpcEgressInterceptor::with_static
#[test]
fn test_with_static_keeps_an_upstream_traceparent_and_adds_no_tracestate(
) -> Result<(), Box<dyn std::error::Error>> {
    let interceptor =
        TraceContextGrpcEgressInterceptor::with_static(INJECTED, Some("v=1".to_string()));
    let mut call = call_with_upstream()?;

    interceptor.before_call(&mut call)?;

    assert_eq!(
        call.get_header_values("traceparent"),
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
    let interceptor = TraceContextGrpcEgressInterceptor::pass_through();
    let reply = InboundReply {
        path: "/svc/M",
        headers: Metadata::default(),
        trailers: Metadata::default(),
    };

    assert!(interceptor.after_call(&reply).is_ok());
}

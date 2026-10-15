//! Integration tests for `api/interceptor/trace/trace_context_source.rs`.

use edge_transport_grpc_egress_transport::TraceContextSource;

/// @covers: TraceContextSource::PassThrough — Clone preserves the variant
#[test]
fn transport_struct_pass_through_variant_is_clone_int_test() {
    let s = TraceContextSource::PassThrough;
    let cloned = s.clone();
    assert!(
        matches!(cloned, TraceContextSource::PassThrough),
        "clone of PassThrough must still be PassThrough"
    );
}

#[test]
fn transport_struct_static_variant_holds_traceparent_and_tracestate_int_test() {
    let s = TraceContextSource::Static {
        traceparent: "00-abc-def-01".into(),
        tracestate: Some("vendor=1".into()),
    };
    match s {
        TraceContextSource::Static {
            traceparent,
            tracestate,
        } => {
            assert_eq!(traceparent, "00-abc-def-01");
            assert_eq!(tracestate, Some("vendor=1".into()));
        }
        _ => panic!("expected Static"),
    }
}

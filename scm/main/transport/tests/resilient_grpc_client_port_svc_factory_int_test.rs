//! Integration tests for `ResilientGrpcClientPortFactory`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use edge_transport_grpc_egress_transport::{
    CircuitStateRequest, ConsecutiveFailuresRequest, GrpcChannelConfig,
    ResilientGrpcClientPortFactory, TransportConstruction,
};

fn ensure_rustls_provider() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    });
}

fn inner() -> std::sync::Arc<dyn edge_transport_grpc_egress_transport::GrpcEgress> {
    ensure_rustls_provider();
    let cfg = GrpcChannelConfig::new("http://127.0.0.1:50999").allow_plaintext();
    TransportConstruction::create_transport_from_config(&cfg).expect("create transport")
}

/// @covers: create
#[test]
fn test_create_wraps_inner_reporting_closed_circuit_happy() {
    let port = ResilientGrpcClientPortFactory::create(inner());
    let resp = port
        .circuit_state(CircuitStateRequest)
        .expect("circuit_state must succeed on a freshly-wrapped port");
    assert_eq!(
        resp.state, "Closed",
        "a freshly-wrapped port must report a closed circuit"
    );
}

/// @covers: create
#[test]
fn test_create_reports_zero_consecutive_failures_edge() {
    let port = ResilientGrpcClientPortFactory::create(inner());
    let resp = port
        .consecutive_failures(ConsecutiveFailuresRequest)
        .expect("consecutive_failures must succeed on a freshly-wrapped port");
    assert_eq!(
        resp.count, 0,
        "a freshly-wrapped port must report zero consecutive failures"
    );
}

/// @covers: create
#[test]
fn test_create_each_call_wraps_an_independent_port_edge() {
    let a = ResilientGrpcClientPortFactory::create(inner());
    let b = ResilientGrpcClientPortFactory::create(inner());
    assert_eq!(
        a.circuit_state(CircuitStateRequest).unwrap().state,
        "Closed"
    );
    assert_eq!(
        b.circuit_state(CircuitStateRequest).unwrap().state,
        "Closed"
    );
}

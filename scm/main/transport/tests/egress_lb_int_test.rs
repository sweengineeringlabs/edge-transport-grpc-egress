//! Integration tests for the load-balanced gRPC egress adapter's public surface.
//!
//! `TonicLbGrpcEgress` is `pub(crate)` (see SEA rule `pub_types_in_api_only`);
//! its own constructor/timeout/health_check behavior is covered by inline
//! tests in `spi/loadbalancer/tonic/lb_grpc_egress.rs`. This file exercises
//! only what's reachable from the crate's public surface.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use edge_transport_grpc_egress_transport::TransportConstruction;
use swe_edge_loadbalancer::{BackendConfig, LoadbalancerConfig, Strategy};

fn one_backend_config(url: &str) -> LoadbalancerConfig {
    LoadbalancerConfig {
        strategy: Strategy::RoundRobin,
        backends: vec![BackendConfig {
            url: url.to_string(),
            weight: 1,
        }],
    }
}

// ── SAF factory ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_lb_transport_from_config_returns_dyn_egress() {
    let config = one_backend_config("http://localhost:50051");
    let transport = TransportConstruction::create_lb_transport_from_config(config);
    assert!(transport.is_ok());
}

// ── object safety ────────────────────────────────────────────────────────────

#[test]
fn test_grpc_egress_trait_is_object_safe() {
    use edge_transport_grpc_egress_transport::GrpcEgress;
    fn _assert(_: &dyn GrpcEgress) {}
}

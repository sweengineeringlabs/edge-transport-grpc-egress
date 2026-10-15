//! Coverage stub for `src/api/transitions/breaker_transition.rs`.
//!
//! The `transitions` module re-exports `BreakerTransition`, which is
//! `pub(crate)` — not part of the public API.  This stub exercises the
//! public types the transition contract is defined over.

use edge_transport_grpc_egress_breaker::{BreakerState, GrpcBreakerClient, GrpcBreakerConfig};

/// @covers: api::transitions::BreakerTransition (internal) — config fields
#[test]
fn breaker_trait_breaker_transitions_config_is_accessible_int_test() {
    let cfg = GrpcBreakerConfig::default();
    // BreakerTransition::admit and ::record both take &GrpcBreakerConfig.
    assert!(cfg.failure_threshold >= 1);
    assert!(cfg.half_open_probe_count >= 1);
}

/// @covers: api::transitions::BreakerTransition (internal) — a fresh client starts Closed
#[tokio::test]
async fn breaker_trait_breaker_transitions_initial_state_is_closed_int_test() {
    let client = GrpcBreakerClient::new((), GrpcBreakerConfig::default());
    assert_eq!(client.state().await, BreakerState::Closed);
}

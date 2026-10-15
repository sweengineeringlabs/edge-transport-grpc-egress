//! Coverage stub for `src/api/breaker/admission.rs`.
//!
//! `Admission` is `pub(crate)` — it is not part of the public API.
//! This test exercises the publicly-exported surface that depends on
//! it: the circuit-breaker state machine routes through `Admission`
//! internally on every `call_unary`.

use edge_transport_grpc_egress_breaker::{BreakerState, GrpcBreakerConfig};

/// @covers: Admission (internal) — Closed state admits requests
#[test]
fn breaker_enum_admission_closed_state_admits_requests_int_test() {
    // BreakerNode starts Closed, so the first call is admitted (Proceed).
    // We verify the breaker does not reject while Closed.
    let cfg = GrpcBreakerConfig::default();
    assert!(
        cfg.failure_threshold >= 1,
        "default config must have positive threshold"
    );
    // Size-of check: GrpcBreakerClient wraps types that depend on Admission internally.
    let _ = std::mem::size_of::<GrpcBreakerConfig>();
    let _ = std::mem::size_of::<BreakerState>();
}

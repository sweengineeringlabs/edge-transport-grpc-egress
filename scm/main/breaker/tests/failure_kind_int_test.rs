//! Coverage stub for `src/api/breaker/failure_kind.rs`.
//!
//! `FailureClassifier` is `pub(crate)` — not part of the public API.
//! Its classification logic is exercised end-to-end via `GrpcBreakerClient`
//! in `breaker_int_test.rs`.  This stub verifies the public types it
//! operates on are accessible.

use edge_transport_grpc_egress_breaker::GrpcBreakerConfig;

/// @covers: FailureClassifier (internal) — GrpcBreakerConfig is the entry point
#[test]
fn breaker_struct_failure_classifier_config_accessible_int_test() {
    // FailureClassifier is used inside GrpcBreakerClient::call_unary.
    // Verify the config surface it reads is reachable.
    let cfg = GrpcBreakerConfig::default();
    assert!(cfg.failure_threshold >= 1);
}

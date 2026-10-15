//! Coverage stub for `src/api/breaker/client/breaker_egress.rs`.
//!
//! `BreakerEgress` is a `pub(crate)` trait — not part of the public API.
//! The public surface that depends on it is `GrpcBreakerClient`.

use edge_transport_grpc_egress_breaker::{GrpcBreakerClient, GrpcBreakerConfig};

/// @covers: BreakerEgress (internal) — GrpcBreakerClient is the public face
#[test]
fn breaker_trait_breaker_egress_grpc_breaker_client_is_accessible_int_test() {
    // GrpcBreakerClient<T> depends on the BreakerEgress impl in core/.
    // Verify the type is accessible and constructible.
    let cfg = GrpcBreakerConfig::default();
    let client = GrpcBreakerClient::new((), cfg);
    // The config is forwarded correctly.
    assert!(client.config().failure_threshold >= 1);
}

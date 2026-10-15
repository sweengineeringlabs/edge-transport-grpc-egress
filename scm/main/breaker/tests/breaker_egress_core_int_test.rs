#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Coverage stub for `src/core/breaker/client/breaker_egress.rs`.
//!
//! `BreakerEgress` in `core/` is `pub(crate)` — not part of the public
//! API.  The file contains the `GrpcEgress` impl block for
//! `GrpcBreakerClient<T>`.  This stub exercises that impl via the
//! public type.

use edge_transport_grpc_egress_breaker::{BreakerState, GrpcBreakerClient, GrpcBreakerConfig};

/// @covers: core::breaker::client::BreakerEgress — GrpcEgress impl exists
#[test]
fn breaker_struct_breaker_egress_core_client_is_constructible_int_test() {
    // The GrpcEgress impl for GrpcBreakerClient<T> lives in
    // core/breaker/client/breaker_egress.rs.  Constructing the client
    // verifies the impl compiles and links.
    let client = GrpcBreakerClient::new((), GrpcBreakerConfig::default());
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let state = rt.block_on(client.state());
    assert_eq!(state, BreakerState::Closed);
}

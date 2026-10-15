#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Coverage stub for `src/api/breaker/node.rs`.
//!
//! `BreakerNode` is `pub(crate)` — not part of the public API.
//! The node is the internal state container for `GrpcBreakerClient`.
//! This stub exercises the public type that owns it.

use edge_transport_grpc_egress_breaker::{BreakerState, GrpcBreakerClient, GrpcBreakerConfig};

/// @covers: BreakerNode (internal) — GrpcBreakerClient owns the node
#[test]
fn breaker_struct_node_initial_state_via_client_is_closed_int_test() {
    // A new GrpcBreakerClient wraps a freshly-initialised BreakerNode
    // whose state starts as BreakerState::Closed.
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let client = GrpcBreakerClient::new((), GrpcBreakerConfig::default());
    let state = rt.block_on(client.state());
    assert_eq!(state, BreakerState::Closed);
}

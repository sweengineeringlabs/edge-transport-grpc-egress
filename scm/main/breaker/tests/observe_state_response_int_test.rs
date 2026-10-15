//! Integration tests for [`ObserveStateResponse`].

use edge_transport_grpc_egress_breaker::{BreakerState, ObserveStateResponse};

/// @covers: ObserveStateResponse
#[test]
fn test_observe_state_response_closed_happy() {
    let resp = ObserveStateResponse {
        state: BreakerState::Closed,
    };
    assert!(matches!(resp.state, BreakerState::Closed));
}

/// @covers: ObserveStateResponse
#[test]
fn test_observe_state_response_open_error() {
    let resp = ObserveStateResponse {
        state: BreakerState::Open {
            since: std::time::Instant::now(),
        },
    };
    assert!(matches!(resp.state, BreakerState::Open { .. }));
}

/// @covers: ObserveStateResponse
#[test]
fn test_observe_state_response_half_open_edge() {
    let resp = ObserveStateResponse {
        state: BreakerState::HalfOpen,
    };
    assert!(matches!(resp.state, BreakerState::HalfOpen));
}

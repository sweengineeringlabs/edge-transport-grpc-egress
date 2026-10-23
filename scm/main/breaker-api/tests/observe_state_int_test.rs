//! The state of a breaker as the domain reports it: `BreakerState` of `edge-transport-breaker-policy`
//! inside an `ObserveStateResponse`, and the baseline policy of the crate.

use edge_transport_breaker_policy::BreakerState;
use grpc_egress_breaker_api::{GrpcBreakerConfig, ObserveStateResponse};

/// @covers: ObserveStateResponse
#[test]
fn test_response_carries_the_state_it_was_built_with() {
    let closed = ObserveStateResponse::new(BreakerState::Closed);

    assert_eq!(closed.state(), BreakerState::Closed);
}

/// @covers: ObserveStateResponse
#[test]
fn test_response_states_that_differ_are_not_equal() {
    let closed = ObserveStateResponse::new(BreakerState::Closed);
    let half_open = ObserveStateResponse::new(BreakerState::HalfOpen);

    assert_ne!(closed.state(), half_open.state());
}

/// @covers: ObserveStateRequest::new
#[test]
fn test_request_new_constructs_a_state_observation_request() {
    let _request = grpc_egress_breaker_api::ObserveStateRequest::new();
}

/// @covers: GrpcBreakerConfig
#[test]
fn test_default_config_is_the_baseline_of_the_crate() {
    let baseline = GrpcBreakerConfig::default();

    assert_eq!(baseline.failure_threshold(), 5);
    assert_eq!(baseline.cool_down_seconds(), 30);
    assert_eq!(baseline.half_open_probe_count(), 1);
}

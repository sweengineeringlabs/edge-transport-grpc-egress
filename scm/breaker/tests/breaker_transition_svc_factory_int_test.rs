#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`BreakerTransitionFactory`].

use edge_transport_grpc_egress_breaker::{
    Admission, AdmitRequest, BreakerState, BreakerTransitionFactory, GrpcBreakerConfig,
};

/// @covers: create
#[test]
fn test_create_produces_a_working_transition_happy() {
    let transition = BreakerTransitionFactory::create();
    let resp = transition
        .admit(AdmitRequest {
            state: BreakerState::Closed,
            consecutive_failures: 0,
            consecutive_successes: 0,
            config: GrpcBreakerConfig::default().into(),
        })
        .expect("factory-produced transition must admit successfully");
    assert_eq!(resp.admission, Admission::Proceed);
}

/// @covers: create
#[test]
fn test_create_rejects_when_open_error() {
    let transition = BreakerTransitionFactory::create();
    let resp = transition
        .admit(AdmitRequest {
            state: BreakerState::Open {
                since: std::time::Instant::now(),
            },
            consecutive_failures: 5,
            consecutive_successes: 0,
            config: GrpcBreakerConfig::default().into(),
        })
        .expect("admit is infallible");
    assert_eq!(resp.admission, Admission::RejectOpen);
}

/// @covers: create
#[test]
fn test_create_produces_independent_instances_edge() {
    let first = BreakerTransitionFactory::create();
    let second = BreakerTransitionFactory::create();
    let resp1 = first
        .admit(AdmitRequest {
            state: BreakerState::Closed,
            consecutive_failures: 0,
            consecutive_successes: 0,
            config: GrpcBreakerConfig::default().into(),
        })
        .expect("first must admit");
    let resp2 = second
        .admit(AdmitRequest {
            state: BreakerState::Closed,
            consecutive_failures: 0,
            consecutive_successes: 0,
            config: GrpcBreakerConfig::default().into(),
        })
        .expect("second must admit");
    assert_eq!(resp1.admission, resp2.admission);
}

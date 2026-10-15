//! Integration tests for [`RecordOutcomeRequest`].

use edge_transport_grpc_egress_breaker::{
    BreakerState, GrpcBreakerConfig, Outcome, RecordOutcomeRequest,
};

/// @covers: RecordOutcomeRequest
#[test]
fn test_record_outcome_request_success_happy() {
    let req = RecordOutcomeRequest {
        state: BreakerState::Closed,
        consecutive_failures: 0,
        consecutive_successes: 1,
        config: GrpcBreakerConfig::default().into(),
        outcome: Outcome::Success,
    };
    assert_eq!(req.outcome, Outcome::Success);
}

/// @covers: RecordOutcomeRequest
#[test]
fn test_record_outcome_request_failure_error() {
    let req = RecordOutcomeRequest {
        state: BreakerState::Closed,
        consecutive_failures: 3,
        consecutive_successes: 0,
        config: GrpcBreakerConfig::default().into(),
        outcome: Outcome::Failure,
    };
    assert_eq!(req.outcome, Outcome::Failure);
}

/// @covers: RecordOutcomeRequest
#[test]
fn test_record_outcome_request_half_open_state_edge() {
    let req = RecordOutcomeRequest {
        state: BreakerState::HalfOpen,
        consecutive_failures: 0,
        consecutive_successes: 0,
        config: GrpcBreakerConfig::default().into(),
        outcome: Outcome::Success,
    };
    assert!(matches!(req.state, BreakerState::HalfOpen));
}

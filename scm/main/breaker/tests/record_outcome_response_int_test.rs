//! Integration tests for [`RecordOutcomeResponse`].

use edge_transport_grpc_egress_breaker::{BreakerState, RecordOutcomeResponse};

/// @covers: RecordOutcomeResponse
#[test]
fn test_record_outcome_response_closed_happy() {
    let resp = RecordOutcomeResponse {
        state: BreakerState::Closed,
        consecutive_failures: 0,
        consecutive_successes: 3,
    };
    assert!(matches!(resp.state, BreakerState::Closed));
    assert_eq!(resp.consecutive_successes, 3);
}

/// @covers: RecordOutcomeResponse
#[test]
fn test_record_outcome_response_open_error() {
    let resp = RecordOutcomeResponse {
        state: BreakerState::Open {
            since: std::time::Instant::now(),
        },
        consecutive_failures: 5,
        consecutive_successes: 0,
    };
    assert!(matches!(resp.state, BreakerState::Open { .. }));
}

/// @covers: RecordOutcomeResponse
#[test]
fn test_record_outcome_response_zero_counts_edge() {
    let resp = RecordOutcomeResponse {
        state: BreakerState::Closed,
        consecutive_failures: 0,
        consecutive_successes: 0,
    };
    assert_eq!(resp.consecutive_failures, 0);
    assert_eq!(resp.consecutive_successes, 0);
}

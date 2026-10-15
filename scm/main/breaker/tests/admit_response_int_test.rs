//! Integration tests for [`AdmitResponse`].

use edge_transport_grpc_egress_breaker::{Admission, AdmitResponse, BreakerState};

/// @covers: AdmitResponse
#[test]
fn test_admit_response_proceed_happy() {
    let resp = AdmitResponse {
        admission: Admission::Proceed,
        state: BreakerState::Closed,
        consecutive_failures: 0,
        consecutive_successes: 0,
    };
    assert_eq!(resp.admission, Admission::Proceed);
}

/// @covers: AdmitResponse
#[test]
fn test_admit_response_reject_open_error() {
    let resp = AdmitResponse {
        admission: Admission::RejectOpen,
        state: BreakerState::Open {
            since: std::time::Instant::now(),
        },
        consecutive_failures: 5,
        consecutive_successes: 0,
    };
    assert_eq!(resp.admission, Admission::RejectOpen);
}

/// @covers: AdmitResponse
#[test]
fn test_admit_response_max_counts_edge() {
    let resp = AdmitResponse {
        admission: Admission::Proceed,
        state: BreakerState::Closed,
        consecutive_failures: u32::MAX,
        consecutive_successes: u32::MAX,
    };
    assert_eq!(resp.consecutive_failures, u32::MAX);
    assert_eq!(resp.consecutive_successes, u32::MAX);
}

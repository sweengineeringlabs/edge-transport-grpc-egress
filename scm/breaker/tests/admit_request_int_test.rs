//! Integration tests for [`AdmitRequest`].

use edge_transport_grpc_egress_breaker::{AdmitRequest, BreakerState, GrpcBreakerConfig};

/// @covers: AdmitRequest
#[test]
fn test_admit_request_construction_preserves_fields_happy() {
    let req = AdmitRequest {
        state: BreakerState::Closed,
        consecutive_failures: 2,
        consecutive_successes: 0,
        config: GrpcBreakerConfig::default().into(),
    };
    assert!(matches!(req.state, BreakerState::Closed));
    assert_eq!(req.consecutive_failures, 2);
}

/// @covers: AdmitRequest
#[test]
fn test_admit_request_open_state_error() {
    let req = AdmitRequest {
        state: BreakerState::Open {
            since: std::time::Instant::now(),
        },
        consecutive_failures: 5,
        consecutive_successes: 0,
        config: GrpcBreakerConfig::default().into(),
    };
    assert!(matches!(req.state, BreakerState::Open { .. }));
}

/// @covers: AdmitRequest
#[test]
fn test_admit_request_zero_counts_edge() {
    let req = AdmitRequest {
        state: BreakerState::HalfOpen,
        consecutive_failures: 0,
        consecutive_successes: 0,
        config: GrpcBreakerConfig::default().into(),
    };
    assert_eq!(req.consecutive_failures, 0);
    assert_eq!(req.consecutive_successes, 0);
}

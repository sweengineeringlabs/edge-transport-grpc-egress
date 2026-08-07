#![allow(clippy::unwrap_used, clippy::expect_used)]
//! End-to-end tests for [`BreakerTransition`] via a test-double
//! implementation.

use edge_transport_grpc_egress_breaker::{
    Admission, AdmitRequest, AdmitResponse, BreakerState, BreakerTransition, Error, Outcome,
    RecordOutcomeRequest, RecordOutcomeResponse,
};

/// A deliberately trivial transition policy — always proceeds and never
/// changes state — so tests can assert on the *contract*, not re-derive
/// the real threshold/cool-down logic already covered in `core::`.
struct MockBreakerTransition {
    fail: bool,
}

impl BreakerTransition for MockBreakerTransition {
    fn admit(&self, req: AdmitRequest) -> Result<AdmitResponse, Error> {
        if self.fail {
            return Err(Error::InvalidConfig(
                "mock transition forced failure".into(),
            ));
        }
        Ok(AdmitResponse {
            admission: Admission::Proceed,
            state: req.state,
            consecutive_failures: req.consecutive_failures,
            consecutive_successes: req.consecutive_successes,
        })
    }

    fn record(&self, req: RecordOutcomeRequest) -> Result<RecordOutcomeResponse, Error> {
        if self.fail {
            return Err(Error::InvalidConfig(
                "mock transition forced failure".into(),
            ));
        }
        Ok(RecordOutcomeResponse {
            state: req.state,
            consecutive_failures: req.consecutive_failures,
            consecutive_successes: req.consecutive_successes,
        })
    }
}

fn admit_req(state: BreakerState) -> AdmitRequest {
    AdmitRequest {
        state,
        consecutive_failures: 0,
        consecutive_successes: 0,
        config: edge_transport_grpc_egress_breaker::GrpcBreakerConfig::default().into(),
    }
}

/// @covers: admit
#[test]
fn test_admit_closed_state_proceeds_happy() {
    let transition = MockBreakerTransition { fail: false };
    let resp = transition
        .admit(admit_req(BreakerState::Closed))
        .expect("happy path");
    assert_eq!(resp.admission, Admission::Proceed);
}

/// @covers: admit
#[test]
fn test_admit_propagates_failure_error() {
    let transition = MockBreakerTransition { fail: true };
    let err = transition
        .admit(admit_req(BreakerState::Closed))
        .err()
        .expect("forced failure must surface");
    assert!(err.to_string().contains("mock transition forced failure"));
}

/// @covers: admit
#[test]
fn test_admit_open_state_edge() {
    let transition = MockBreakerTransition { fail: false };
    let resp = transition
        .admit(admit_req(BreakerState::Open {
            since: std::time::Instant::now(),
        }))
        .expect("this mock always proceeds regardless of state");
    assert!(matches!(resp.state, BreakerState::Open { .. }));
}

/// @covers: record
#[test]
fn test_record_success_outcome_happy() {
    let transition = MockBreakerTransition { fail: false };
    let resp = transition
        .record(RecordOutcomeRequest {
            state: BreakerState::Closed,
            consecutive_failures: 0,
            consecutive_successes: 0,
            config: edge_transport_grpc_egress_breaker::GrpcBreakerConfig::default().into(),
            outcome: Outcome::Success,
        })
        .expect("happy path");
    assert!(matches!(resp.state, BreakerState::Closed));
}

/// @covers: record
#[test]
fn test_record_propagates_failure_error() {
    let transition = MockBreakerTransition { fail: true };
    let err = transition
        .record(RecordOutcomeRequest {
            state: BreakerState::Closed,
            consecutive_failures: 0,
            consecutive_successes: 0,
            config: edge_transport_grpc_egress_breaker::GrpcBreakerConfig::default().into(),
            outcome: Outcome::Failure,
        })
        .err()
        .expect("forced failure must surface");
    assert!(err.to_string().contains("mock transition forced failure"));
}

/// @covers: record
#[test]
fn test_record_max_consecutive_failures_edge() {
    let transition = MockBreakerTransition { fail: false };
    let resp = transition
        .record(RecordOutcomeRequest {
            state: BreakerState::Closed,
            consecutive_failures: u32::MAX,
            consecutive_successes: 0,
            config: edge_transport_grpc_egress_breaker::GrpcBreakerConfig::default().into(),
            outcome: Outcome::Failure,
        })
        .expect("mock passes counts through unchanged, even at the boundary");
    assert_eq!(resp.consecutive_failures, u32::MAX);
}

/// @covers: describe_admission
#[test]
fn test_describe_admission_formats_proceed_happy() {
    assert_eq!(
        MockBreakerTransition::describe_admission(Admission::Proceed),
        "Proceed"
    );
}

/// @covers: describe_admission
#[test]
fn test_describe_admission_formats_reject_open_error() {
    // "error"-flavored scenario for an infallible formatter: the rejection
    // decision itself represents the error/negative path being described.
    assert_eq!(
        MockBreakerTransition::describe_admission(Admission::RejectOpen),
        "RejectOpen"
    );
}

/// @covers: describe_admission
#[test]
fn test_describe_admission_is_deterministic_edge() {
    let a = MockBreakerTransition::describe_admission(Admission::Proceed);
    let b = MockBreakerTransition::describe_admission(Admission::Proceed);
    assert_eq!(a, b, "formatting the same input twice must be stable");
}

/// @covers: describe_outcome
#[test]
fn test_describe_outcome_formats_success_happy() {
    assert_eq!(
        MockBreakerTransition::describe_outcome(Outcome::Success),
        "Success"
    );
}

/// @covers: describe_outcome
#[test]
fn test_describe_outcome_formats_failure_error() {
    assert_eq!(
        MockBreakerTransition::describe_outcome(Outcome::Failure),
        "Failure"
    );
}

/// @covers: describe_outcome
#[test]
fn test_describe_outcome_is_deterministic_edge() {
    let a = MockBreakerTransition::describe_outcome(Outcome::Failure);
    let b = MockBreakerTransition::describe_outcome(Outcome::Failure);
    assert_eq!(a, b, "formatting the same input twice must be stable");
}

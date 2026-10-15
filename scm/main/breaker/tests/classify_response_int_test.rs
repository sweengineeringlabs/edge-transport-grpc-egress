//! Integration tests for [`ClassifyResponse`].

use edge_transport_grpc_egress_breaker::{ClassifyResponse, Outcome};

/// @covers: ClassifyResponse
#[test]
fn test_classify_response_success_happy() {
    let resp = ClassifyResponse {
        outcome: Outcome::Success,
    };
    assert_eq!(resp.outcome, Outcome::Success);
}

/// @covers: ClassifyResponse
#[test]
fn test_classify_response_failure_error() {
    let resp = ClassifyResponse {
        outcome: Outcome::Failure,
    };
    assert_eq!(resp.outcome, Outcome::Failure);
}

/// @covers: ClassifyResponse
#[test]
fn test_classify_response_field_is_copy_edge() {
    let resp = ClassifyResponse {
        outcome: Outcome::Success,
    };
    let outcome_copy = resp.outcome;
    assert_eq!(outcome_copy, resp.outcome);
}

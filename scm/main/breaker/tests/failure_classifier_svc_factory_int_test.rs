#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`FailureClassifierFactory`].

use edge_transport_grpc_egress_breaker::{ClassifyRequest, FailureClassifierFactory, Outcome};

/// @covers: create
#[test]
fn test_create_classifies_failure_signal_happy() {
    let classifier = FailureClassifierFactory::create();
    let resp = classifier
        .classify(ClassifyRequest {
            is_breaker_failure: true,
        })
        .expect("factory-produced classifier must classify successfully");
    assert_eq!(resp.outcome, Outcome::Failure);
}

/// @covers: create
#[test]
fn test_create_classifies_non_failure_signal_error() {
    let classifier = FailureClassifierFactory::create();
    let resp = classifier
        .classify(ClassifyRequest {
            is_breaker_failure: false,
        })
        .expect("classify is infallible");
    assert_eq!(resp.outcome, Outcome::Success);
}

/// @covers: create
#[test]
fn test_create_produces_independent_instances_edge() {
    let first = FailureClassifierFactory::create();
    let second = FailureClassifierFactory::create();
    let resp1 = first
        .classify(ClassifyRequest {
            is_breaker_failure: true,
        })
        .expect("first must classify");
    let resp2 = second
        .classify(ClassifyRequest {
            is_breaker_failure: true,
        })
        .expect("second must classify");
    assert_eq!(resp1.outcome, resp2.outcome);
}

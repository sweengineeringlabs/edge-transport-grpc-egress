#![allow(clippy::unwrap_used, clippy::expect_used)]
//! End-to-end tests for [`FailureClassifier`] via a test-double
//! implementation.

use edge_transport_grpc_egress_breaker::{
    ClassifyRequest, ClassifyResponse, Error, FailureClassifier, Outcome,
};

struct MockFailureClassifier {
    fail: bool,
}

impl FailureClassifier for MockFailureClassifier {
    fn classify(&self, req: ClassifyRequest) -> Result<ClassifyResponse, Error> {
        if self.fail {
            return Err(Error::InvalidConfig(
                "mock classifier forced failure".into(),
            ));
        }
        Ok(ClassifyResponse {
            outcome: if req.is_breaker_failure {
                Outcome::Failure
            } else {
                Outcome::Success
            },
        })
    }
}

/// @covers: classify
#[test]
fn test_classify_failure_signal_returns_outcome_failure_happy() {
    let classifier = MockFailureClassifier { fail: false };
    let resp = classifier
        .classify(ClassifyRequest {
            is_breaker_failure: true,
        })
        .expect("happy path");
    assert_eq!(resp.outcome, Outcome::Failure);
}

/// @covers: classify
#[test]
fn test_classify_propagates_failure_error() {
    let classifier = MockFailureClassifier { fail: true };
    let err = classifier
        .classify(ClassifyRequest {
            is_breaker_failure: true,
        })
        .err()
        .expect("forced failure must surface");
    assert!(err.to_string().contains("mock classifier forced failure"));
}

/// @covers: classify
#[test]
fn test_classify_non_failure_signal_returns_outcome_success_edge() {
    let classifier = MockFailureClassifier { fail: false };
    let resp = classifier
        .classify(ClassifyRequest {
            is_breaker_failure: false,
        })
        .expect("happy path");
    assert_eq!(resp.outcome, Outcome::Success);
}

#![allow(clippy::unwrap_used, clippy::expect_used)]
//! End-to-end tests for [`JitterRng`] via a test-double implementation.

use edge_transport_grpc_egress_retry::{Error, JitterRng, NextUnitRequest, NextUnitResponse};

struct MockJitterRng {
    fail: bool,
    value: f64,
}

impl JitterRng for MockJitterRng {
    fn next_unit(&mut self, _req: NextUnitRequest) -> Result<NextUnitResponse, Error> {
        if self.fail {
            return Err(Error::InvalidConfig("mock rng forced failure".into()));
        }
        Ok(NextUnitResponse { value: self.value })
    }
}

/// @covers: next_unit
#[test]
fn test_next_unit_returns_configured_value_happy() {
    let mut rng = MockJitterRng {
        fail: false,
        value: 0.25,
    };
    let resp = rng.next_unit(NextUnitRequest).expect("happy path");
    assert_eq!(resp.value, 0.25);
}

/// @covers: next_unit
#[test]
fn test_next_unit_propagates_failure_error() {
    let mut rng = MockJitterRng {
        fail: true,
        value: 0.0,
    };
    let err = rng
        .next_unit(NextUnitRequest)
        .expect_err("forced failure must surface");
    assert!(err.to_string().contains("mock rng forced failure"));
}

/// @covers: next_unit
#[test]
fn test_next_unit_boundary_values_edge() {
    let mut low = MockJitterRng {
        fail: false,
        value: 0.0,
    };
    let mut high = MockJitterRng {
        fail: false,
        value: 0.999_999,
    };
    assert_eq!(low.next_unit(NextUnitRequest).unwrap().value, 0.0);
    assert!(high.next_unit(NextUnitRequest).unwrap().value < 1.0);
}

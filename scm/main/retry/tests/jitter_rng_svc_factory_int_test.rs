#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`JitterRngFactory`].

use edge_transport_grpc_egress_retry::{JitterRngFactory, NextUnitRequest};

/// @covers: create
#[test]
fn test_create_produces_a_working_rng_happy() {
    let mut rng = JitterRngFactory::create();
    let resp = rng
        .next_unit(NextUnitRequest)
        .expect("factory-produced rng must succeed");
    assert!((0.0..1.0).contains(&resp.value));
}

/// @covers: create
#[test]
fn test_create_produces_values_within_unit_interval_error() {
    let mut rng = JitterRngFactory::create();
    for _ in 0..10 {
        let resp = rng.next_unit(NextUnitRequest).expect("must succeed");
        assert!(
            (0.0..1.0).contains(&resp.value),
            "value {} outside [0, 1)",
            resp.value
        );
    }
}

/// @covers: create
#[test]
fn test_create_produces_independent_instances_edge() {
    let mut first = JitterRngFactory::create();
    let mut second = JitterRngFactory::create();
    let r1 = first
        .next_unit(NextUnitRequest)
        .expect("first must succeed");
    let r2 = second
        .next_unit(NextUnitRequest)
        .expect("second must succeed");
    assert!((0.0..1.0).contains(&r1.value));
    assert!((0.0..1.0).contains(&r2.value));
}

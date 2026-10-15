#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`NextUnitRequest`].

use edge_transport_grpc_egress_retry::{JitterRngFactory, NextUnitRequest};

/// @covers: NextUnitRequest
#[test]
fn test_next_unit_request_is_constructible_happy() {
    let req = NextUnitRequest;
    assert_eq!(std::mem::size_of_val(&req), 0);
}

/// @covers: NextUnitRequest
#[test]
fn test_next_unit_request_used_by_real_rng_error() {
    let mut rng = JitterRngFactory::create();
    let resp = rng
        .next_unit(NextUnitRequest)
        .expect("real rng must accept this request type");
    assert!((0.0..1.0).contains(&resp.value));
}

/// @covers: NextUnitRequest
#[test]
fn test_next_unit_request_reusable_edge() {
    let a = NextUnitRequest;
    let b = NextUnitRequest;
    assert_eq!(std::mem::size_of_val(&a), std::mem::size_of_val(&b));
}

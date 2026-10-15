//! Coverage stub for `src/api/breaker/breaker_state.rs`.

use std::time::{Duration, Instant};

use edge_transport_grpc_egress_breaker::BreakerState;

/// @covers: BreakerState — Debug is derived and names the variant
#[test]
fn breaker_enum_breaker_state_is_accessible_int_test() {
    let state = BreakerState::Closed;
    assert!(format!("{state:?}").contains("Closed"));
}

/// @covers: BreakerState::Open — two different `since` instants must not compare equal
#[test]
fn breaker_enum_breaker_state_open_with_different_instants_are_not_equal_int_test() {
    let earlier = Instant::now();
    let later = earlier + Duration::from_secs(1);
    assert_ne!(
        BreakerState::Open { since: earlier },
        BreakerState::Open { since: later }
    );
}

/// @covers: BreakerState::HalfOpen — distinct from Closed
#[test]
fn breaker_enum_breaker_state_half_open_differs_from_closed_int_test() {
    assert_ne!(BreakerState::HalfOpen, BreakerState::Closed);
}

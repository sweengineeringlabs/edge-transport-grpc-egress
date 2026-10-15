//! Integration tests for [`BackoffSchedule`].

use std::time::Duration;

use edge_transport_grpc_egress_retry::BackoffSchedule;

/// @covers: BackoffSchedule::from_duration
#[test]
fn test_from_duration_wraps_sleep() {
    let s = BackoffSchedule::from_duration(Duration::from_millis(150));
    assert_eq!(s.sleep, Duration::from_millis(150));
}

/// @covers: BackoffSchedule::from_duration
#[test]
fn test_from_duration_zero_is_valid() {
    let s = BackoffSchedule::from_duration(Duration::ZERO);
    assert_eq!(s.sleep, Duration::ZERO);
}

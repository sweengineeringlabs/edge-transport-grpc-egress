//! Integration tests for `KeepAliveConfig`.

use grpc_egress_transport::KeepAliveConfig;
use std::time::Duration;

/// @covers: KeepAliveConfig::default — uses recommended gRPC keep-alive intervals
#[test]
fn transport_struct_keep_alive_config_default_uses_recommended_grpc_intervals_int_test() {
    let cfg = KeepAliveConfig::default();
    assert_eq!(cfg.interval(), Duration::from_secs(10));
    assert_eq!(cfg.timeout(), Duration::from_secs(20));
    assert!(!cfg.permit_without_calls());
}

#[test]
fn keep_alive_config_rejects_zero_interval_or_timeout() {
    assert!(KeepAliveConfig::try_new(Duration::ZERO, Duration::from_secs(1), false).is_err());
    assert!(KeepAliveConfig::try_new(Duration::from_secs(1), Duration::ZERO, false).is_err());
}

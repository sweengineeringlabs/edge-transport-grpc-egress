//! Coverage stub for `src/api/breaker/grpc/grpc_breaker_config.rs`.

use edge_transport_grpc_egress_breaker::GrpcBreakerConfig;

/// @covers: GrpcBreakerConfig — type is accessible and holds real fields (not zero-sized)
#[test]
fn breaker_struct_grpc_breaker_config_is_accessible_int_test() {
    assert!(
        std::mem::size_of::<GrpcBreakerConfig>() > 0,
        "GrpcBreakerConfig holds failure_threshold/cool_down_seconds/half_open_probe_count and must not be zero-sized"
    );
}

/// @covers: GrpcBreakerConfig::default — positive defaults
#[test]
fn breaker_struct_grpc_breaker_config_default_has_positive_values_int_test() {
    let cfg = GrpcBreakerConfig::default();
    assert!(cfg.failure_threshold >= 1);
    assert!(cfg.half_open_probe_count >= 1);
}

/// @covers: GrpcBreakerConfig::cool_down — Duration from seconds
#[test]
fn breaker_struct_grpc_breaker_config_cool_down_matches_seconds_int_test() {
    use std::time::Duration;
    let cfg = GrpcBreakerConfig::default();
    assert_eq!(cfg.cool_down(), Duration::from_secs(cfg.cool_down_seconds));
}

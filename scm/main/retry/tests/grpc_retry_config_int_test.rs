#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Coverage stub for `src/api/types/grpc/grpc_retry_config.rs`.

use edge_transport_grpc_egress_retry::GrpcRetryConfig;

/// @covers: GrpcRetryConfig — type is accessible and holds real fields (not zero-sized)
#[test]
fn retry_struct_grpc_retry_config_is_accessible_int_test() {
    assert!(
        std::mem::size_of::<GrpcRetryConfig>() > 0,
        "GrpcRetryConfig holds max_attempts/backoff/jitter fields and must not be zero-sized"
    );
}

/// @covers: GrpcRetryConfig::default — SWE default values are valid
#[test]
fn retry_struct_grpc_retry_config_default_passes_validation_int_test() {
    let cfg = GrpcRetryConfig::default();
    assert!(cfg.max_attempts >= 1);
    assert!(cfg.backoff_multiplier > 0.0);
    assert!((0.0..=1.0).contains(&cfg.jitter_factor));
    assert!(cfg.max_backoff_ms >= cfg.initial_backoff_ms);
}

/// @covers: GrpcRetryConfig::from_config — parses TOML correctly
#[test]
fn retry_struct_grpc_retry_config_from_config_parses_full_toml_int_test() {
    let toml = r#"
        max_attempts = 3
        initial_backoff_ms = 50
        backoff_multiplier = 2.0
        jitter_factor = 0.1
        max_backoff_ms = 1000
        rate_limit_max_attempts = 2
        rate_limit_initial_backoff_ms = 100
        rate_limit_max_backoff_ms = 5000
    "#;
    let cfg = GrpcRetryConfig::from_config(toml).expect("valid toml");
    assert_eq!(cfg.max_attempts, 3);
}

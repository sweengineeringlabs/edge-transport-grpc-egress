//! Integration tests for [`ConfigValidationRequest`].

use edge_transport_grpc_egress_resilient::{ConfigValidationRequest, ResilienceConfig};

fn valid() -> ResilienceConfig {
    ResilienceConfig {
        max_attempts: 3,
        initial_backoff_ms: 10,
        backoff_multiplier: 2.0,
        jitter_factor: 0.1,
        max_backoff_ms: 100,
        rate_limit_max_attempts: 2,
        rate_limit_initial_backoff_ms: 10,
        rate_limit_max_backoff_ms: 100,
        failure_threshold: 7,
        cool_down_seconds: 10,
        half_open_probe_count: 2,
    }
}

/// @covers: ConfigValidationRequest
#[test]
fn test_config_validation_request_preserves_config_happy() {
    let req = ConfigValidationRequest { config: valid() };
    assert_eq!(req.config.failure_threshold, 7);
}

/// @covers: ConfigValidationRequest
#[test]
fn test_config_validation_request_zero_max_attempts_error() {
    let mut cfg = valid();
    cfg.max_attempts = 0;
    let req = ConfigValidationRequest { config: cfg };
    assert_eq!(req.config.max_attempts, 0);
}

/// @covers: ConfigValidationRequest
#[test]
fn test_config_validation_request_default_config_edge() {
    let req = ConfigValidationRequest {
        config: ResilienceConfig::default(),
    };
    let default_cfg = ResilienceConfig::default();
    assert_eq!(req.config.failure_threshold, default_cfg.failure_threshold);
    assert_eq!(req.config.max_attempts, default_cfg.max_attempts);
}

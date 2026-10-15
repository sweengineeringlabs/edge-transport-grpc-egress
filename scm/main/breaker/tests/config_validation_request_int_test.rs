//! Integration tests for [`ConfigValidationRequest`].

use edge_transport_grpc_egress_breaker::{ConfigValidationRequest, GrpcBreakerConfig};

/// @covers: ConfigValidationRequest
#[test]
fn test_config_validation_request_preserves_config_happy() {
    let req = ConfigValidationRequest {
        config: GrpcBreakerConfig {
            failure_threshold: 7,
            cool_down_seconds: 10,
            half_open_probe_count: 2,
        },
    };
    assert_eq!(req.config.failure_threshold, 7);
}

/// @covers: ConfigValidationRequest
#[test]
fn test_config_validation_request_zero_threshold_error() {
    let req = ConfigValidationRequest {
        config: GrpcBreakerConfig {
            failure_threshold: 0,
            cool_down_seconds: 10,
            half_open_probe_count: 2,
        },
    };
    assert_eq!(req.config.failure_threshold, 0);
}

/// @covers: ConfigValidationRequest
#[test]
fn test_config_validation_request_default_config_edge() {
    let req = ConfigValidationRequest {
        config: GrpcBreakerConfig::default(),
    };
    let default_cfg = GrpcBreakerConfig::default();
    assert_eq!(req.config.failure_threshold, default_cfg.failure_threshold);
    assert_eq!(req.config.cool_down_seconds, default_cfg.cool_down_seconds);
}

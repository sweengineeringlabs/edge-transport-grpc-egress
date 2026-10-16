//! Tests of `GrpcBreakerConfigProvider::validate`: the policy the breaker cannot work with is refused.

use grpc_egress_breaker::{BreakerDomainError, GrpcBreakerConfig};
use grpc_egress_breaker_std::GrpcBreakerConfigProvider;

fn valid() -> GrpcBreakerConfig {
    GrpcBreakerConfig {
        failure_threshold: 3,
        cool_down_seconds: 10,
        half_open_probe_count: 1,
    }
}

/// @covers: GrpcBreakerConfigProvider::validate
#[test]
fn test_validate_valid_config_returns_ok() {
    assert!(valid().validate().is_ok());
}

/// @covers: GrpcBreakerConfigProvider::validate
#[test]
fn test_validate_zero_failure_threshold_names_the_field() {
    let config = GrpcBreakerConfig {
        failure_threshold: 0,
        ..valid()
    };

    match config.validate() {
        Err(BreakerDomainError::InvalidConfig(message)) => {
            assert!(message.contains("failure_threshold"), "{message}");
        }
        other => panic!("expected InvalidConfig, got {other:?}"),
    }
}

/// @covers: GrpcBreakerConfigProvider::validate
#[test]
fn test_validate_zero_probe_count_names_the_field() {
    let config = GrpcBreakerConfig {
        half_open_probe_count: 0,
        ..valid()
    };

    match config.validate() {
        Err(BreakerDomainError::InvalidConfig(message)) => {
            assert!(message.contains("half_open_probe_count"), "{message}");
        }
        other => panic!("expected InvalidConfig, got {other:?}"),
    }
}

/// @covers: GrpcBreakerConfigProvider::from_config
#[test]
fn test_from_config_zero_failure_threshold_is_refused() {
    let text = "failure_threshold = 0\ncool_down_seconds = 1\nhalf_open_probe_count = 1\n";

    assert!(matches!(
        GrpcBreakerConfig::from_config(text),
        Err(BreakerDomainError::InvalidConfig(_))
    ));
}

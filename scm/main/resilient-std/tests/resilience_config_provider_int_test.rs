//! Tests of `ResilienceConfigProvider`: parsing, validation, and the conversion into the retry and
//! breaker policies.

use std::error::Error;

use grpc_egress_resilient::{ResilienceConfig, ResilientTransportError};
use grpc_egress_resilient_std::ResilienceConfigProvider;

fn valid() -> ResilienceConfig {
    ResilienceConfig {
        max_attempts: 3,
        initial_backoff_ms: 100,
        backoff_multiplier: 2.0,
        jitter_factor: 0.1,
        max_backoff_ms: 2_000,
        rate_limit_max_attempts: 2,
        rate_limit_initial_backoff_ms: 1_000,
        rate_limit_max_backoff_ms: 10_000,
        failure_threshold: 5,
        cool_down_seconds: 10,
        half_open_probe_count: 1,
    }
}

fn refusal(config: &ResilienceConfig) -> Result<String, Box<dyn Error>> {
    match config.validate() {
        Err(ResilientTransportError::InvalidResilience(message)) => Ok(message),
        other => Err(format!("expected InvalidResilience, got {other:?}").into()),
    }
}

/// @covers: ResilienceConfigProvider::validate
#[test]
fn test_validate_valid_config_returns_ok() {
    let invalid = ResilienceConfig {
        max_attempts: 0,
        ..valid()
    };

    assert!(valid().validate().is_ok());
    assert!(
        invalid.validate().is_err(),
        "the same policy with no attempts must not"
    );
}

/// @covers: ResilienceConfigProvider::validate
#[test]
fn test_validate_zero_max_attempts_names_the_field() -> Result<(), Box<dyn Error>> {
    let config = ResilienceConfig {
        max_attempts: 0,
        ..valid()
    };

    assert!(refusal(&config)?.contains("max_attempts"));
    Ok(())
}

/// @covers: ResilienceConfigProvider::validate
#[test]
fn test_validate_zero_rate_limit_max_attempts_names_the_field() -> Result<(), Box<dyn Error>> {
    let config = ResilienceConfig {
        rate_limit_max_attempts: 0,
        ..valid()
    };

    assert!(refusal(&config)?.contains("rate_limit_max_attempts"));
    Ok(())
}

/// @covers: ResilienceConfigProvider::validate
#[test]
fn test_validate_jitter_factor_out_of_range_names_the_field() -> Result<(), Box<dyn Error>> {
    for jitter_factor in [1.5, -0.1] {
        let config = ResilienceConfig {
            jitter_factor,
            ..valid()
        };

        assert!(
            refusal(&config)?.contains("jitter_factor"),
            "{jitter_factor}"
        );
    }
    Ok(())
}

/// @covers: ResilienceConfigProvider::validate
#[test]
fn test_validate_zero_half_open_probe_count_names_the_field() -> Result<(), Box<dyn Error>> {
    let config = ResilienceConfig {
        half_open_probe_count: 0,
        ..valid()
    };

    assert!(refusal(&config)?.contains("half_open_probe_count"));
    Ok(())
}

/// @covers: ResilienceConfigProvider::validate
#[test]
fn test_validate_rate_limit_max_backoff_below_initial_names_both_fields(
) -> Result<(), Box<dyn Error>> {
    let config = ResilienceConfig {
        rate_limit_max_backoff_ms: 500,
        rate_limit_initial_backoff_ms: 1_000,
        ..valid()
    };

    let message = refusal(&config)?;

    assert!(message.contains("rate_limit_max_backoff_ms"), "{message}");
    assert!(
        message.contains("rate_limit_initial_backoff_ms"),
        "{message}"
    );
    Ok(())
}

/// @covers: ResilienceConfigProvider::to_retry_config
#[test]
fn test_to_retry_config_maps_every_retry_field() {
    let config = valid();

    let retry = config.to_retry_config();

    assert_eq!(retry.max_attempts, 3);
    assert_eq!(retry.initial_backoff_ms, 100);
    assert_eq!(retry.max_backoff_ms, 2_000);
    assert_eq!(retry.rate_limit_max_attempts, 2);
    assert_eq!(retry.rate_limit_initial_backoff_ms, 1_000);
    assert_eq!(retry.rate_limit_max_backoff_ms, 10_000);
}

/// @covers: ResilienceConfigProvider::to_breaker_config
#[test]
fn test_to_breaker_config_maps_every_breaker_field() {
    let config = valid();

    let breaker = config.to_breaker_config();

    assert_eq!(breaker.failure_threshold, 5);
    assert_eq!(breaker.cool_down_seconds, 10);
    assert_eq!(breaker.half_open_probe_count, 1);
}

/// Real TOML is parsed and validated into a `ResilienceConfig` that produces the right retry and
/// breaker policies. Every asserted field differs from the profile default, so a field-mapping bug
/// (say, swapping `max_attempts` for `rate_limit_max_attempts`) fails this test.
///
/// @covers: ResilienceConfigProvider::from_config
#[test]
fn test_from_config_parses_toml_into_the_retry_and_breaker_policies() -> Result<(), Box<dyn Error>>
{
    let toml_text = r#"
        max_attempts                  = 7
        initial_backoff_ms            = 250
        backoff_multiplier            = 3.0
        jitter_factor                 = 0.2
        max_backoff_ms                = 4000
        rate_limit_max_attempts       = 4
        rate_limit_initial_backoff_ms = 2000
        rate_limit_max_backoff_ms     = 20000
        failure_threshold             = 9
        cool_down_seconds             = 45
        half_open_probe_count         = 3
    "#;

    let config = ResilienceConfig::from_config(toml_text)?;

    let retry = config.to_retry_config();
    assert_eq!(retry.max_attempts, 7);
    assert_eq!(retry.rate_limit_max_attempts, 4);
    assert_eq!(retry.rate_limit_initial_backoff_ms, 2000);
    let breaker = config.to_breaker_config();
    assert_eq!(breaker.failure_threshold, 9);
    assert_eq!(breaker.cool_down_seconds, 45);
    assert_eq!(breaker.half_open_probe_count, 3);
    Ok(())
}

/// @covers: ResilienceConfigProvider::from_config
#[test]
fn test_from_config_malformed_toml_is_an_invalid_resilience_error() {
    assert!(matches!(
        ResilienceConfig::from_config("not valid toml [[["),
        Err(ResilientTransportError::InvalidResilience(_))
    ));
}

/// @covers: ResilienceConfigProvider::from_config
#[test]
fn test_from_config_zero_max_attempts_is_an_invalid_resilience_error() {
    let toml_text = r#"
        max_attempts                  = 0
        initial_backoff_ms            = 100
        backoff_multiplier            = 2.0
        jitter_factor                 = 0.1
        max_backoff_ms                = 5000
        rate_limit_max_attempts       = 2
        rate_limit_initial_backoff_ms = 1000
        rate_limit_max_backoff_ms     = 10000
        failure_threshold             = 5
        cool_down_seconds             = 30
        half_open_probe_count         = 1
    "#;

    assert!(matches!(
        ResilienceConfig::from_config(toml_text),
        Err(ResilientTransportError::InvalidResilience(_))
    ));
}

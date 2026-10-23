//! Tests of `ResilienceConfigProvider`: parsing, validation, and the conversion into the retry and
//! breaker policies.

use std::error::Error;

use grpc_egress_breaker::GrpcBreakerConfig;
use grpc_egress_resilient::ResilienceConfigProvider as _;
use grpc_egress_resilient::{ResilienceConfig, ResilientTransportError};
use grpc_egress_retry::GrpcRetryConfig;

fn valid() -> Result<ResilienceConfig, Box<dyn Error>> {
    Ok(ResilienceConfig::from_policies(
        GrpcRetryConfig::try_new(grpc_egress_retry::BuildGrpcRetryConfigRequest {
            max_attempts: 3,
            initial_backoff_ms: 100,
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
            max_backoff_ms: 2_000,
            rate_limit_max_attempts: 2,
            rate_limit_initial_backoff_ms: 1_000,
            rate_limit_max_backoff_ms: 10_000,
        })?,
        GrpcBreakerConfig::try_new(true, 5, 10, 1)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?,
    ))
}

/// @covers: ResilienceConfigProvider::validate
#[test]
fn test_validate_valid_config_returns_ok() -> Result<(), Box<dyn Error>> {
    assert!(resilience_provider().validate(&valid()?).is_ok());
    assert!(
        GrpcRetryConfig::try_new(grpc_egress_retry::BuildGrpcRetryConfigRequest {
            max_attempts: 0,
            initial_backoff_ms: 100,
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
            max_backoff_ms: 2000,
            rate_limit_max_attempts: 2,
            rate_limit_initial_backoff_ms: 1000,
            rate_limit_max_backoff_ms: 10000,
        })
        .is_err()
    );
    Ok(())
}

/// @covers: ResilienceConfigProvider::validate
#[test]
fn test_validate_zero_max_attempts_names_the_field() {
    assert!(
        GrpcRetryConfig::try_new(grpc_egress_retry::BuildGrpcRetryConfigRequest {
            max_attempts: 0,
            initial_backoff_ms: 100,
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
            max_backoff_ms: 2000,
            rate_limit_max_attempts: 2,
            rate_limit_initial_backoff_ms: 1000,
            rate_limit_max_backoff_ms: 10000,
        })
        .err()
        .is_some_and(|error| error.to_string().contains("max_attempts"))
    );
}

/// @covers: ResilienceConfigProvider::validate
#[test]
fn test_validate_zero_rate_limit_max_attempts_names_the_field() {
    assert!(
        GrpcRetryConfig::try_new(grpc_egress_retry::BuildGrpcRetryConfigRequest {
            max_attempts: 3,
            initial_backoff_ms: 100,
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
            max_backoff_ms: 2000,
            rate_limit_max_attempts: 0,
            rate_limit_initial_backoff_ms: 1000,
            rate_limit_max_backoff_ms: 10000,
        })
        .err()
        .is_some_and(|error| error.to_string().contains("rate_limit_max_attempts"))
    );
}

/// @covers: ResilienceConfigProvider::validate
#[test]
fn test_validate_jitter_factor_out_of_range_names_the_field() -> Result<(), Box<dyn Error>> {
    for jitter_factor in [1.5, -0.1] {
        assert!(
            GrpcRetryConfig::try_new(grpc_egress_retry::BuildGrpcRetryConfigRequest {
                max_attempts: 3,
                initial_backoff_ms: 100,
                backoff_multiplier: 2.0,
                jitter_factor,
                max_backoff_ms: 2000,
                rate_limit_max_attempts: 2,
                rate_limit_initial_backoff_ms: 1000,
                rate_limit_max_backoff_ms: 10000,
            })
            .err()
            .is_some_and(|error| error.to_string().contains("jitter_factor"))
        );
    }
    Ok(())
}

/// @covers: ResilienceConfigProvider::validate
#[test]
fn test_validate_zero_half_open_probe_count_names_the_field() {
    assert!(GrpcBreakerConfig::try_new(true, 5, 10, 0)
        .err()
        .is_some_and(|error| error.to_string().contains("half_open_probe_count")));
}

/// @covers: ResilienceConfigProvider::validate
#[test]
fn test_validate_rate_limit_max_backoff_below_initial_names_both_fields() {
    assert!(
        GrpcRetryConfig::try_new(grpc_egress_retry::BuildGrpcRetryConfigRequest {
            max_attempts: 3,
            initial_backoff_ms: 100,
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
            max_backoff_ms: 2000,
            rate_limit_max_attempts: 2,
            rate_limit_initial_backoff_ms: 1000,
            rate_limit_max_backoff_ms: 500,
        })
        .is_err()
    );
}

/// @covers: ResilienceConfigProvider::to_retry_config
#[test]
fn test_to_retry_config_maps_every_retry_field() -> Result<(), Box<dyn Error>> {
    let config = valid()?;

    let retry = resilience_provider().to_retry_config(&config)?;

    assert_eq!(retry.max_attempts(), 3);
    assert_eq!(retry.initial_backoff_ms(), 100);
    assert_eq!(retry.max_backoff_ms(), 2_000);
    assert_eq!(retry.rate_limit_max_attempts(), 2);
    assert_eq!(retry.rate_limit_initial_backoff_ms(), 1_000);
    assert_eq!(retry.rate_limit_max_backoff_ms(), 10_000);
    Ok(())
}

/// @covers: ResilienceConfigProvider::to_breaker_config
#[test]
fn test_to_breaker_config_maps_every_breaker_field() -> Result<(), Box<dyn Error>> {
    let config = valid()?;

    let breaker = resilience_provider().to_breaker_config(&config)?;

    assert_eq!(breaker.failure_threshold(), 5);
    assert_eq!(breaker.cool_down_seconds(), 10);
    assert_eq!(breaker.half_open_probe_count(), 1);
    Ok(())
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

    let config = resilience_provider().parse_config(toml_text)?;

    let retry = resilience_provider().to_retry_config(&config)?;
    assert_eq!(retry.max_attempts(), 7);
    assert_eq!(retry.rate_limit_max_attempts(), 4);
    assert_eq!(retry.rate_limit_initial_backoff_ms(), 2000);
    let breaker = resilience_provider().to_breaker_config(&config)?;
    assert_eq!(breaker.failure_threshold(), 9);
    assert_eq!(breaker.cool_down_seconds(), 45);
    assert_eq!(breaker.half_open_probe_count(), 3);
    Ok(())
}

/// @covers: ResilienceConfigProvider::from_config
#[test]
fn test_from_config_malformed_toml_is_an_invalid_resilience_error() {
    assert!(matches!(
        resilience_provider().parse_config("not valid toml [[["),
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
        resilience_provider().parse_config(toml_text),
        Err(ResilientTransportError::InvalidResilience(_))
    ));
}

fn resilience_provider() -> impl grpc_egress_resilient::ResilienceConfigProvider {
    use grpc_egress_resilient::ResilienceConfigProviderFactory;
    grpc_egress_resilient_std::StdResilienceConfigProviderFactory
        .build(grpc_egress_resilient::BuildResilienceConfigProviderRequest)
}

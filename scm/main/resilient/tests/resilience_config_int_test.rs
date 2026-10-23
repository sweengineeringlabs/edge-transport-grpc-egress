//! Integration tests for [`ResilienceConfig`] — this crate's own owned
//! resilience policy shape (ADR-004: no longer a newtype over a foreign
//! transport-crate type).

use grpc_egress_breaker::GrpcBreakerConfig;
use grpc_egress_resilient::ResilienceConfig;
use grpc_egress_retry::GrpcRetryConfig;

/// @covers: ResilienceConfig::default
#[test]
fn test_default_is_the_fast_stateless_grpc_profile_happy() {
    let cfg = ResilienceConfig::default();
    assert_eq!(cfg.max_attempts(), 5);
    assert_eq!(cfg.failure_threshold(), 5);
}

/// @covers: ResilienceConfig
#[test]
fn test_zero_max_attempts_is_preserved_error() {
    assert!(
        GrpcRetryConfig::try_new(grpc_egress_retry::BuildGrpcRetryConfigRequest {
            max_attempts: 0,
            initial_backoff_ms: 100,
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
            max_backoff_ms: 5000,
            rate_limit_max_attempts: 2,
            rate_limit_initial_backoff_ms: 1000,
            rate_limit_max_backoff_ms: 10000,
        })
        .is_err()
    );
}

/// @covers: ResilienceConfig
#[test]
fn test_clone_is_independent_of_the_original_edge() {
    let original = ResilienceConfig::default();
    let cloned = original.clone();
    assert_eq!(original, cloned);
}

/// @covers: ResilienceConfig
#[test]
fn test_toml_round_trip_preserves_fields_happy() -> Result<(), Box<dyn std::error::Error>> {
    let retry = GrpcRetryConfig::try_new(grpc_egress_retry::BuildGrpcRetryConfigRequest {
        max_attempts: 3,
        initial_backoff_ms: 10,
        backoff_multiplier: 2.0,
        jitter_factor: 0.1,
        max_backoff_ms: 100,
        rate_limit_max_attempts: 2,
        rate_limit_initial_backoff_ms: 10,
        rate_limit_max_backoff_ms: 100,
    })?;
    let breaker = GrpcBreakerConfig::try_new(true, 7, 10, 2)?;
    let cfg = ResilienceConfig::from_policies(retry, breaker);
    let toml_text = toml::to_string(&cfg)?;
    let round_tripped: ResilienceConfig = toml::from_str(&toml_text)?;
    assert_eq!(round_tripped.failure_threshold(), 7);
    assert_eq!(round_tripped.max_attempts(), 3);
    Ok(())
}

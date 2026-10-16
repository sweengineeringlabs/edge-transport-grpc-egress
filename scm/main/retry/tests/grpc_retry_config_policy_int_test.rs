//! `GrpcRetryConfig` is the backoff policy of the shared retry core: the policy traits of
//! `edge-transport-retry-policy` read exactly the fields of the configuration.

use edge_transport_retry_policy::{BackoffPolicy, RateLimitBackoffPolicy};
use grpc_egress_retry::GrpcRetryConfig;

fn config() -> GrpcRetryConfig {
    GrpcRetryConfig {
        max_attempts: 7,
        initial_backoff_ms: 250,
        backoff_multiplier: 3.0,
        jitter_factor: 0.2,
        max_backoff_ms: 4_000,
        rate_limit_max_attempts: 4,
        rate_limit_initial_backoff_ms: 2_000,
        rate_limit_max_backoff_ms: 20_000,
    }
}

/// @covers: GrpcRetryConfig
#[test]
fn test_backoff_policy_reads_the_standard_track_fields() {
    let policy: &dyn BackoffPolicy = &config();

    assert_eq!(policy.initial_backoff_ms(), 250);
    assert_eq!(policy.max_backoff_ms(), 4_000);
    assert_eq!(policy.backoff_multiplier(), 3.0);
    assert_eq!(policy.jitter_factor(), 0.2);
}

/// @covers: GrpcRetryConfig
#[test]
fn test_rate_limit_policy_reads_the_rate_limit_track_fields() {
    let policy: &dyn RateLimitBackoffPolicy = &config();

    assert_eq!(policy.rate_limit_max_attempts(), 4);
    assert_eq!(policy.rate_limit_initial_backoff_ms(), 2_000);
    assert_eq!(policy.rate_limit_max_backoff_ms(), 20_000);
}

/// @covers: GrpcRetryConfig
#[test]
fn test_default_config_is_the_baseline_of_the_crate() {
    let baseline = GrpcRetryConfig::default();

    assert_eq!(baseline.max_attempts, 5);
    assert_eq!(baseline.initial_backoff_ms, 100);
    assert_eq!(baseline.rate_limit_max_attempts, 2);
    assert_ne!(baseline.max_attempts, config().max_attempts);
}

#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`GrpcRetryClient`].

use edge_transport_grpc_egress_retry::{GrpcRetryClient, GrpcRetryConfig};

/// @covers: GrpcRetryClient::new
#[test]
fn test_grpc_retry_client_new_stores_config_max_attempts() {
    let cfg = GrpcRetryConfig::from_config(
        r#"
            max_attempts = 5
            initial_backoff_ms = 10
            backoff_multiplier = 1.0
            jitter_factor = 0.0
            max_backoff_ms = 100
            rate_limit_max_attempts = 2
            rate_limit_initial_backoff_ms = 10
            rate_limit_max_backoff_ms = 100
        "#,
    )
    .unwrap();
    let client = GrpcRetryClient::new((), cfg);
    assert_eq!(client.config().max_attempts, 5);
    assert_eq!(client.config().initial_backoff_ms, 10);
}

/// @covers: GrpcRetryClient::config
#[test]
fn test_grpc_retry_client_config_returns_borrowed_config() {
    let cfg = GrpcRetryConfig::default();
    let max = cfg.max_attempts;
    let client = GrpcRetryClient::new((), cfg);
    assert_eq!(client.config().max_attempts, max);
}

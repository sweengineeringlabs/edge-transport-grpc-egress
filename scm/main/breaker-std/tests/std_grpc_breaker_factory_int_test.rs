//! Integration tests for `StdGrpcBreakerFactory`: it refuses a policy the breaker cannot work with, and the
//! client it builds starts closed and is observable.

use grpc_egress_breaker::{
    BreakerDomainError, BreakerObservable, BreakerState, BuildGrpcBreakerRequest,
    GrpcBreakerConfig, GrpcBreakerFactory, ObserveStateRequest,
};
use grpc_egress_breaker_std::StdGrpcBreakerFactory;
use grpc_egress_test_support::{ClientFixture, Fixture};
use grpc_egress_transport::GrpcChannelConfig;
use grpc_egress_transport_std::GrpcChannelConfigFactory;
use proto_pattern::{RawTransport, Transport};

/// A real client for an endpoint nothing listens on: building a client sends nothing, and the tests never call it.
fn idle_client() -> Result<impl Transport + RawTransport, Box<dyn std::error::Error>> {
    Fixture::ensure_tls_provider();
    let config = GrpcChannelConfig::new("http://127.0.0.1:1").allow_plaintext();
    Ok(ClientFixture::from_config(&config)?)
}

fn config(failure_threshold: u32, half_open_probe_count: u32) -> GrpcBreakerConfig {
    GrpcBreakerConfig {
        failure_threshold,
        cool_down_seconds: 5,
        half_open_probe_count,
    }
}

/// @covers: StdGrpcBreakerFactory::build
#[tokio::test]
async fn test_build_valid_config_returns_a_client_that_starts_closed(
) -> Result<(), Box<dyn std::error::Error>> {
    let client = StdGrpcBreakerFactory::new(config(3, 1)).build(BuildGrpcBreakerRequest {
        inner: idle_client()?,
    })?;

    let observed = client.state(ObserveStateRequest).await?;

    assert_eq!(observed.state, BreakerState::Closed);
    Ok(())
}

/// @covers: StdGrpcBreakerFactory::build
#[test]
fn test_build_zero_failure_threshold_returns_invalid_config_error(
) -> Result<(), Box<dyn std::error::Error>> {
    let result = StdGrpcBreakerFactory::new(config(0, 1)).build(BuildGrpcBreakerRequest {
        inner: idle_client()?,
    });

    match result {
        Err(BreakerDomainError::InvalidConfig(message)) => {
            assert!(message.contains("failure_threshold"), "{message}");
        }
        Err(other) => panic!("expected InvalidConfig, got {other}"),
        Ok(_) => panic!("a failure threshold of zero must be refused"),
    }
    Ok(())
}

/// @covers: StdGrpcBreakerFactory::build
#[test]
fn test_build_zero_probe_count_returns_invalid_config_error(
) -> Result<(), Box<dyn std::error::Error>> {
    let result = StdGrpcBreakerFactory::new(config(3, 0)).build(BuildGrpcBreakerRequest {
        inner: idle_client()?,
    });

    match result {
        Err(BreakerDomainError::InvalidConfig(message)) => {
            assert!(message.contains("half_open_probe_count"), "{message}");
        }
        Err(other) => panic!("expected InvalidConfig, got {other}"),
        Ok(_) => panic!("a probe count of zero must be refused"),
    }
    Ok(())
}

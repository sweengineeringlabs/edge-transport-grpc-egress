//! Integration tests for `StdGrpcBreakerFactory`: it refuses a policy the breaker cannot work with, and the
//! client it builds starts closed and is observable.

use grpc_egress_breaker_api::{
    BreakerDomainError, BreakerObservable, BreakerState, BuildGrpcBreakerRequest,
    GrpcBreakerConfig, GrpcBreakerFactory, ObserveStateRequest,
};
use grpc_egress_breaker_spi::StdGrpcBreakerFactory;
use grpc_egress_test_support::{ClientFixture, Fixture};
use grpc_egress_transport_api::GrpcChannelConfigBuilder as _;
use proto_pattern::{RawTransport, Transport};

/// A real client for an endpoint nothing listens on: building a client sends nothing, and the tests never call it.
fn idle_client() -> Result<impl Transport + RawTransport, Box<dyn std::error::Error>> {
    Fixture::ensure_tls_provider();
    let config = config_builder("http://127.0.0.1:1")
        .allow_plaintext()
        .build()?;
    Ok(ClientFixture::from_config(&config)?)
}

fn config(
    failure_threshold: u32,
    half_open_probe_count: u32,
) -> Result<GrpcBreakerConfig, grpc_egress_breaker_api::Error> {
    GrpcBreakerConfig::try_new(true, failure_threshold, 5, half_open_probe_count)
}

/// @covers: StdGrpcBreakerFactory::build
#[tokio::test]
async fn test_build_valid_config_returns_a_client_that_starts_closed(
) -> Result<(), Box<dyn std::error::Error>> {
    let client = StdGrpcBreakerFactory::new(config(3, 1)?).build(BuildGrpcBreakerRequest {
        inner: idle_client()?,
    })?;

    let observed = client.state(ObserveStateRequest::new()).await?;

    assert_eq!(observed.state(), BreakerState::Closed);
    Ok(())
}

/// @covers: StdGrpcBreakerFactory::build
#[test]
fn test_build_zero_failure_threshold_returns_invalid_config_error(
) -> Result<(), Box<dyn std::error::Error>> {
    assert!(matches!(
        config(0, 1),
        Err(BreakerDomainError::InvalidConfig(_))
    ));
    Ok(())
}

/// @covers: StdGrpcBreakerFactory::build
#[test]
fn test_build_zero_probe_count_returns_invalid_config_error(
) -> Result<(), Box<dyn std::error::Error>> {
    assert!(matches!(
        config(3, 0),
        Err(BreakerDomainError::InvalidConfig(_))
    ));
    Ok(())
}

fn config_builder(
    endpoint: impl Into<String>,
) -> impl grpc_egress_transport_api::GrpcChannelConfigBuilder {
    new_config_builder().with_endpoint(endpoint)
}

fn new_config_builder() -> impl grpc_egress_transport_api::GrpcChannelConfigBuilder {
    use grpc_egress_transport_api::GrpcChannelConfigBuilderFactory;
    grpc_egress_transport_std::StdGrpcChannelConfigBuilderFactory
        .build(grpc_egress_transport_api::BuildGrpcChannelConfigBuilderRequest)
}

//! Integration tests for `ResilientTransportError`, produced by
//! [`StdGrpcResilientFactory::build`].

use grpc_egress_breaker::{BreakerObservable, BreakerState, ObserveStateRequest};
use grpc_egress_resilient::{BuildGrpcResilientRequest, GrpcResilientFactory, ResilienceConfig};
use grpc_egress_resilient_std::StdGrpcResilientFactory;
use grpc_egress_test_support::{ClientFixture, Fixture};
use grpc_egress_transport::GrpcChannelConfig;
use grpc_egress_transport_std::GrpcChannelConfigFactory;
use proto_pattern::{RawTransport, Transport};

/// A real client built through the public builder; no connection is made until a call is.
fn built_client() -> Result<impl Transport + RawTransport, Box<dyn std::error::Error>> {
    Fixture::ensure_tls_provider();
    Ok(ClientFixture::from_config(
        &GrpcChannelConfig::new("http://127.0.0.1:1").allow_plaintext(),
    )?)
}

/// @covers: ResilientTransportError::InvalidResilience
#[test]
fn test_error_invalid_resilience_variant_produced_on_bad_config(
) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = ResilienceConfig {
        max_attempts: 0,
        ..ResilienceConfig::default()
    };
    let Err(err) = StdGrpcResilientFactory::new(cfg).build(BuildGrpcResilientRequest {
        inner: built_client()?,
    }) else {
        panic!("expected build to reject max_attempts = 0");
    };
    assert!(matches!(
        err,
        grpc_egress_resilient::ResilientTransportError::InvalidResilience(_)
    ));
    assert!(err.to_string().contains("invalid resilience config"));
    Ok(())
}

/// @covers: StdGrpcResilientFactory::build — a valid config wraps the inner client without error
#[tokio::test]
async fn test_build_valid_config_returns_a_client_whose_breaker_is_closed(
) -> Result<(), Box<dyn std::error::Error>> {
    let client = StdGrpcResilientFactory::new(ResilienceConfig::default()).build(
        BuildGrpcResilientRequest {
            inner: built_client()?,
        },
    )?;

    let observed = client.state(ObserveStateRequest).await?;

    assert_eq!(
        observed.state,
        BreakerState::Closed,
        "a fresh stack has not tripped"
    );
    Ok(())
}

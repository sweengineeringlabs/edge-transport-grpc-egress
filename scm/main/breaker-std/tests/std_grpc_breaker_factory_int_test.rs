//! Integration tests for `StdGrpcBreakerFactory`: it refuses a policy the breaker cannot work with, and
//! the client it builds starts closed and is observable.

use std::collections::HashMap;

use futures::future::BoxFuture;
use grpc_egress_breaker::{
    BreakerDomainError, BreakerObservable, BreakerState, BuildGrpcBreakerRequest,
    GrpcBreakerConfig, GrpcBreakerFactory, ObserveStateRequest,
};
use grpc_egress_breaker_std::StdGrpcBreakerFactory;
use grpc_egress_transport::{
    CallStreamRequest, GrpcEgress, GrpcEgressResult, GrpcMessageStreamResponse, GrpcRequest,
    GrpcResponse, HealthCheckRequest,
};

/// A client that answers every call with an empty success; the tests never call it.
struct Idle;

impl GrpcEgress for Idle {
    fn call_unary(&self, _req: GrpcRequest) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
        Box::pin(async {
            Ok(GrpcResponse {
                body: Vec::new(),
                metadata: HashMap::new(),
            })
        })
    }
    fn call_stream(
        &self,
        req: CallStreamRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcMessageStreamResponse>> {
        Box::pin(async move { Ok(req.messages) })
    }
    fn health_check(&self, _req: HealthCheckRequest) -> BoxFuture<'_, GrpcEgressResult<()>> {
        Box::pin(async { Ok(()) })
    }
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
    let client =
        StdGrpcBreakerFactory::new(config(3, 1)).build(BuildGrpcBreakerRequest { inner: Idle })?;

    let observed = client.state(ObserveStateRequest).await?;

    assert_eq!(observed.state, BreakerState::Closed);
    Ok(())
}

/// @covers: StdGrpcBreakerFactory::build
#[test]
fn test_build_zero_failure_threshold_returns_invalid_config_error() {
    let result =
        StdGrpcBreakerFactory::new(config(0, 1)).build(BuildGrpcBreakerRequest { inner: Idle });

    match result {
        Err(BreakerDomainError::InvalidConfig(message)) => {
            assert!(message.contains("failure_threshold"), "{message}");
        }
        Err(other) => panic!("expected InvalidConfig, got {other}"),
        Ok(_) => panic!("a failure threshold of zero must be refused"),
    }
}

/// @covers: StdGrpcBreakerFactory::build
#[test]
fn test_build_zero_probe_count_returns_invalid_config_error() {
    let result =
        StdGrpcBreakerFactory::new(config(3, 0)).build(BuildGrpcBreakerRequest { inner: Idle });

    match result {
        Err(BreakerDomainError::InvalidConfig(message)) => {
            assert!(message.contains("half_open_probe_count"), "{message}");
        }
        Err(other) => panic!("expected InvalidConfig, got {other}"),
        Ok(_) => panic!("a probe count of zero must be refused"),
    }
}

//! The contract of `GrpcResilientFactory`: an implementor builds a client that is also a
//! `BreakerObservable` from the breaker domain, and refuses an invalid policy with a
//! `ResilientTransportError`.

use std::collections::HashMap;
use std::error::Error;

use futures::future::BoxFuture;
use grpc_egress_breaker::{
    BreakerDomainError, BreakerObservable, BreakerState, ObserveStateRequest, ObserveStateResponse,
};
use grpc_egress_resilient::{
    BuildGrpcResilientRequest, GrpcResilientFactory, ResilienceConfig, ResilientTransportError,
};
use grpc_egress_transport::{
    GrpcEgress, GrpcEgressResult, GrpcRequest, GrpcResponse, HealthCheckRequest,
};

/// A client that answers every call with an empty success and reports a closed breaker.
struct ClosedClient;

impl GrpcEgress for ClosedClient {
    fn call_unary(&self, _req: GrpcRequest) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
        Box::pin(async {
            Ok(GrpcResponse {
                body: Vec::new(),
                metadata: HashMap::new(),
            })
        })
    }

    fn health_check(&self, _req: HealthCheckRequest) -> BoxFuture<'_, GrpcEgressResult<()>> {
        Box::pin(async { Ok(()) })
    }
}

impl BreakerObservable for ClosedClient {
    fn state(
        &self,
        _req: ObserveStateRequest,
    ) -> BoxFuture<'_, Result<ObserveStateResponse, BreakerDomainError>> {
        Box::pin(async {
            Ok(ObserveStateResponse {
                state: BreakerState::Closed,
            })
        })
    }
}

/// A factory that wraps nothing: it checks the policy and hands back [`ClosedClient`].
struct CheckingFactory {
    config: ResilienceConfig,
}

impl GrpcResilientFactory for CheckingFactory {
    fn build<T: GrpcEgress + Send + Sync + 'static>(
        &self,
        _req: BuildGrpcResilientRequest<T>,
    ) -> Result<impl GrpcEgress + BreakerObservable + use<T>, ResilientTransportError> {
        if self.config.max_attempts == 0 {
            return Err(ResilientTransportError::InvalidResilience(
                "max_attempts must be >= 1".into(),
            ));
        }
        Ok(ClosedClient)
    }
}

/// @covers: GrpcResilientFactory::build
#[tokio::test]
async fn test_build_valid_policy_returns_a_client_that_can_be_called_and_observed(
) -> Result<(), Box<dyn Error>> {
    let factory = CheckingFactory {
        config: ResilienceConfig::default(),
    };

    let client = factory.build(BuildGrpcResilientRequest {
        inner: ClosedClient,
    })?;

    let observed = client.state(ObserveStateRequest).await?;
    assert_eq!(observed.state, BreakerState::Closed);
    Ok(())
}

/// @covers: GrpcResilientFactory::build
#[test]
fn test_build_invalid_policy_returns_invalid_resilience_error() {
    let factory = CheckingFactory {
        config: ResilienceConfig {
            max_attempts: 0,
            ..ResilienceConfig::default()
        },
    };

    let result = factory.build(BuildGrpcResilientRequest {
        inner: ClosedClient,
    });

    assert!(matches!(
        result,
        Err(ResilientTransportError::InvalidResilience(_))
    ));
}

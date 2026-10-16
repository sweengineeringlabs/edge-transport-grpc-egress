//! The contract of `GrpcBreakerFactory`: an implementor returns a client that is also a
//! `BreakerObservable`, or refuses the policy with a `BreakerDomainError`.

use std::collections::HashMap;
use std::error::Error;
use std::time::Duration;

use futures::future::BoxFuture;
use grpc_egress_breaker::{
    BreakerDomainError, BreakerObservable, BreakerState, BuildGrpcBreakerRequest,
    GrpcBreakerConfig, GrpcBreakerFactory, ObserveStateRequest, ObserveStateResponse,
};
use grpc_egress_transport::{
    GrpcEgress, GrpcEgressResult, GrpcRequest, GrpcResponse, HealthCheckRequest,
};

/// A client that answers every call with an empty success.
struct Quiet;

impl GrpcEgress for Quiet {
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

/// The wrapped client, reporting a closed breaker.
struct Observed<T>(T);

impl<T: GrpcEgress + Send + Sync + 'static> GrpcEgress for Observed<T> {
    fn call_unary(&self, req: GrpcRequest) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
        self.0.call_unary(req)
    }

    fn health_check(&self, req: HealthCheckRequest) -> BoxFuture<'_, GrpcEgressResult<()>> {
        self.0.health_check(req)
    }
}

impl<T: Send + Sync> BreakerObservable for Observed<T> {
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

/// A factory that refuses a policy without a threshold and otherwise wraps without guarding.
struct CheckingFactory {
    config: GrpcBreakerConfig,
}

impl GrpcBreakerFactory for CheckingFactory {
    fn build<T: GrpcEgress + Send + Sync + 'static>(
        &self,
        req: BuildGrpcBreakerRequest<T>,
    ) -> Result<impl GrpcEgress + BreakerObservable + use<T>, BreakerDomainError> {
        if self.config.failure_threshold == 0 {
            return Err(BreakerDomainError::InvalidConfig(
                "failure_threshold must be >= 1".into(),
            ));
        }
        Ok(Observed(req.inner))
    }
}

fn request() -> GrpcRequest {
    GrpcRequest {
        method: "pkg.Service/Method".to_string(),
        body: Vec::new(),
        metadata: HashMap::new(),
        deadline: Duration::from_secs(1),
        cancellation: None,
    }
}

/// @covers: GrpcBreakerFactory::build
#[tokio::test]
async fn test_build_valid_policy_returns_a_client_that_can_be_called_and_observed(
) -> Result<(), Box<dyn Error>> {
    let factory = CheckingFactory {
        config: GrpcBreakerConfig::default(),
    };

    let client = factory.build(BuildGrpcBreakerRequest { inner: Quiet })?;

    client.call_unary(request()).await?;
    assert_eq!(
        client.state(ObserveStateRequest).await?.state,
        BreakerState::Closed
    );
    Ok(())
}

/// @covers: GrpcBreakerFactory::build
#[test]
fn test_build_policy_without_a_threshold_returns_invalid_config_error() {
    let factory = CheckingFactory {
        config: GrpcBreakerConfig {
            failure_threshold: 0,
            ..GrpcBreakerConfig::default()
        },
    };

    let result = factory.build(BuildGrpcBreakerRequest { inner: Quiet });

    assert!(matches!(result, Err(BreakerDomainError::InvalidConfig(_))));
}

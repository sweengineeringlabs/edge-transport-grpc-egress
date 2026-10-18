//! `StdGrpcBreakerFactory` -- wraps a client with a circuit breaker.

use grpc_egress_breaker::{
    BreakerDomainError, BreakerObservable, BuildGrpcBreakerRequest, GrpcBreakerConfig,
    GrpcBreakerFactory,
};
use proto_pattern::{RawTransport, Transport};
use tracing::debug;

use crate::std::{GrpcBreakerClient, GrpcBreakerConfigProvider};

/// Wraps clients with a circuit breaker under one [`GrpcBreakerConfig`].
pub struct StdGrpcBreakerFactory {
    config: GrpcBreakerConfig,
}

impl StdGrpcBreakerFactory {
    /// A factory whose clients are guarded by `config`.
    pub fn new(config: GrpcBreakerConfig) -> Self {
        Self { config }
    }
}

impl GrpcBreakerFactory for StdGrpcBreakerFactory {
    fn build<T: Transport + RawTransport + Send + Sync + 'static>(
        &self,
        req: BuildGrpcBreakerRequest<T>,
    ) -> Result<impl Transport + RawTransport + BreakerObservable + use<T>, BreakerDomainError>
    {
        self.config.validate()?;
        debug!("grpc-breaker: wrapping inner client");
        Ok(GrpcBreakerClient::new(req.inner, self.config.clone()))
    }
}

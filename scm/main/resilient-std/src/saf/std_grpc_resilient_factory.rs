//! `StdGrpcResilientFactory` -- wraps a client with retry and a circuit breaker.

use grpc_egress_breaker::{BreakerObservable, BuildGrpcBreakerRequest, GrpcBreakerFactory};
use grpc_egress_breaker_std::StdGrpcBreakerFactory;
use grpc_egress_resilient::{
    BuildGrpcResilientRequest, GrpcResilientFactory, ResilienceConfig, ResilientTransportError,
};
use grpc_egress_retry::{BuildGrpcRetryRequest, GrpcRetryFactory};
use grpc_egress_retry_std::StdGrpcRetryFactory;
use grpc_egress_transport::GrpcEgress;

use crate::std::ResilienceConfigProvider;

/// Wraps clients with retry and a circuit breaker under one [`ResilienceConfig`].
///
/// Call stack of a wrapped client: the breaker fails fast while the circuit is open, around the retry
/// with exponential backoff, around the bare client the caller supplied.
pub struct StdGrpcResilientFactory {
    config: ResilienceConfig,
}

impl StdGrpcResilientFactory {
    /// A factory whose clients follow `config`.
    pub fn new(config: ResilienceConfig) -> Self {
        Self { config }
    }
}

impl GrpcResilientFactory for StdGrpcResilientFactory {
    fn build<T: GrpcEgress + Send + Sync + 'static>(
        &self,
        req: BuildGrpcResilientRequest<T>,
    ) -> Result<impl GrpcEgress + BreakerObservable + use<T>, ResilientTransportError> {
        self.config.validate()?;
        let with_retry = StdGrpcRetryFactory::new(self.config.to_retry_config())
            .build(BuildGrpcRetryRequest { inner: req.inner });
        let with_breaker = StdGrpcBreakerFactory::new(self.config.to_breaker_config())
            .build(BuildGrpcBreakerRequest { inner: with_retry })?;
        Ok(with_breaker)
    }
}

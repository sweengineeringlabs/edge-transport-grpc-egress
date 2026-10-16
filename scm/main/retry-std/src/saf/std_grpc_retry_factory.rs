//! `StdGrpcRetryFactory` -- wraps a client with retry.

use grpc_egress_retry::{BuildGrpcRetryRequest, GrpcRetryConfig, GrpcRetryFactory};
use grpc_egress_transport::GrpcEgress;

use crate::std::GrpcRetryClient;

/// Wraps clients with retry under one [`GrpcRetryConfig`].
pub struct StdGrpcRetryFactory {
    config: GrpcRetryConfig,
}

impl StdGrpcRetryFactory {
    /// A factory whose clients retry according to `config`.
    pub fn new(config: GrpcRetryConfig) -> Self {
        Self { config }
    }
}

impl GrpcRetryFactory for StdGrpcRetryFactory {
    fn build<T: GrpcEgress + Send + Sync + 'static>(
        &self,
        req: BuildGrpcRetryRequest<T>,
    ) -> impl GrpcEgress + use<T> {
        GrpcRetryClient::new(req.inner, self.config.clone())
    }
}

//! `StdGrpcRetryFactory` -- wraps a client with retry.

use grpc_egress_retry::{BuildGrpcRetryRequest, GrpcRetryConfig, GrpcRetryFactory};
use proto_pattern::{RawTransport, Transport};

use crate::spi::decorator::GrpcRetryClient;

/// Wraps clients with retry as one policy says.
pub struct StdGrpcRetryFactory {
    config: GrpcRetryConfig,
}

impl StdGrpcRetryFactory {
    /// A factory whose clients retry as `config` says.
    pub fn new(config: GrpcRetryConfig) -> Self {
        Self { config }
    }
}

impl GrpcRetryFactory for StdGrpcRetryFactory {
    fn build<T: Transport + RawTransport + Send + Sync + 'static>(
        &self,
        req: BuildGrpcRetryRequest<T>,
    ) -> impl Transport + RawTransport + use<T> {
        GrpcRetryClient::from((req.inner, self.config.clone()))
    }
}

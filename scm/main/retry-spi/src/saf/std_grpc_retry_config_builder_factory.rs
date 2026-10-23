//! `StdGrpcRetryConfigBuilderFactory` -- creates the builder of a retry policy.

use grpc_egress_retry::{
    BuildGrpcRetryConfigBuilderRequest, GrpcRetryConfigBuilder, GrpcRetryConfigBuilderFactory,
};

use crate::spi::config::StdGrpcRetryConfigBuilder;

/// Creates builders of retry policies.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdGrpcRetryConfigBuilderFactory;

impl GrpcRetryConfigBuilderFactory for StdGrpcRetryConfigBuilderFactory {
    fn build(
        &self,
        _req: BuildGrpcRetryConfigBuilderRequest,
    ) -> impl GrpcRetryConfigBuilder + use<> {
        StdGrpcRetryConfigBuilder::default()
    }
}

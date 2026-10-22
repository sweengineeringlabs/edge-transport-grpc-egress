//! `GrpcRetryConfigBuilderFactory` -- creates the builder of a retry policy.

use crate::{BuildGrpcRetryConfigBuilderRequest, GrpcRetryConfigBuilder};

/// Creates a [`GrpcRetryConfigBuilder`].
pub trait GrpcRetryConfigBuilderFactory {
    /// A new builder at the baseline values. Each call returns an independent builder.
    fn build(
        &self,
        req: BuildGrpcRetryConfigBuilderRequest,
    ) -> impl GrpcRetryConfigBuilder + use<Self>;
}

//! `GrpcRetryConfigProviderFactory` -- creates the reader of retry policies.

use crate::{BuildGrpcRetryConfigProviderRequest, GrpcRetryConfigProvider};

/// Creates a [`GrpcRetryConfigProvider`].
pub trait GrpcRetryConfigProviderFactory {
    /// A provider that reads policies from TOML text.
    fn build(
        &self,
        req: BuildGrpcRetryConfigProviderRequest,
    ) -> impl GrpcRetryConfigProvider + use<Self>;
}

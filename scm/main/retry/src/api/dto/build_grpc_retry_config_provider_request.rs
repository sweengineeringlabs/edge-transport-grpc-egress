//! `BuildGrpcRetryConfigProviderRequest` -- the request of the retry policy provider factory.

/// Request for [`GrpcRetryConfigProviderFactory::build`](crate::GrpcRetryConfigProviderFactory::build). It carries
/// nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BuildGrpcRetryConfigProviderRequest;

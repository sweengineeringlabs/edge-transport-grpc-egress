//! `BuildGrpcRetryConfigBuilderRequest` -- the request of the retry policy builder factory.

/// Request for [`GrpcRetryConfigBuilderFactory::build`](crate::GrpcRetryConfigBuilderFactory::build). It carries
/// nothing: the builder starts at the baseline values and each is overridden on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BuildGrpcRetryConfigBuilderRequest;

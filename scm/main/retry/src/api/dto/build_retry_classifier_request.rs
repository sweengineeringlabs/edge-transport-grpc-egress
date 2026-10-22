//! `BuildRetryClassifierRequest` -- the request of the retry classifier factory.

/// Request for [`RetryClassifierFactory::build`](crate::RetryClassifierFactory::build). It carries nothing: the
/// classification is fixed by gRPC's semantics, not tunable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BuildRetryClassifierRequest;

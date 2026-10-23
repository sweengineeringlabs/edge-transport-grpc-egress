//! `BuildBearerSecretGuardRequest` -- the request of the secret guard factory.

/// Request for [`BearerSecretGuardFactory::build`](crate::BearerSecretGuardFactory::build). It carries nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BuildBearerSecretGuardRequest;

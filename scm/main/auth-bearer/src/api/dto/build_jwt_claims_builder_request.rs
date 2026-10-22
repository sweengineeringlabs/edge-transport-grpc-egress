//! `BuildJwtClaimsBuilderRequest` -- the request of the claims builder factory.

/// Request for [`JwtClaimsBuilderFactory::build`](crate::JwtClaimsBuilderFactory::build). It carries nothing: every
/// claim is set on the builder it returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BuildJwtClaimsBuilderRequest;

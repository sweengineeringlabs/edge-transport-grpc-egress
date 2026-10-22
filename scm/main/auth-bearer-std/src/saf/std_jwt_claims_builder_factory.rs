//! `StdJwtClaimsBuilderFactory` -- creates the builder of token claims.

use grpc_egress_auth_bearer::{
    BuildJwtClaimsBuilderRequest, JwtClaimsBuilder, JwtClaimsBuilderFactory,
};

use crate::std::StdJwtClaimsBuilder;

/// Creates builders of token claims.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdJwtClaimsBuilderFactory;

impl JwtClaimsBuilderFactory for StdJwtClaimsBuilderFactory {
    fn build(&self, _req: BuildJwtClaimsBuilderRequest) -> impl JwtClaimsBuilder + use<> {
        StdJwtClaimsBuilder::default()
    }
}

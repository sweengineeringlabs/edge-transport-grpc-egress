//! `JwtClaimsBuilderFactory` -- creates the builder of token claims.

use crate::{BuildJwtClaimsBuilderRequest, JwtClaimsBuilder};

/// Creates a [`JwtClaimsBuilder`].
pub trait JwtClaimsBuilderFactory {
    /// A new, empty builder. Each call returns an independent builder.
    fn build(&self, req: BuildJwtClaimsBuilderRequest) -> impl JwtClaimsBuilder + use<Self>;
}

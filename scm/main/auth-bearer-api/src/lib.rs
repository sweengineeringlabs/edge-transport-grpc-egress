//! `grpc_egress_auth_bearer_api` -- the bearer-auth domain: the outbound config, the secret, the claims,
//! the errors and the validation contract. The implementation (the interceptor that signs a JWT and
//! adds `authorization: Bearer <jwt>`) is in `grpc-egress-auth-bearer-spi`.
//!
//! ```compile_fail
//! use grpc_egress_auth_bearer_api::vo::BearerEgressConfig;
//! ```
//!
//! For the inbound (server-side) bearer validation counterpart see
//! `edge-transport-grpc-ingress-auth-bearer`.

#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod dto;
mod error;
mod traits;
mod vo;
pub use dto::{
    BuildBearerEgressConfigBuilderRequest, BuildBearerSecretGuardRequest,
    BuildJwtClaimsBuilderRequest,
};
pub use error::BearerAuthError;
pub use traits::{
    BearerEgressConfigBuilder, BearerEgressConfigBuilderFactory, BearerSecretGuard,
    BearerSecretGuardFactory, JwtClaimsBuilder, JwtClaimsBuilderFactory, Validator,
};
pub use vo::{
    BearerEgressConfig, BearerSecret, JwtClaims, AUTHORIZATION_HEADER, EXTRACTED_BEARER_SUBJECT,
};

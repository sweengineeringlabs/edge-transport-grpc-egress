//! `grpc_egress_auth_bearer` -- the bearer-auth domain: the outbound config, the secret, the claims,
//! the errors and the validation contract. The implementation (the interceptor that signs a JWT and
//! adds `authorization: Bearer <jwt>`) is in `grpc-egress-auth-bearer-std`.
//!
//! For the inbound (server-side) bearer validation counterpart see
//! `edge-transport-grpc-ingress-auth-bearer`.

#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod api;

pub use api::error::BearerAuthError;
pub use api::traits::Validator;
pub use api::vo::{
    BearerEgressConfig, BearerSecret, JwtClaims, AUTHORIZATION_HEADER, EXTRACTED_BEARER_SUBJECT,
};

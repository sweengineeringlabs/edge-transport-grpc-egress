//! `grpc-egress-auth-bearer-std` — JWT bearer
//! [`GrpcEgressInterceptor`](grpc_egress_transport::GrpcEgressInterceptor)
//! for the gRPC egress stack.
//!
//! ```compile_fail
//! use grpc_egress_auth_bearer_std::saf::StdBearerEgressConfigBuilderFactory;
//! ```
//!
//! [`StdBearerInterceptorFactory`] signs a JWT from configured claims
//! (or accepts a pre-minted token) and injects it into the
//! `authorization` request metadata of every outgoing call that does not already carry one.
//!
//! For the inbound (server-side) bearer validation counterpart see
//! `edge-transport-grpc-ingress-auth-bearer`.
//!
//! Constant-time comparisons (`subtle`) are used for any symmetric
//! shared-secret material in the configuration loaders.

#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod saf;
mod spi;
mod std;

pub use crate::saf::{
    StdBearerEgressConfigBuilderFactory, StdBearerInterceptorFactory, StdBearerSecretGuardFactory,
    StdJwtClaimsBuilderFactory,
};

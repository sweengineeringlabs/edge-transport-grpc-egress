//! `edge-transport-grpc-egress-auth-bearer` — JWT bearer
//! [`GrpcEgressInterceptor`](grpc_egress_transport::GrpcEgressInterceptor)
//! for the gRPC egress stack.
//!
//! [`BearerEgressInterceptor`] signs a JWT from configured claims
//! (or accepts a pre-minted token) and injects it into the
//! `authorization` request metadata on every outgoing call.
//!
//! For the inbound (server-side) bearer validation counterpart see
//! `edge-transport-grpc-ingress-auth-bearer`.
//!
//! Constant-time comparisons (`subtle`) are used for any symmetric
//! shared-secret material in the configuration loaders.

#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod api;
mod core;
mod saf;
mod spi;

pub use saf::*;

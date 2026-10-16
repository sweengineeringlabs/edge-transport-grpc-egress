//! `grpc_egress_resilient_std` — resilience decorator for gRPC egress.
//!
//! This crate is a **decorator only** — it never builds a base transport
//! client. A composition root builds the bare client (e.g. via
//! `grpc_egress_transport_std::TransportConstruction::create_tonic_client_from_config`)
//! and calls [`GrpcResilientFacade::apply_resilience`] to wrap it in a
//! [`grpc_egress_retry_std::GrpcRetryClient`] then a
//! [`grpc_egress_breaker_std::GrpcBreakerClient`].
//!
//! Call-stack once wrapped:
//! ```text
//! GrpcBreakerClient   ← fast-fail when circuit is open
//!   └─ GrpcRetryClient ← exponential-backoff retry
//!        └─ (caller-supplied bare client) ← hyper HTTP/2 transport
//! ```

#![warn(missing_docs)]
#![deny(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod saf;
mod std;

pub use crate::std::{
    ApplicationConfigBuilderProvider, GrpcResilientFacade, GrpcResilientSvcProcessor,
    ResilienceConfigProvider,
};
pub use saf::*;

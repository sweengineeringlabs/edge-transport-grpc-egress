//! `grpc_egress_resilient_std` — resilience decorator for gRPC egress.
//!
//! This crate is a **decorator only** — it never builds a base transport
//! client. A composition root builds the bare client (e.g. via
//! `grpc_egress_transport_std::TonicGrpcEgressBuilderSpiFactory`)
//! and wraps it with [`StdGrpcResilientFactory`]: retry first
//! ([`grpc_egress_retry_std::StdGrpcRetryFactory`]), then a circuit breaker
//! ([`grpc_egress_breaker_std::StdGrpcBreakerFactory`]).
//!
//! Call-stack once wrapped:
//! ```text
//! circuit breaker     ← fast-fail when circuit is open
//!   └─ retry          ← exponential-backoff retry
//!        └─ (caller-supplied bare client) ← hyper HTTP/2 transport
//! ```

#![warn(missing_docs)]
#![deny(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod saf;
mod std;

pub use crate::std::ResilienceConfigProvider;
pub use saf::StdGrpcResilientFactory;

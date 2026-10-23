//! `grpc_egress_resilient_spi` — resilience decorator for gRPC egress.
//!
//! This crate is a **decorator only** — it never builds a base transport
//! client. A composition root builds the bare client (e.g. via
//! `grpc_egress_transport_spi::TonicGrpcEgressBuilderSpiFactory`; a `Transport` and a `RawTransport`)
//! and wraps it with [`StdGrpcResilientFactory`]: retry first
//! ([`grpc_egress_retry_spi::StdGrpcRetryFactory`]), then a circuit breaker
//! ([`grpc_egress_breaker_spi::StdGrpcBreakerFactory`]).
//!
//! ```compile_fail
//! use grpc_egress_resilient_spi::saf::StdGrpcResilientFactory;
//! ```
//!
//! Call-stack once wrapped:
//! ```text
//! circuit breaker     ← fast-fail when circuit is open
//!   └─ retry          ← exponential-backoff retry
//!        └─ (caller-supplied bare client) ← the proto-svc HTTP/2 transport
//! ```

#![warn(missing_docs)]
#![deny(unsafe_code)]

mod saf;
mod spi;

pub use saf::{StdGrpcResilientFactory, StdResilienceConfigProviderFactory};

//! `edge_transport_grpc_egress_resilient` — resilience decorator for gRPC egress.
//!
//! This crate is a **decorator only** — it never builds a base transport
//! client. A composition root builds the bare client (e.g. via
//! `edge_transport_grpc_egress::TransportConstruction::create_tonic_client_from_config`)
//! and calls [`GrpcResilientFacade::apply_resilience`] to wrap it in a
//! [`edge_transport_grpc_egress_retry::GrpcRetryClient`] then a
//! [`edge_transport_grpc_egress_breaker::GrpcBreakerClient`].
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

mod api;
mod core;
mod saf;

// Public contracts and value objects — all flow directly from api/.
pub use crate::api::{
    ApplicationConfigBuilder, ConfigBuilderProvider, ConfigBuilderRequest, ConfigBuilderResponse,
    ConfigValidationRequest, DescribeRequest, DescribeResponse, GrpcResilientFacade,
    GrpcResilientSvcProcessor, Processor, ResilienceConfig, ResilientTransportError, Validator,
};
pub use saf::*;

//! `grpc_egress_resilient` -- the resilience domain: the contracts, requests, responses, errors and the
//! resilience policy value (`ResilienceConfig`). The implementation, which wraps a `GrpcEgress` in
//! retry and a circuit breaker, is in `grpc-egress-resilient-std`.

#![warn(missing_docs)]
#![deny(unsafe_code)]

mod api;

pub use api::dto::BuildGrpcResilientRequest;
pub use api::error::ResilientTransportError;
pub use api::traits::GrpcResilientFactory;
pub use api::vo::ResilienceConfig;

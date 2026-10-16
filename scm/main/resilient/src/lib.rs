//! `grpc_egress_resilient` -- the resilience domain: the contracts, requests, responses, errors and the
//! resilience policy value (`ResilienceConfig`). The implementation, which wraps a `GrpcEgress` in
//! retry and a circuit breaker, is in `grpc-egress-resilient-std`.

#![warn(missing_docs)]
#![deny(unsafe_code)]

mod dto;
mod error;
mod traits;
mod vo;

pub use dto::BuildGrpcResilientRequest;
pub use error::ResilientTransportError;
pub use traits::GrpcResilientFactory;
pub use vo::ResilienceConfig;

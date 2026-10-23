//! `grpc_egress_resilient_api` -- the resilience domain: the contracts, requests, responses, errors and the
//! resilience policy value (`ResilienceConfig`). The implementation, which wraps a client that is a `Transport` and a `RawTransport` in
//! retry and a circuit breaker, is in `grpc-egress-resilient-spi`.
//!
//! ```compile_fail
//! use grpc_egress_resilient_api::vo::ResilienceConfig;
//! ```

#![warn(missing_docs)]
#![deny(unsafe_code)]

mod dto;
mod error;
mod traits;
mod vo;
pub use dto::{BuildGrpcResilientRequest, BuildResilienceConfigProviderRequest};
pub use error::ResilientTransportError;
pub use traits::{GrpcResilientFactory, ResilienceConfigProvider, ResilienceConfigProviderFactory};
pub use vo::ResilienceConfig;

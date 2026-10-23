//! `grpc_egress_resilient` -- the resilience domain: the contracts, requests, responses, errors and the
//! resilience policy value (`ResilienceConfig`). The implementation, which wraps a client that is a `Transport` and a `RawTransport` in
//! retry and a circuit breaker, is in `grpc-egress-resilient-spi`.
//!
//! ```compile_fail
//! use grpc_egress_resilient::api::vo::ResilienceConfig;
//! ```

#![warn(missing_docs)]
#![deny(unsafe_code)]

mod api;

pub use api::dto::{BuildGrpcResilientRequest, BuildResilienceConfigProviderRequest};
pub use api::error::ResilientTransportError;
pub use api::traits::{
    GrpcResilientFactory, ResilienceConfigProvider, ResilienceConfigProviderFactory,
};
pub use api::vo::ResilienceConfig;

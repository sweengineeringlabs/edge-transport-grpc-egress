//! `grpc_egress_breaker_api` -- the circuit-breaker domain: the contracts, requests, responses, errors and
//! the breaker policy value. The implementation (the decorator that wraps a `Transport` and a `RawTransport` of
//! `proto-pattern`) is in `grpc-egress-breaker-spi`.
//!
//! ```compile_fail
//! use grpc_egress_breaker_api::vo::GrpcBreakerConfig;
//! ```

#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod dto;
mod error;
mod traits;
mod vo;
pub use dto::{
    BuildGrpcBreakerConfigProviderRequest, BuildGrpcBreakerRequest, ObserveStateRequest,
    ObserveStateResponse,
};
pub use error::{BreakerDomainError, Error};
pub use traits::{
    BreakerObservable, FailureClassifier, GrpcBreakerConfigProvider,
    GrpcBreakerConfigProviderFactory, GrpcBreakerFactory,
};
pub use vo::{
    BreakerState, GrpcBreakerConfig, Outcome, BREAKER_EGRESS_LOG_PREFIX,
    FAILURE_CLASSIFIER_LOG_TARGET, GRPC_BREAKER_CLIENT_LOG_TARGET,
};

/// The contract the interfaces of this crate are written over: an implementation names `CallError`, `Transport` and the
/// rest through the domain crate, and so depends on nothing but the domain crate.
pub use proto_pattern;

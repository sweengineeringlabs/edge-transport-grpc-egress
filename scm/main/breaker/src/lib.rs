//! `grpc_egress_breaker` -- the circuit-breaker domain: the contracts, requests, responses, errors and
//! the breaker policy value. The implementation (the decorator that wraps a `GrpcEgress`) is in
//! `grpc-egress-breaker-std`.

#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod dto;
mod error;
mod traits;
mod vo;

pub use dto::{BuildGrpcBreakerRequest, ObserveStateRequest, ObserveStateResponse};
pub use error::{BreakerDomainError, Error};
pub use traits::{BreakerObservable, GrpcBreakerFactory};
pub use vo::{
    BreakerState, GrpcBreakerConfig, BREAKER_EGRESS_LOG_PREFIX, FAILURE_CLASSIFIER_LOG_TARGET,
    GRPC_BREAKER_CLIENT_LOG_TARGET,
};

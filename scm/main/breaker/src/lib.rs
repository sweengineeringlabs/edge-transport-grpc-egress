//! `grpc_egress_breaker` -- the circuit-breaker domain: the contracts, requests, responses, errors and
//! the breaker policy value. The implementation (the decorator that wraps a `Transport` and a `RawTransport` of
//! `proto-pattern`) is in `grpc-egress-breaker-std`.

#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod api;

pub use api::dto::{BuildGrpcBreakerRequest, ObserveStateRequest, ObserveStateResponse};
pub use api::error::{BreakerDomainError, Error};
pub use api::traits::{BreakerObservable, GrpcBreakerFactory};
pub use api::vo::{
    BreakerState, GrpcBreakerConfig, BREAKER_EGRESS_LOG_PREFIX, FAILURE_CLASSIFIER_LOG_TARGET,
    GRPC_BREAKER_CLIENT_LOG_TARGET,
};

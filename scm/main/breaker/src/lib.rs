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

pub use dto::{
    AdmitRequest, AdmitResponse, ClassifyRequest, ClassifyResponse, ConfigBuilderRequest,
    ConfigBuilderResponse, ConfigValidationRequest, DescribeRequest, DescribeResponse,
    ObserveStateRequest, ObserveStateResponse, RecordOutcomeRequest, RecordOutcomeResponse,
    WrapBreakerRequest, WrapBreakerResponse,
};
pub use error::{BreakerDomainError, Error};
pub use traits::{
    BreakerDecorator, BreakerObservable, BreakerTransition, ConfigBuilderProvider,
    FailureClassifier, Processor, Validator,
};
pub use vo::{
    Admission, ApplicationConfigBuilder, BreakerState, GrpcBreakerConfig, Outcome,
    BREAKER_DECORATOR_LABEL, BREAKER_EGRESS_LOG_PREFIX, CONFIG_BUILDER_PROVIDER_SECTION,
    FAILURE_CLASSIFIER_LOG_TARGET, GRPC_BREAKER_CLIENT_LOG_TARGET,
};

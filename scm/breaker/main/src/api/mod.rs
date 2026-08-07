//! API layer — public schema + trait declarations + public types.
//!
//! Per SEA rule 160, public type *declarations* live here.  Impl
//! blocks live in `core/`.
//!
//! This crate has a single cohesive domain (the circuit breaker), so all
//! contracts live in one theme: `api::breaker`.

mod breaker;

pub use breaker::{
    Admission, AdmitRequest, AdmitResponse, ApplicationConfigBuilder, BreakerDecorator,
    BreakerDomainError, BreakerObservable, BreakerState, BreakerTransition, ClassifyRequest,
    ClassifyResponse, ConfigBuilderProvider, ConfigBuilderRequest, ConfigBuilderResponse,
    ConfigValidationRequest, DescribeRequest, DescribeResponse, Error, FailureClassifier,
    GrpcBreakerClient, GrpcBreakerConfig, GrpcBreakerFacade, GrpcBreakerSvc, ObserveStateRequest,
    ObserveStateResponse, Outcome, Processor, RecordOutcomeRequest, RecordOutcomeResponse,
    Validator, WrapBreakerRequest, WrapBreakerResponse, BREAKER_DECORATOR_LABEL,
    BREAKER_EGRESS_LOG_PREFIX, CONFIG_BUILDER_PROVIDER_SECTION, FAILURE_CLASSIFIER_LOG_TARGET,
    GRPC_BREAKER_CLIENT_LOG_TARGET,
};

pub(crate) use breaker::BreakerNode;

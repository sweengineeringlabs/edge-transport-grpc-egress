//! Circuit-breaker theme — the only domain in this crate. All contracts,
//! value types, and errors live here as one cohesive vertical slice.

pub mod breaker_decorator;
pub mod breaker_egress;
pub mod config_builder_provider;
pub mod errors;
pub mod failure_classifier;
pub mod grpc;
pub mod traits;
pub mod types;

pub use breaker_decorator::BREAKER_DECORATOR_LABEL;
pub use breaker_egress::BREAKER_EGRESS_LOG_PREFIX;
pub use config_builder_provider::CONFIG_BUILDER_PROVIDER_SECTION;
pub use failure_classifier::FAILURE_CLASSIFIER_LOG_TARGET;
pub use grpc::GRPC_BREAKER_CLIENT_LOG_TARGET;

pub use errors::{BreakerDomainError, Error};
pub use traits::{
    BreakerDecorator, BreakerObservable, BreakerTransition, ConfigBuilderProvider,
    FailureClassifier, Processor, Validator,
};
pub use types::{
    Admission, AdmitRequest, AdmitResponse, ApplicationConfigBuilder, BreakerState,
    ClassifyRequest, ClassifyResponse, ConfigBuilderRequest, ConfigBuilderResponse,
    ConfigValidationRequest, DescribeRequest, DescribeResponse, GrpcBreakerClient,
    GrpcBreakerConfig, GrpcBreakerFacade, GrpcBreakerSvc, ObserveStateRequest,
    ObserveStateResponse, Outcome, RecordOutcomeRequest, RecordOutcomeResponse, WrapBreakerRequest,
    WrapBreakerResponse,
};

pub(crate) use types::BreakerNode;

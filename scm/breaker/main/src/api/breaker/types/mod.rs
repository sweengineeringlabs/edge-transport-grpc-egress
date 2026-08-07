//! Circuit-breaker value types.

pub mod application_config_builder;
pub mod config_builder_request;
pub mod config_builder_response;
pub mod config_validation_request;
pub mod describe_request;
pub mod describe_response;
pub mod grpc_breaker_client;
pub mod grpc_breaker_config;
pub mod grpc_breaker_facade;
pub mod grpc_breaker_svc;
pub mod observe_state_request;
pub mod observe_state_response;
pub mod wrap_breaker_request;
pub mod wrap_breaker_response;

pub use application_config_builder::ApplicationConfigBuilder;
pub use config_builder_request::ConfigBuilderRequest;
pub use config_builder_response::ConfigBuilderResponse;
pub use config_validation_request::ConfigValidationRequest;
pub use describe_request::DescribeRequest;
pub use describe_response::DescribeResponse;
pub use grpc_breaker_client::GrpcBreakerClient;
pub use grpc_breaker_config::GrpcBreakerConfig;
pub use grpc_breaker_facade::GrpcBreakerFacade;
pub use grpc_breaker_svc::GrpcBreakerSvc;
pub use observe_state_request::ObserveStateRequest;
pub use observe_state_response::ObserveStateResponse;
pub use wrap_breaker_request::WrapBreakerRequest;
pub use wrap_breaker_response::WrapBreakerResponse;

// Moved to edge-transport-breaker-policy (ADR-004/ADR-003, edge-transport-http-egress/
// edge-transport-grpc-egress): the circuit-state machine and its DTOs are protocol-agnostic.
pub use edge_transport_breaker_policy::{
    Admission, AdmitRequest, AdmitResponse, BreakerNode, BreakerState, ClassifyRequest,
    ClassifyResponse, Outcome, RecordOutcomeRequest, RecordOutcomeResponse,
};

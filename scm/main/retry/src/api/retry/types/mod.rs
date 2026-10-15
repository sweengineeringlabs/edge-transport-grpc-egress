//! Types — behavioural type declarations whose impl blocks live in `core/`.

pub mod application_config_builder;
pub mod backoff_schedule;
pub mod backoff_schedule_request;
pub mod backoff_track;
pub mod config_builder_request;
pub mod config_builder_response;
pub mod describe_policy_request;
pub mod describe_policy_response;
pub mod grpc_retry_client;
pub mod grpc_retry_config;
pub mod grpc_retry_config_builder;
pub mod grpc_retry_facade;
pub mod grpc_retry_svc;
pub mod processor_request;
pub mod resource_exhausted_context;
pub mod retry_decision;
pub mod retry_inspect_request;
pub mod retry_inspect_response;
pub mod schedule_response;
pub mod validation_request;

pub use application_config_builder::ApplicationConfigBuilder;
pub use backoff_schedule::BackoffSchedule;
pub use backoff_schedule_request::BackoffScheduleRequest;
pub use backoff_track::BackoffTrack;
pub use config_builder_request::ConfigBuilderRequest;
pub use config_builder_response::ConfigBuilderResponse;
pub use describe_policy_request::DescribePolicyRequest;
pub use describe_policy_response::DescribePolicyResponse;
pub use grpc_retry_client::GrpcRetryClient;
pub use grpc_retry_config::GrpcRetryConfig;
pub use grpc_retry_config_builder::GrpcRetryConfigBuilder;
pub use grpc_retry_facade::GrpcRetryFacade;
pub use grpc_retry_svc::GrpcRetrySvc;
pub use processor_request::ProcessorRequest;
pub use resource_exhausted_context::ResourceExhaustedContext;
pub use retry_decision::RetryDecision;
pub use retry_inspect_request::RetryInspectRequest;
pub use retry_inspect_response::RetryInspectResponse;
pub use schedule_response::ScheduleResponse;
pub use validation_request::ValidationRequest;

// Moved to edge-transport-retry-policy (ADR-003/ADR-004): zero-dependency jitter DTOs.
pub use edge_transport_retry_policy::{NextUnitRequest, NextUnitResponse};

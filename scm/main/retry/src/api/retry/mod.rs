//! API layer — this crate's single domain (gRPC retry).
//!
//! Per SEA rule 160, public type *declarations* live here. Impl blocks
//! live in `core/`. `backoff/` and `grpc/` are nested sub-themes within
//! this domain, matching the equivalent grouping in `core/`; their
//! traits live in the shared `traits/` directory (not nested further)
//! so every type stays within the single `retry` theme subtree.

pub mod backoff;
pub mod errors;
pub mod grpc;
pub mod retry_egress;
pub mod traits;
pub mod types;

pub use backoff::BACKOFF_SCHEDULER_LOG_TARGET;
pub use errors::Error;
pub use grpc::GRPC_RETRY_CLIENT_LOG_TARGET;
pub use retry_egress::RETRY_EGRESS_LOG_TARGET;
pub use traits::{
    BackoffScheduler, ConfigBuilderProvider, JitterRng, Processor, RetryDecorator, RetryInspector,
    Validator,
};
pub use types::{
    ApplicationConfigBuilder, BackoffSchedule, BackoffScheduleRequest, BackoffTrack,
    ConfigBuilderRequest, ConfigBuilderResponse, DescribePolicyRequest, DescribePolicyResponse,
    GrpcRetryClient, GrpcRetryConfig, GrpcRetryConfigBuilder, GrpcRetryFacade, GrpcRetrySvc,
    NextUnitRequest, NextUnitResponse, ProcessorRequest, ResourceExhaustedContext, RetryDecision,
    RetryInspectRequest, RetryInspectResponse, ScheduleResponse, ValidationRequest,
};

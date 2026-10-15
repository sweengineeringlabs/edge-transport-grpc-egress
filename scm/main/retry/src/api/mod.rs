//! API layer — public schema + trait declarations + public types.
//!
//! Per SEA rule 160, public type *declarations* live here.  Impl
//! blocks live in `core/`.
//!
//! This crate has a single cohesive domain (gRPC retry), so all
//! contracts live in one theme: `api::retry`.

mod retry;

pub use retry::{
    ApplicationConfigBuilder, BackoffSchedule, BackoffScheduleRequest, BackoffScheduler,
    BackoffTrack, ConfigBuilderProvider, ConfigBuilderRequest, ConfigBuilderResponse,
    DescribePolicyRequest, DescribePolicyResponse, Error, GrpcRetryClient, GrpcRetryConfig,
    GrpcRetryConfigBuilder, GrpcRetryFacade, GrpcRetrySvc, JitterRng, NextUnitRequest,
    NextUnitResponse, Processor, ProcessorRequest, ResourceExhaustedContext, RetryDecision,
    RetryDecorator, RetryInspectRequest, RetryInspectResponse, RetryInspector, ScheduleResponse,
    ValidationRequest, Validator, BACKOFF_SCHEDULER_LOG_TARGET, GRPC_RETRY_CLIENT_LOG_TARGET,
    RETRY_EGRESS_LOG_TARGET,
};

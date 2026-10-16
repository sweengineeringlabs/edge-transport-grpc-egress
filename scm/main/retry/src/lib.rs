//! `grpc_egress_retry` -- the retry domain: the contracts, requests, responses, errors and the
//! retry policy value. The implementation (the decorator that wraps a `GrpcEgress`) is in
//! `grpc-egress-retry-std`.

#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod dto;
mod error;
mod traits;
mod vo;

pub use dto::{
    BackoffScheduleRequest, ConfigBuilderRequest, ConfigBuilderResponse, DescribePolicyRequest,
    DescribePolicyResponse, NextUnitRequest, NextUnitResponse, ProcessorRequest,
    RetryInspectRequest, RetryInspectResponse, ScheduleResponse, ValidationRequest,
};
pub use error::Error;
pub use traits::{
    BackoffScheduler, ConfigBuilderProvider, JitterRng, Processor, RetryDecorator, RetryInspector,
    Validator,
};
pub use vo::{
    ApplicationConfigBuilder, BackoffTrack, GrpcRetryConfig, BACKOFF_SCHEDULER_LOG_TARGET,
    GRPC_RETRY_CLIENT_LOG_TARGET, RETRY_EGRESS_LOG_TARGET,
};

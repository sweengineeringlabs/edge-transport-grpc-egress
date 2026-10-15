//! `edge_transport_grpc_egress_retry` — decorator that wraps a
//! [`GrpcEgress`](edge_transport_grpc_egress::GrpcEgress) with
//! exponential-backoff-with-jitter retry on gRPC-canonical
//! retryable status codes.
//!
//! ## Retry policy
//!
//! Retries on `Unavailable` and `ResourceExhausted` (both are
//! transient on the gRPC side and have a real chance of
//! succeeding on a fresh attempt).  Never retries
//! `Unauthenticated` / `PermissionDenied` (those don't get better
//! by trying again — they need a credential refresh upstream).
//! Never retries `DeadlineExceeded` (the caller's deadline is
//! the retry budget; eating the deadline on a re-issue is
//! pointless) or `Internal` (server bug — amplifying it just
//! amplifies the bug).
//!
//! ## Deadline budget
//!
//! Total retries are bounded by the caller's deadline carried
//! on [`GrpcRequest::deadline`](edge_transport_grpc_egress::GrpcRequest).
//! Even if the configured `max_attempts` allows more tries, the
//! loop stops when the elapsed time + next backoff would
//! overrun the deadline.
//!
//! ## Composition
//!
//! Wrap any [`GrpcEgress`](edge_transport_grpc_egress::GrpcEgress) implementor via
//! [`GrpcRetryFacade::create_retry_client`] or
//! [`GrpcRetryFacade::wrap_retry`].

#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod api;
mod core;
mod saf;

// Public contracts and value objects — all flow directly from api/.
pub use crate::api::{
    ApplicationConfigBuilder, BackoffSchedule, BackoffScheduleRequest, BackoffScheduler,
    BackoffTrack, ConfigBuilderProvider, ConfigBuilderRequest, ConfigBuilderResponse,
    DescribePolicyRequest, DescribePolicyResponse, Error, GrpcRetryClient, GrpcRetryConfig,
    GrpcRetryConfigBuilder, GrpcRetryFacade, GrpcRetrySvc, JitterRng, NextUnitRequest,
    NextUnitResponse, Processor, ProcessorRequest, ResourceExhaustedContext, RetryDecision,
    RetryDecorator, RetryInspectRequest, RetryInspectResponse, RetryInspector, ScheduleResponse,
    ValidationRequest, Validator,
};
pub use saf::*;

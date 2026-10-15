//! `edge_transport_grpc_egress_breaker` — circuit-breaker decorator
//! around any [`GrpcEgress`](edge_transport_grpc_egress::GrpcEgress).
//!
//! ## State machine
//!
//! Standard three-state breaker:
//!
//! - **Closed**: requests pass through; consecutive
//!   `Unavailable`/`Internal` failures are counted.  At
//!   `failure_threshold` the breaker trips Open.
//! - **Open**: requests short-circuit with
//!   `GrpcEgressError::Unavailable` *without* calling the
//!   inner client.  After `cool_down`, the next request promotes
//!   to HalfOpen.
//! - **HalfOpen**: a small number of probe requests are admitted.
//!   `half_open_probe_count` consecutive successes → Closed.
//!   Any failure during probing → back to Open.
//!
//! ## Failure classification
//!
//! Only `Unavailable` (status or transport variant) and
//! `Internal` count as breaker failures.  `ResourceExhausted` is
//! a rate-limit signal that the retry layer handles, not a
//! reason to shed traffic.  `Unauthenticated` / `PermissionDenied`
//! reflect a bad credential, not an unhealthy upstream — the
//! upstream is fine; the call is just unauthorized.
//!
//! ## Observability
//!
//! State transitions emit `tracing` events at INFO (Open, Closed)
//! and DEBUG (HalfOpen probe).  No internal metrics framework —
//! consumers wire `tracing` into whatever they already use.

#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod api;
mod core;
mod saf;

// Public contracts and value objects — all flow directly from api/.
pub use crate::api::{
    Admission, AdmitRequest, AdmitResponse, ApplicationConfigBuilder, BreakerDecorator,
    BreakerObservable, BreakerState, BreakerTransition, ClassifyRequest, ClassifyResponse,
    ConfigBuilderProvider, ConfigBuilderRequest, ConfigBuilderResponse, ConfigValidationRequest,
    DescribeRequest, DescribeResponse, Error, FailureClassifier, GrpcBreakerClient,
    GrpcBreakerConfig, GrpcBreakerFacade, GrpcBreakerSvc, ObserveStateRequest,
    ObserveStateResponse, Outcome, Processor, RecordOutcomeRequest, RecordOutcomeResponse,
    Validator, WrapBreakerRequest, WrapBreakerResponse,
};
pub use saf::*;

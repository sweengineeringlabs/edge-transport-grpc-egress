//! `grpc_egress_transport` -- the gRPC egress domain: the builder of a client and the options it takes, the
//! interceptor that runs around a call, and the factories of the views a client is wrapped in. What a view needs to know (a
//! signal that ends calls, the identity of the caller, the call an interceptor edits) is an interface here, never a type
//! of a library. A client is a
//! `proto-pattern` `Transport` and `RawTransport`; the calls, their requests, responses, errors, status codes and
//! metadata are the contract's, not this crate's. Plain data and traits only; the implementations are in
//! `grpc-egress-transport-std`.
//!
//! One folder per theme, each with the folders its kinds need (`dto/`, `vo/`, `error/`, `traits/`):
//! `client`, `channel`, `interceptor`, `loadbalancer`, `cancellation` and `context`.

mod api;

pub use api::cancellation::{
    BuildGrpcCancellationRequest, CancellationSignal, GrpcCancellationFactory,
};
pub use api::channel::{
    CompressionMode, GrpcChannelConfig, GrpcChannelConfigError, KeepAliveConfig, MtlsConfig,
    DEFAULT_MAX_MESSAGE_BYTES, DEFAULT_REQUEST_TIMEOUT_SECS,
};
pub use api::client::{BuildGrpcEgressBuilderRequest, GrpcEgressBuilder, GrpcEgressBuilderFactory};
pub use api::context::{BuildGrpcCallerIdentityRequest, CallerIdentity, GrpcCallerIdentityFactory};
pub use api::interceptor::{
    BuildGrpcEgressInterceptorRequest, GrpcEgressInterceptor, GrpcEgressInterceptorFactory,
    InboundReply, OutboundCall, TraceContextSource,
};
pub use api::loadbalancer::{BackendTarget, BalancingStrategy, LoadbalancerSettings};

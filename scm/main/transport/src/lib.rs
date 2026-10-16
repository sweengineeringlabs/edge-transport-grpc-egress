//! `grpc_egress_transport` -- the gRPC egress domain: the `GrpcEgress` trait, the requests and
//! responses it carries, its errors and the options of a client. Plain data and traits only; the
//! implementations are in `grpc-egress-transport-std`.
//!
//! One folder per theme, each with the folders its kinds need (`dto/`, `vo/`, `error/`, `traits/`):
//! `client`, `channel`, `interceptor`, `loadbalancer` and, behind the Cargo feature `prost`, `codec`.

mod channel;
mod client;
#[cfg(feature = "prost")]
mod codec;
mod interceptor;
mod loadbalancer;

pub use channel::{
    CompressionMode, GrpcChannelConfig, GrpcChannelConfigError, KeepAliveConfig, MtlsConfig,
    DEFAULT_MAX_MESSAGE_BYTES, DEFAULT_REQUEST_TIMEOUT_SECS,
};
pub use client::{
    BuildGrpcEgressBuilderRequest, CallStreamRequest, CallUnaryWithContextRequest, GrpcEgress,
    GrpcEgressBuilder, GrpcEgressBuilderFactory, GrpcEgressError, GrpcEgressResult,
    GrpcMessageStreamResponse, GrpcRequest, GrpcResponse, GrpcStatusCode, HealthCheckRequest,
};
#[cfg(feature = "prost")]
pub use codec::GrpcEgressProstCodec;
pub use edge_security_runtime::SecurityContext;
pub use interceptor::{
    AfterCallRequest, BuildGrpcEgressInterceptorRequest, GrpcEgressInterceptor,
    GrpcEgressInterceptorFactory, TraceContextSource,
};
pub use loadbalancer::{BackendTarget, BalancingStrategy, LoadbalancerSettings};

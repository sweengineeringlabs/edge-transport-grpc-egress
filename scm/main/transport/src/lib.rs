//! `grpc_egress_transport` -- the gRPC egress domain: the `GrpcEgress` trait, the requests and
//! responses it carries, its errors and the options of a client. Plain data and traits only; the
//! implementations are in `grpc-egress-transport-std`.
//!
//! One folder per theme, each with the folders its kinds need (`dto/`, `vo/`, `error/`, `traits/`):
//! `client`, `channel`, `interceptor`, `loadbalancer` and, behind the Cargo feature `prost`, `codec`.

mod api;

pub use api::channel::{
    CompressionMode, GrpcChannelConfig, GrpcChannelConfigError, KeepAliveConfig, MtlsConfig,
    DEFAULT_MAX_MESSAGE_BYTES, DEFAULT_REQUEST_TIMEOUT_SECS,
};
pub use api::client::{
    BuildGrpcEgressBuilderRequest, CallStreamRequest, CallUnaryWithContextRequest, GrpcEgress,
    GrpcEgressBuilder, GrpcEgressBuilderFactory, GrpcEgressError, GrpcEgressResult,
    GrpcMessageStreamResponse, GrpcRequest, GrpcResponse, GrpcStatusCode, HealthCheckRequest,
};
#[cfg(feature = "prost")]
pub use api::codec::GrpcEgressProstCodec;
pub use api::interceptor::{
    AfterCallRequest, BuildGrpcEgressInterceptorRequest, GrpcEgressInterceptor,
    GrpcEgressInterceptorFactory, TraceContextSource,
};
pub use api::loadbalancer::{BackendTarget, BalancingStrategy, LoadbalancerSettings};
pub use edge_security_runtime::SecurityContext;

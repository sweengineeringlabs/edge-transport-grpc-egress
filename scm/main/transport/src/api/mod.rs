//! gRPC egress API: one folder per theme, each with `dto/`, `vo/`, `error/` and `traits/` as needed.

mod channel;
mod client;
#[cfg(feature = "prost")]
mod codec;
mod interceptor;
mod loadbalancer;

// Flat re-export surface: the other layers and lib.rs reach api/ items only as `crate::api::TypeName`,
// never through a theme folder. `pub` so that lib.rs can re-export a subset as the public surface;
// the theme modules themselves stay private.
pub use channel::{
    CompressionMode, GrpcChannelConfig, GrpcChannelConfigError, KeepAliveConfig, MtlsConfig,
    DEFAULT_MAX_MESSAGE_BYTES, DEFAULT_REQUEST_TIMEOUT_SECS,
};
pub use client::{
    BuildGrpcEgressBuilderRequest, CallStreamRequest, CallUnaryWithContextRequest, DescribeRequest,
    DescribeResponse, GrpcEgress, GrpcEgressBuilder, GrpcEgressBuilderFactory, GrpcEgressError,
    GrpcEgressResult, GrpcMessageStreamResponse, GrpcRequest, GrpcResponse, GrpcStatusCode,
    HealthCheckRequest, ProcessingRequest, Processor,
};
#[cfg(feature = "prost")]
pub use codec::GrpcEgressProstCodec;
pub use interceptor::{AfterCallRequest, GrpcEgressInterceptor, TraceContextSource};
pub use loadbalancer::{BackendTarget, BalancingStrategy, LoadbalancerSettings};

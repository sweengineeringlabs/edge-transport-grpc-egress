//! `edge_transport_grpc_egress` — gRPC outbound domain.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod api;
mod saf;
mod spi;
mod std;

// Re-export the crate's public contracts and value objects via api/'s own
// flat re-export surface (crate::api::TypeName) -- saf/ carries only
// trait re-exports and genuinely saf-declared composition helpers, never
// a pass-through of types declared elsewhere.
pub use crate::std::{
    ApplicationConfigBuilder, Conversions, GrpcChannelConfigBuilder, GrpcEgressInterceptorChain,
    GrpcRequestBuilder, TraceContextGrpcEgressInterceptor, TransportSvc,
};
#[cfg(feature = "prost")]
pub use api::GrpcEgressProstCodec;
pub use api::{
    AfterCallRequest, BackendTarget, BalancingStrategy, BuildGrpcEgressBuilderRequest,
    CallStreamRequest, CallUnaryWithContextRequest, CompressionMode, DescribeRequest,
    DescribeResponse, GrpcChannelConfig, GrpcChannelConfigError, GrpcEgress, GrpcEgressBuilder,
    GrpcEgressBuilderFactory, GrpcEgressError, GrpcEgressInterceptor, GrpcEgressResult,
    GrpcMessageStreamResponse, GrpcRequest, GrpcResponse, GrpcStatusCode, HealthCheckRequest,
    KeepAliveConfig, LoadbalancerSettings, MtlsConfig, ProcessingRequest, Processor,
    TraceContextSource, DEFAULT_MAX_MESSAGE_BYTES, DEFAULT_REQUEST_TIMEOUT_SECS,
};
pub use edge_security_runtime::SecurityContext;
pub use saf::{
    GrpcEgressFactory, GrpcEgressInterceptorFactory, ProcessorFactory,
    TonicGrpcEgressBuilderSpiFactory, TransportConstruction,
};

#[cfg(feature = "prost")]
pub use saf::GrpcEgressProstCodecFactory;

//! `edge_transport_grpc_egress` — gRPC outbound domain.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod api;
mod core;
mod saf;
mod spi;

// Re-export the crate's public contracts and value objects via api/'s own
// flat re-export surface (crate::api::TypeName) -- saf/ carries only
// trait re-exports and genuinely saf-declared composition helpers, never
// a pass-through of types declared elsewhere.
#[cfg(feature = "prost")]
pub use api::GrpcEgressProstCodec;
pub use api::{
    AfterCallRequest, ApplicationConfigBuilder, CallStreamRequest, CallUnaryWithContextRequest,
    CircuitStateRequest, CircuitStateResponse, CompressionMode, ConfigValidationRequest,
    ConsecutiveFailuresRequest, ConsecutiveFailuresResponse, Conversions, DescribeRequest,
    DescribeResponse, GrpcChannelConfig, GrpcChannelConfigBuilder, GrpcChannelConfigError,
    GrpcClientBuilder, GrpcEgress, GrpcEgressError, GrpcEgressInterceptor,
    GrpcEgressInterceptorChain, GrpcEgressResult, GrpcMessageStreamResponse, GrpcRequest,
    GrpcRequestBuilder, GrpcResponse, GrpcStatusCode, HealthCheckRequest, KeepAliveConfig,
    LastErrorRequest, LastErrorResponse, MtlsConfig, ProcessingRequest, Processor,
    ResilienceConfigBuilder, ResilienceConfigResilienceValidator, ResilienceValidator,
    ResilientGrpcClientPort, TraceContextGrpcEgressInterceptor, TraceContextSource, TransportSvc,
    ValidationRequest, Validator, DEFAULT_MAX_MESSAGE_BYTES,
};
pub use edge_security_runtime::SecurityContext;
pub use saf::{
    GrpcEgressFactory, GrpcEgressInterceptorFactory, ProcessorFactory, ResilienceValidatorFactory,
    ResilientGrpcClientPortFactory, TransportConstruction, ValidatorFactory,
};
pub use swe_edge_loadbalancer::{BackendConfig, BackendPoolInstance, LoadbalancerConfig, Strategy};

#[cfg(feature = "prost")]
pub use saf::GrpcEgressProstCodecFactory;

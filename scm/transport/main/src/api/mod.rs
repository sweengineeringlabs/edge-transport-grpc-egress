//! gRPC egress API — ports, interceptors, and value objects.

mod config;
mod error;
mod traits;
mod types;

// Flat re-export surface: core/, spi/, saf/, and lib.rs may only reach
// api/ items via `crate::api::TypeName` -- never by traversing into a
// submodule (`crate::api::types::TypeName`). This is the one place that
// boundary is crossed; every item any other layer or lib.rs needs must
// be named here exactly once. `pub` (not `pub(crate)`) so lib.rs can
// re-export a subset of these further as the crate's public surface --
// the submodules themselves stay fully private, which is what satisfies
// the "layer mod paths must be private" half of the rule.
pub use error::{GrpcChannelConfigError, GrpcEgressError};
pub use traits::grpc_egress_interceptor::GrpcEgressInterceptor;
pub use traits::resilient_grpc_client_port::ResilientGrpcClientPort;
#[cfg(feature = "prost")]
pub use traits::GrpcEgressProstCodec;
pub use traits::{GrpcEgress, Processor};
pub use types::ApplicationConfigBuilder;
pub use types::{
    AfterCallRequest, CallStreamRequest, CallUnaryWithContextRequest, CircuitStateRequest,
    CircuitStateResponse, CompressionMode, ConsecutiveFailuresRequest, ConsecutiveFailuresResponse,
    Conversions, DescribeRequest, DescribeResponse, GrpcChannelConfig, GrpcChannelConfigBuilder,
    GrpcClientBuilder, GrpcEgressInterceptorChain, GrpcEgressResult, GrpcMessageStreamResponse,
    GrpcRequest, GrpcRequestBuilder, GrpcResponse, GrpcStatusCode, HealthCheckRequest,
    KeepAliveConfig, LastErrorRequest, LastErrorResponse, MtlsConfig, ProcessingRequest,
    TraceContextGrpcEgressInterceptor, TraceContextSource, TransportSvc, DEFAULT_MAX_MESSAGE_BYTES,
    DEFAULT_REQUEST_TIMEOUT_SECS,
};

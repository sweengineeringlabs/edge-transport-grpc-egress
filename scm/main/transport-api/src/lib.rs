//! `grpc_egress_transport_api` -- the gRPC egress domain: the builder of a client and the options it takes, the
//! interceptor that runs around a call, and the factories of the views a client is wrapped in. What a view needs to know (a
//! signal that ends calls, the identity of the caller, the call an interceptor edits) is an interface here, never a type
//! of a library. A client is a
//! `proto-pattern` `Transport` and `RawTransport`; the calls, their requests, responses, errors, status codes and
//! metadata are the contract's, not this crate's. Plain data and traits only; the implementations are in
//! `grpc-egress-transport-std`.
//!
//! One folder per theme, each with the folders its kinds need (`dto/`, `vo/`, `error/`, `traits/`):
//! `client`, `channel`, `interceptor`, `loadbalancer`, `cancellation` and `context`.
//!
//! ```compile_fail
//! use grpc_egress_transport_api::channel::vo::GrpcChannelConfig;
//! ```

mod cancellation;
mod channel;
mod client;
mod context;
mod interceptor;
mod loadbalancer;
pub use cancellation::{BuildGrpcCancellationRequest, CancellationSignal, GrpcCancellationFactory};
pub use channel::{
    BuildGrpcChannelConfigBuilderRequest, BuildMtlsConfigRequest, BuildTlsSettingsResolverRequest,
    CompressionMode, GrpcChannelConfig, GrpcChannelConfigBuilder, GrpcChannelConfigBuilderFactory,
    GrpcChannelConfigError, KeepAliveConfig, MtlsConfig, MtlsConfigFactory,
    ResolveTlsSettingsRequest, TlsSettingsResolver, TlsSettingsResolverFactory,
    DEFAULT_MAX_MESSAGE_BYTES, DEFAULT_REQUEST_TIMEOUT_SECS,
};
pub use client::{
    BuildGrpcEgressBuilderRequest, GrpcEgressBuilder, GrpcEgressBuilderFactory, HealthCheck,
    HealthCheckRequest, HealthCheckResponse, HealthStatus, MetadataExtender,
};
pub use context::{BuildGrpcCallerIdentityRequest, CallerIdentity, GrpcCallerIdentityFactory};
pub use interceptor::{
    BuildGrpcEgressInterceptorChainRequest, BuildGrpcEgressInterceptorRequest,
    GrpcEgressInterceptor, GrpcEgressInterceptorChain, GrpcEgressInterceptorChainFactory,
    GrpcEgressInterceptorFactory, InboundReply, OutboundCall, TraceContextSource,
};
pub use loadbalancer::{
    BackendTarget, BalancingStrategy, BuildLoadbalancerSettingsProviderRequest,
    LoadbalancerSettings, LoadbalancerSettingsProvider, LoadbalancerSettingsProviderFactory,
};

/// The contract the interfaces of this crate are written over: an implementation names `CallError`, `Metadata`,
/// `Transport` and the rest through the domain crate, and so depends on nothing but the domain crate.
pub use proto_pattern;

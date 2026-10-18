//! `grpc_egress_transport_std` -- the implementation of the gRPC egress domain
//! (`grpc-egress-transport`): the client over the `proto-svc` engine, its builder and factory, the interceptor,
//! cancellation and security-context wrappers, the load balancer, the health-check call, and the constructors and
//! builders of the domain's data types.

mod saf;
mod spi;
mod std;

pub use crate::std::{
    GrpcChannelConfigBuilder, GrpcChannelConfigFactory, GrpcEgressInterceptorChain, HealthCheck,
    HealthCheckRequest, HealthCheckResponse, HealthStatus, LoadbalancerSettingsProvider,
    MtlsConfigFactory, SecurityContextIdentity, TlsSettingsResolver, TokenCancellation,
    TraceContextGrpcEgressInterceptor,
};
pub use edge_security_runtime::SecurityContext;
pub use saf::{
    StdGrpcCallerIdentityFactory, StdGrpcCancellationFactory, StdGrpcEgressInterceptorFactory,
    TonicGrpcEgressBuilderSpiFactory,
};

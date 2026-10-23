//! Standard, dependency-light implementations of gRPC egress domain contracts.
//!
//! This package provides standard channel configuration, TLS resolution, and interceptor factories. The client over
//! `proto-svc`, cancellation and load-balancer providers are in `grpc-egress-transport-spi`.
//!
//! ```compile_fail
//! use grpc_egress_transport_std::saf::TonicGrpcEgressBuilderSpiFactory;
//! ```

mod saf;
mod std;

pub use saf::{
    StdGrpcChannelConfigBuilderFactory, StdGrpcEgressInterceptorChainFactory,
    StdGrpcEgressInterceptorFactory, StdMtlsConfigFactory, StdTlsSettingsResolverFactory,
};

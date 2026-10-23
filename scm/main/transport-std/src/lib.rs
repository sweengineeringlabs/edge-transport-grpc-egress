//! `grpc_egress_transport_std` -- the implementation of the gRPC egress domain
//! (`grpc-egress-transport`): the client over the `proto-svc` engine, its builder and factory, the interceptor,
//! cancellation and caller-identity views, and the load balancer.
//!
//! Everything a caller uses is an interface of the domain crate; this crate implements those interfaces on
//! `pub(crate)` types and creates them in `saf/`: a factory is a public type implementing a domain factory trait.
//! `std/` holds the standard implementations (defaults over the domain crate only); `spi/` holds the code that names
//! third-party crates.
//!
//! ```compile_fail
//! use grpc_egress_transport_std::saf::TonicGrpcEgressBuilderSpiFactory;
//! ```

mod saf;
mod spi;
mod std;

pub use saf::{
    StdGrpcCallerIdentityFactory, StdGrpcCancellationFactory, StdGrpcChannelConfigBuilderFactory,
    StdGrpcEgressInterceptorChainFactory, StdGrpcEgressInterceptorFactory,
    StdLoadbalancerSettingsProviderFactory, StdMtlsConfigFactory, StdTlsSettingsResolverFactory,
    TonicGrpcEgressBuilderSpiFactory,
};

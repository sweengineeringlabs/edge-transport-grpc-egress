//! `grpc_egress_transport_std` -- the implementation of the gRPC egress domain
//! (`grpc-egress-transport`): the `hyper` client, its builder and factory, the load balancer, and
//! the constructors and builders of the domain's data types.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod saf;
mod spi;
mod std;

#[cfg(feature = "prost")]
pub use crate::std::ProstUnaryCaller;
pub use crate::std::{
    CompressionModeProvider, Conversions, GrpcChannelConfigBuilder, GrpcChannelConfigFactory,
    GrpcEgressInterceptorChain, GrpcRequestBuilder, GrpcRequestFactory, MtlsConfigFactory,
    TraceContextGrpcEgressInterceptor,
};
pub use saf::{GrpcEgressInterceptorFactory, TonicGrpcEgressBuilderSpiFactory};

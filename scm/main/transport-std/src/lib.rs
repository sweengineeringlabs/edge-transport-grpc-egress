//! `grpc_egress_transport_std` -- the implementation of the gRPC egress domain
//! (`grpc-egress-transport`): the `hyper` client, its builder and factory, the load balancer, and
//! the constructors and builders of the domain's data types.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod saf;
mod spi;
mod std;

pub use crate::std::{
    ApplicationConfigBuilder, CompressionModeProvider, Conversions, GrpcChannelConfigBuilder,
    GrpcChannelConfigFactory, GrpcEgressInterceptorChain, GrpcRequestBuilder, GrpcRequestFactory,
    MtlsConfigFactory, TraceContextGrpcEgressInterceptor, TransportSvc,
};
pub use saf::{
    GrpcEgressFactory, GrpcEgressInterceptorFactory, ProcessorFactory,
    TonicGrpcEgressBuilderSpiFactory, TransportConstruction,
};

#[cfg(feature = "prost")]
pub use saf::GrpcEgressProstCodecFactory;

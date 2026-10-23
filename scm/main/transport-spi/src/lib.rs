//! Provider-backed gRPC egress client over `proto-svc` and the load balancer.
//!
//! Standard channel, TLS, and interceptor implementations live in `grpc-egress-transport-std`; this package uses
//! their domain factory contracts to compose a provider-backed client.
//!
//! ```compile_fail
//! use grpc_egress_transport_spi::saf::TonicGrpcEgressBuilderSpiFactory;
//! ```

#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod saf;
mod spi;

pub use crate::saf::{
    StdGrpcCallerIdentityFactory, StdGrpcCancellationFactory,
    StdLoadbalancerSettingsProviderFactory, TonicGrpcEgressBuilderSpiFactory,
};

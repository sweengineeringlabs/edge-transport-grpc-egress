//! `GrpcEgressBuilderFactory` -- creates the builder of a gRPC egress client.

use crate::{BuildGrpcEgressBuilderRequest, GrpcEgressBuilder};

/// Creates a [`GrpcEgressBuilder`] for the endpoint the factory was created with.
///
/// The factory does nothing else: the options and the work of building are the builder's. Its
/// shape follows `proto_pattern::TransportFactory`, so that moving to the contract changes the
/// types and not the shape.
pub trait GrpcEgressBuilderFactory: Send + Sync {
    /// A new builder with no option set. Each call returns an independent builder.
    fn build(&self, req: BuildGrpcEgressBuilderRequest) -> impl GrpcEgressBuilder + use<Self>;
}

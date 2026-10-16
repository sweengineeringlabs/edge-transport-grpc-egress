//! `traits` of the `client` theme.

pub mod grpc_egress;
pub mod grpc_egress_builder;
pub mod grpc_egress_builder_factory;

pub use grpc_egress::GrpcEgress;
pub use grpc_egress_builder::GrpcEgressBuilder;
pub use grpc_egress_builder_factory::GrpcEgressBuilderFactory;

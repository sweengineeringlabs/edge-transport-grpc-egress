//! `traits` of the `client` theme.

pub mod grpc_egress_builder;
pub mod grpc_egress_builder_factory;
pub mod metadata_extender;

pub use grpc_egress_builder::GrpcEgressBuilder;
pub use grpc_egress_builder_factory::GrpcEgressBuilderFactory;
pub use metadata_extender::MetadataExtender;

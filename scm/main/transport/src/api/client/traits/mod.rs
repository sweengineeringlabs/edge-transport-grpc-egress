//! Trait declarations of the `client` theme.
//!
//! | Trait | Contract |
//! |---|---|
//! | [`GrpcEgressBuilder`] | the options of a client and the call that builds it |
//! | [`GrpcEgressBuilderFactory`] | creates a builder |

pub mod grpc_egress_builder;
pub mod grpc_egress_builder_factory;

pub use grpc_egress_builder::GrpcEgressBuilder;
pub use grpc_egress_builder_factory::GrpcEgressBuilderFactory;

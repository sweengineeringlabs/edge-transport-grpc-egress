//! Domain error types for `edge_transport_grpc_egress_transport`.

pub mod grpc_channel_config_error;
pub mod grpc_egress_error;
pub use grpc_channel_config_error::GrpcChannelConfigError;
pub use grpc_egress_error::GrpcEgressError;

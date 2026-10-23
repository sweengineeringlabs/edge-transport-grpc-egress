//! `dto` of the `client` theme.

pub mod build_grpc_egress_builder_request;
pub mod health_check_request;
pub mod health_check_response;
mod health_proto_codec;

pub use build_grpc_egress_builder_request::BuildGrpcEgressBuilderRequest;
pub use health_check_request::HealthCheckRequest;
pub use health_check_response::HealthCheckResponse;

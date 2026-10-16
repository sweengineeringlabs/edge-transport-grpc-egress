//! API of the `client` theme: making a gRPC call and building the client that makes it.

pub mod dto;
pub mod error;
pub mod traits;
pub mod vo;

pub use dto::{
    BuildGrpcEgressBuilderRequest, CallStreamRequest, CallUnaryWithContextRequest, DescribeRequest,
    DescribeResponse, GrpcMessageStreamResponse, GrpcRequest, GrpcResponse, HealthCheckRequest,
    ProcessingRequest,
};
pub use error::{GrpcEgressError, GrpcEgressResult};
pub use traits::{GrpcEgress, GrpcEgressBuilder, GrpcEgressBuilderFactory, Processor};
pub use vo::GrpcStatusCode;

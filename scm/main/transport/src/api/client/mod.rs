//! API of the `client` theme: building the client that makes a gRPC call.

pub mod dto;
pub mod traits;

pub use dto::BuildGrpcEgressBuilderRequest;
pub use traits::{GrpcEgressBuilder, GrpcEgressBuilderFactory, MetadataExtender};

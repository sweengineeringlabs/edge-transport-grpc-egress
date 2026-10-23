//! API of the `client` theme: building the client that makes a gRPC call.

pub mod dto;
pub mod health_check;
pub mod traits;
pub mod vo;

pub use dto::BuildGrpcEgressBuilderRequest;
pub use dto::{HealthCheckRequest, HealthCheckResponse};
pub use health_check::HealthCheck;
pub use traits::{GrpcEgressBuilder, GrpcEgressBuilderFactory, MetadataExtender};
pub use vo::HealthStatus;

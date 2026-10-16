//! `GrpcEgressResult` — result type alias for gRPC outbound operations.

use crate::api::GrpcEgressError;

/// Result type for gRPC outbound operations.
pub type GrpcEgressResult<T> = Result<T, GrpcEgressError>;

//! gRPC status codes (mirrors tonic/gRPC standard codes).

use serde::{Deserialize, Serialize};

/// A gRPC status code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrpcStatusCode {
    /// Not an error; returned on success.
    Ok,
    /// The operation was cancelled (typically by the caller).
    Cancelled,
    /// Unknown error.
    Unknown,
    /// Client specified an invalid argument.
    InvalidArgument,
    /// Deadline expired before operation could complete.
    DeadlineExceeded,
    /// Requested entity was not found.
    NotFound,
    /// Entity that a client attempted to create already exists.
    AlreadyExists,
    /// Caller does not have permission to execute the operation.
    PermissionDenied,
    /// Some resource has been exhausted.
    ResourceExhausted,
    /// Operation was rejected because the system is not in a state required for execution.
    FailedPrecondition,
    /// Operation was aborted, typically due to a concurrency issue.
    Aborted,
    /// Operation was attempted past the valid range.
    OutOfRange,
    /// Operation is not implemented or not supported.
    Unimplemented,
    /// Internal error.
    Internal,
    /// Service is currently unavailable.
    Unavailable,
    /// Unrecoverable data loss or corruption.
    DataLoss,
    /// Request does not have valid authentication credentials.
    Unauthenticated,
}

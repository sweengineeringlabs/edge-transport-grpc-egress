//! `Conversions` — status-code conversions; the mappings are in `conversions_impl.rs`.

use grpc_egress_transport::GrpcStatusCode;

/// Marker type identifying this as the status code conversion interface module.
pub struct Conversions;

impl Conversions {
    /// Parse a numeric `grpc-status` wire value into a [`GrpcStatusCode`].  Covers all 17 codes.
    ///
    /// Returns [`GrpcStatusCode::Unknown`] for any unrecognized code.
    pub(crate) fn from_wire(value: i32) -> GrpcStatusCode {
        match value {
            0 => GrpcStatusCode::Ok,
            1 => GrpcStatusCode::Cancelled,
            2 => GrpcStatusCode::Unknown,
            3 => GrpcStatusCode::InvalidArgument,
            4 => GrpcStatusCode::DeadlineExceeded,
            5 => GrpcStatusCode::NotFound,
            6 => GrpcStatusCode::AlreadyExists,
            7 => GrpcStatusCode::PermissionDenied,
            8 => GrpcStatusCode::ResourceExhausted,
            9 => GrpcStatusCode::FailedPrecondition,
            10 => GrpcStatusCode::Aborted,
            11 => GrpcStatusCode::OutOfRange,
            12 => GrpcStatusCode::Unimplemented,
            13 => GrpcStatusCode::Internal,
            14 => GrpcStatusCode::Unavailable,
            15 => GrpcStatusCode::DataLoss,
            16 => GrpcStatusCode::Unauthenticated,
            _ => GrpcStatusCode::Unknown,
        }
    }
}

//! `Conversions` — status-code conversions; the mappings are in `conversions_impl.rs`.

use grpc_egress_transport::GrpcStatusCode;

/// Marker type identifying this as the status code conversion interface module.
pub struct Conversions;

impl Conversions {
    /// Convert a [`tonic::Code`] into [`GrpcStatusCode`].  Covers all 17 variants.
    pub(crate) fn from_tonic_code(code: tonic::Code) -> GrpcStatusCode {
        match code {
            tonic::Code::Ok => GrpcStatusCode::Ok,
            tonic::Code::Cancelled => GrpcStatusCode::Cancelled,
            tonic::Code::Unknown => GrpcStatusCode::Unknown,
            tonic::Code::InvalidArgument => GrpcStatusCode::InvalidArgument,
            tonic::Code::DeadlineExceeded => GrpcStatusCode::DeadlineExceeded,
            tonic::Code::NotFound => GrpcStatusCode::NotFound,
            tonic::Code::AlreadyExists => GrpcStatusCode::AlreadyExists,
            tonic::Code::PermissionDenied => GrpcStatusCode::PermissionDenied,
            tonic::Code::ResourceExhausted => GrpcStatusCode::ResourceExhausted,
            tonic::Code::FailedPrecondition => GrpcStatusCode::FailedPrecondition,
            tonic::Code::Aborted => GrpcStatusCode::Aborted,
            tonic::Code::OutOfRange => GrpcStatusCode::OutOfRange,
            tonic::Code::Unimplemented => GrpcStatusCode::Unimplemented,
            tonic::Code::Internal => GrpcStatusCode::Internal,
            tonic::Code::Unavailable => GrpcStatusCode::Unavailable,
            tonic::Code::DataLoss => GrpcStatusCode::DataLoss,
            tonic::Code::Unauthenticated => GrpcStatusCode::Unauthenticated,
        }
    }

    /// Parse a numeric `grpc-status` wire value into a [`GrpcStatusCode`].
    ///
    /// Returns [`GrpcStatusCode::Unknown`] for any unrecognized code.
    pub(crate) fn from_wire(value: i32) -> GrpcStatusCode {
        Conversions::from_tonic_code(tonic::Code::from(value))
    }
}

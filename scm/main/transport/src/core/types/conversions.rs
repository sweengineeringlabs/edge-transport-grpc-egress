//! Exhaustive bidirectional mapping between [`GrpcStatusCode`], `tonic::Code`,
//! and the gRPC wire status integer.
//!
//! All 17 standard gRPC status codes are covered.  Adding or removing a
//! variant on either side will fail to compile here.

use crate::api::{Conversions, GrpcStatusCode};

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

#[cfg(test)]
impl Conversions {
    fn to_tonic_code(code: GrpcStatusCode) -> tonic::Code {
        match code {
            GrpcStatusCode::Ok => tonic::Code::Ok,
            GrpcStatusCode::Cancelled => tonic::Code::Cancelled,
            GrpcStatusCode::Unknown => tonic::Code::Unknown,
            GrpcStatusCode::InvalidArgument => tonic::Code::InvalidArgument,
            GrpcStatusCode::DeadlineExceeded => tonic::Code::DeadlineExceeded,
            GrpcStatusCode::NotFound => tonic::Code::NotFound,
            GrpcStatusCode::AlreadyExists => tonic::Code::AlreadyExists,
            GrpcStatusCode::PermissionDenied => tonic::Code::PermissionDenied,
            GrpcStatusCode::ResourceExhausted => tonic::Code::ResourceExhausted,
            GrpcStatusCode::FailedPrecondition => tonic::Code::FailedPrecondition,
            GrpcStatusCode::Aborted => tonic::Code::Aborted,
            GrpcStatusCode::OutOfRange => tonic::Code::OutOfRange,
            GrpcStatusCode::Unimplemented => tonic::Code::Unimplemented,
            GrpcStatusCode::Internal => tonic::Code::Internal,
            GrpcStatusCode::Unavailable => tonic::Code::Unavailable,
            GrpcStatusCode::DataLoss => tonic::Code::DataLoss,
            GrpcStatusCode::Unauthenticated => tonic::Code::Unauthenticated,
        }
    }

    fn to_wire(code: GrpcStatusCode) -> i32 {
        Conversions::to_tonic_code(code) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_17: [GrpcStatusCode; 17] = [
        GrpcStatusCode::Ok,
        GrpcStatusCode::Cancelled,
        GrpcStatusCode::Unknown,
        GrpcStatusCode::InvalidArgument,
        GrpcStatusCode::DeadlineExceeded,
        GrpcStatusCode::NotFound,
        GrpcStatusCode::AlreadyExists,
        GrpcStatusCode::PermissionDenied,
        GrpcStatusCode::ResourceExhausted,
        GrpcStatusCode::FailedPrecondition,
        GrpcStatusCode::Aborted,
        GrpcStatusCode::OutOfRange,
        GrpcStatusCode::Unimplemented,
        GrpcStatusCode::Internal,
        GrpcStatusCode::Unavailable,
        GrpcStatusCode::DataLoss,
        GrpcStatusCode::Unauthenticated,
    ];

    #[test]
    fn test_round_trip_through_tonic_code_preserves_all_17_variants() {
        for code in ALL_17 {
            let trip = Conversions::from_tonic_code(Conversions::to_tonic_code(code));
            assert_eq!(trip, code, "round-trip failed for {code:?}");
        }
    }

    #[test]
    fn test_round_trip_through_wire_value_preserves_all_17_variants() {
        for code in ALL_17 {
            let wire = Conversions::to_wire(code);
            let trip = Conversions::from_wire(wire);
            assert_eq!(
                trip, code,
                "wire round-trip failed for {code:?} (wire={wire})"
            );
        }
    }

    #[test]
    fn test_from_wire_returns_unknown_for_out_of_range_value() {
        assert_eq!(Conversions::from_wire(99), GrpcStatusCode::Unknown);
        assert_eq!(Conversions::from_wire(-1), GrpcStatusCode::Unknown);
    }

    #[test]
    fn test_to_wire_matches_canonical_grpc_codes() {
        assert_eq!(Conversions::to_wire(GrpcStatusCode::Ok), 0);
        assert_eq!(Conversions::to_wire(GrpcStatusCode::Cancelled), 1);
        assert_eq!(Conversions::to_wire(GrpcStatusCode::Unknown), 2);
        assert_eq!(Conversions::to_wire(GrpcStatusCode::InvalidArgument), 3);
        assert_eq!(Conversions::to_wire(GrpcStatusCode::DeadlineExceeded), 4);
        assert_eq!(Conversions::to_wire(GrpcStatusCode::Unauthenticated), 16);
    }

    #[test]
    fn test_from_tonic_code_maps_ok_to_ok() {
        assert_eq!(
            Conversions::from_tonic_code(tonic::Code::Ok),
            GrpcStatusCode::Ok
        );
    }

    #[test]
    fn test_to_tonic_code_maps_internal_to_internal() {
        assert_eq!(
            Conversions::to_tonic_code(GrpcStatusCode::Internal),
            tonic::Code::Internal
        );
    }

    #[test]
    fn test_from_wire_maps_zero_to_ok() {
        assert_eq!(Conversions::from_wire(0), GrpcStatusCode::Ok);
    }

    #[test]
    fn test_to_wire_maps_internal_to_13() {
        assert_eq!(Conversions::to_wire(GrpcStatusCode::Internal), 13);
    }
}

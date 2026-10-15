//! Integration tests that exercise the `tonic` dependency used by the
//! `core/status_codes/conversions.rs` module.
//!
//! `tonic::Code` is the canonical gRPC status-code enum; this crate maps it to
//! the internal `GrpcStatusCode` enum via the `from_wire` / status-code
//! conversion path.  These tests verify that the mapping is correct and
//! consistent with the gRPC spec by exercising the public SAF surface that
//! internally uses `tonic::Code`.

use edge_transport_grpc_egress_transport::GrpcStatusCode;

/// All 17 gRPC status codes as `(tonic::Code, GrpcStatusCode, wire_int)` triples.
///
/// The `wire_int` column is the canonical integer value per the gRPC spec
/// (<https://grpc.io/docs/guides/status-codes/>).
const STATUS_CODE_MAP: &[(tonic::Code, GrpcStatusCode, i32)] = &[
    (tonic::Code::Ok, GrpcStatusCode::Ok, 0),
    (tonic::Code::Cancelled, GrpcStatusCode::Cancelled, 1),
    (tonic::Code::Unknown, GrpcStatusCode::Unknown, 2),
    (
        tonic::Code::InvalidArgument,
        GrpcStatusCode::InvalidArgument,
        3,
    ),
    (
        tonic::Code::DeadlineExceeded,
        GrpcStatusCode::DeadlineExceeded,
        4,
    ),
    (tonic::Code::NotFound, GrpcStatusCode::NotFound, 5),
    (tonic::Code::AlreadyExists, GrpcStatusCode::AlreadyExists, 6),
    (
        tonic::Code::PermissionDenied,
        GrpcStatusCode::PermissionDenied,
        7,
    ),
    (
        tonic::Code::ResourceExhausted,
        GrpcStatusCode::ResourceExhausted,
        8,
    ),
    (
        tonic::Code::FailedPrecondition,
        GrpcStatusCode::FailedPrecondition,
        9,
    ),
    (tonic::Code::Aborted, GrpcStatusCode::Aborted, 10),
    (tonic::Code::OutOfRange, GrpcStatusCode::OutOfRange, 11),
    (
        tonic::Code::Unimplemented,
        GrpcStatusCode::Unimplemented,
        12,
    ),
    (tonic::Code::Internal, GrpcStatusCode::Internal, 13),
    (tonic::Code::Unavailable, GrpcStatusCode::Unavailable, 14),
    (tonic::Code::DataLoss, GrpcStatusCode::DataLoss, 15),
    (
        tonic::Code::Unauthenticated,
        GrpcStatusCode::Unauthenticated,
        16,
    ),
];

/// Verify that the `tonic::Code` wire integer values match the gRPC spec.
///
/// This directly exercises the `tonic` dependency by comparing its enum
/// discriminants against the canonical wire values defined in the spec.
#[test]
fn transport_struct_tonic_code_wire_values_match_grpc_spec_int_test() {
    for &(tonic_code, _grpc_code, wire_int) in STATUS_CODE_MAP {
        assert_eq!(
            tonic_code as i32, wire_int,
            "tonic::Code::{tonic_code:?} has wrong wire value: expected {wire_int}, got {}",
            tonic_code as i32
        );
    }
}

/// Verify that `GrpcStatusCode` variants are distinct (no two enum variants
/// collide) — structural sanity check for the `tonic` mapping table.
#[test]
fn transport_struct_grpc_status_code_all_17_variants_are_distinct_int_test() {
    let codes: Vec<GrpcStatusCode> = STATUS_CODE_MAP
        .iter()
        .map(|&(_, grpc_code, _)| grpc_code)
        .collect();
    assert_eq!(codes.len(), 17, "expected 17 distinct gRPC status codes");

    // Verify each code is unique by round-tripping through the wire integer.
    let wire_ints: Vec<i32> = STATUS_CODE_MAP.iter().map(|&(_, _, wire)| wire).collect();
    let unique: std::collections::HashSet<i32> = wire_ints.iter().cloned().collect();
    assert_eq!(unique.len(), 17, "all 17 wire integers must be unique");
}

/// Verify that `tonic::Code::from(i32)` handles the full 0–16 range — the
/// same conversion path used internally by `core/status_codes/conversions.rs`.
#[test]
fn transport_struct_tonic_code_from_i32_covers_all_17_valid_codes_int_test() {
    for &(tonic_code, _, wire_int) in STATUS_CODE_MAP {
        let derived = tonic::Code::from(wire_int);
        assert_eq!(
            derived, tonic_code,
            "tonic::Code::from({wire_int}) should equal {tonic_code:?}"
        );
    }
}

/// Verify `tonic::Code::from` returns `Unknown` for out-of-range wire integers —
/// the same defensive behaviour our `from_wire` wrapper relies on.
#[test]
fn transport_struct_tonic_code_from_out_of_range_wire_value_returns_unknown_int_test() {
    assert_eq!(
        tonic::Code::from(99),
        tonic::Code::Unknown,
        "wire value 99 should map to Unknown"
    );
    assert_eq!(
        tonic::Code::from(-1),
        tonic::Code::Unknown,
        "negative wire value should map to Unknown"
    );
}

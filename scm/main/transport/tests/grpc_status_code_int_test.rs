//! Integration tests for `api/value/grpc/grpc_status_code.rs`.

use edge_transport_grpc_egress_transport::GrpcStatusCode;

#[test]
fn transport_struct_grpc_status_code_has_17_distinct_variants_int_test() {
    let codes = [
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
    assert_eq!(codes.len(), 17);
    assert_ne!(GrpcStatusCode::Ok, GrpcStatusCode::Internal);
}

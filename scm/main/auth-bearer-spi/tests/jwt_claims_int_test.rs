//! Integration tests covering the error of the bearer crate.

use grpc_egress_auth_bearer_api::BearerAuthError;

/// @covers: BearerAuthError::InvalidSystemTime
#[test]
fn test_bearer_auth_error_implements_std_error() {
    let e = BearerAuthError::InvalidSystemTime;
    let _ = format!("{e}");
    let _: &dyn std::error::Error = &e;
}

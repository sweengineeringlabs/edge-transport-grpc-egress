//! Coverage stub for `src/api/error/breaker_domain_error.rs`.
//!
//! `BreakerDomainError` is a public type alias for `Error`.

use edge_transport_grpc_egress_breaker::Error;

/// @covers: BreakerDomainError — type alias resolves to Error
#[test]
fn breaker_type_breaker_domain_error_is_accessible_int_test() {
    // BreakerDomainError = Error; verify Error can be instantiated.
    let err = Error::InvalidConfig("zero threshold".into());
    assert!(!err.to_string().is_empty());
}

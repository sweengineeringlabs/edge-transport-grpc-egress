//! Coverage stub for `Error` (re-exported from `edge_transport_breaker_policy::BreakerError`).
//!
//! `Error`'s Display messages no longer embed this crate's name — the type moved to
//! `edge-transport-breaker-policy` (ADR-004/ADR-003), shared across both protocols, so a
//! protocol-specific crate name in the message would be actively wrong now. These tests assert
//! the new generic "breaker policy:" prefix and that the underlying detail string still surfaces.

use edge_transport_grpc_egress_breaker::Error;

/// @covers: Error::ParseFailed — display includes the parse-failure detail
#[test]
fn breaker_enum_error_parse_failed_display_includes_detail_int_test() {
    let err = Error::ParseFailed("missing field `failure_threshold`".into());
    let s = err.to_string();
    assert!(
        s.contains("breaker policy") && s.contains("missing field `failure_threshold`"),
        "expected generic prefix + detail in display, got: {s}",
    );
}

/// @covers: Error::InvalidConfig — display includes the validation detail
#[test]
fn breaker_enum_error_invalid_config_display_includes_detail_int_test() {
    let err = Error::InvalidConfig("failure_threshold must be >= 1".into());
    let s = err.to_string();
    assert!(
        s.contains("breaker policy") && s.contains("failure_threshold must be >= 1"),
        "expected generic prefix + detail in display, got: {s}",
    );
}

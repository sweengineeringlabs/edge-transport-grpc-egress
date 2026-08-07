//! Integration tests for [`Error`] (re-exported from `edge_transport_retry_policy::RetryError`).
//!
//! `Error`'s Display messages no longer embed this crate's name — the type moved to
//! `edge-transport-retry-policy` (ADR-003/ADR-004), shared across both protocols, so a
//! protocol-specific crate name in the message would be actively wrong now. These tests assert
//! the new generic "retry policy:" prefix and that the underlying detail string still surfaces.

use edge_transport_grpc_egress_retry::Error;

/// @covers: ParseFailed
#[test]
fn test_parse_failed_display_names_prefix_and_reason() {
    let err = Error::ParseFailed("missing field `max_attempts`".into());
    let s = err.to_string();
    assert!(s.contains("retry policy"), "missing generic prefix: {s}");
    assert!(s.contains("max_attempts"), "missing field name: {s}");
}

/// @covers: InvalidConfig
#[test]
fn test_invalid_config_display_includes_prefix() {
    let err = Error::InvalidConfig("backoff_multiplier must be > 0".into());
    let s = err.to_string();
    assert!(s.contains("retry policy"), "missing generic prefix: {s}");
    assert!(s.contains("backoff_multiplier"), "missing field name: {s}");
}

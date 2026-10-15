//! Coverage stub for `src/api/error/resilient_transport_error.rs`.

use edge_transport_grpc_egress_resilient::ResilientTransportError;

/// @covers: ResilientTransportError::InvalidResilience — display is non-empty
#[test]
fn resilient_enum_resilient_transport_error_invalid_resilience_display_is_non_empty_int_test() {
    let err = ResilientTransportError::InvalidResilience("max_attempts must be >= 1".into());
    let s = err.to_string();
    assert!(
        s.contains("invalid resilience config"),
        "expected 'invalid resilience config' in display, got: {s}",
    );
}

/// @covers: ResilientTransportError::InvalidResilience — carries the original message
#[test]
fn resilient_enum_resilient_transport_error_invalid_resilience_carries_message_int_test() {
    let err = ResilientTransportError::InvalidResilience("jitter_factor out of range".into());
    let s = err.to_string();
    assert!(
        s.contains("jitter_factor out of range"),
        "expected the original violation message in display, got: {s}",
    );
}

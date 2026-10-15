//! Integration tests for metadata key constants.

use edge_transport_grpc_egress_auth_bearer::{AUTHORIZATION_HEADER, EXTRACTED_BEARER_SUBJECT};

/// @covers: AUTHORIZATION_HEADER
#[test]
fn test_authorization_header_is_lowercase() {
    assert_eq!(AUTHORIZATION_HEADER, "authorization");
}

/// @covers: EXTRACTED_BEARER_SUBJECT
#[test]
fn test_extracted_bearer_subject_has_x_edge_prefix() {
    assert!(
        EXTRACTED_BEARER_SUBJECT.starts_with("x-edge-"),
        "expected x-edge- prefix, got: {}",
        EXTRACTED_BEARER_SUBJECT
    );
}

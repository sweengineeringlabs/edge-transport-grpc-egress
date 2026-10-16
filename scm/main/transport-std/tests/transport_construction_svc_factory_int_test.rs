//! Integration tests for `TransportConstruction`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use grpc_egress_transport::{GrpcChannelConfig, GrpcChannelConfigError};
use grpc_egress_transport_std::GrpcChannelConfigFactory;
use grpc_egress_transport_std::TransportConstruction;

/// @covers: create_tonic_client_from_config
#[test]
fn test_create_tonic_client_from_config_valid_plaintext_config_happy() {
    let allowed = GrpcChannelConfig::new("http://127.0.0.1:50051").allow_plaintext();
    assert!(TransportConstruction::create_tonic_client_from_config(&allowed).is_ok());

    // Negative counterpart in the same test: the same endpoint without
    // allow_plaintext() must still be rejected.
    let rejected = GrpcChannelConfig::new("http://127.0.0.1:50051");
    assert!(TransportConstruction::create_tonic_client_from_config(&rejected).is_err());
}

/// @covers: create_tonic_client_from_config
#[test]
fn test_create_tonic_client_from_config_rejects_plaintext_when_tls_required_error() {
    let cfg = GrpcChannelConfig::new("http://127.0.0.1:50051");
    assert!(matches!(
        TransportConstruction::create_tonic_client_from_config(&cfg),
        Err(GrpcChannelConfigError::PlaintextRejected(_))
    ));
}

/// @covers: create_tonic_client_from_config
#[test]
fn test_create_tonic_client_from_config_https_endpoint_needs_no_opt_in_edge() {
    let https = GrpcChannelConfig::new("https://127.0.0.1:50051");
    assert!(TransportConstruction::create_tonic_client_from_config(&https).is_ok());

    // Contrast in the same test: the equivalent http:// endpoint without
    // allow_plaintext() must still be rejected.
    let http = GrpcChannelConfig::new("http://127.0.0.1:50051");
    assert!(TransportConstruction::create_tonic_client_from_config(&http).is_err());
}

/// @covers: create_transport_from_config
#[test]
fn test_create_transport_from_config_valid_plaintext_config_happy() {
    let allowed = GrpcChannelConfig::new("http://127.0.0.1:50051").allow_plaintext();
    assert!(TransportConstruction::create_transport_from_config(&allowed).is_ok());

    // Negative counterpart in the same test: the same endpoint without
    // allow_plaintext() must still be rejected.
    let rejected = GrpcChannelConfig::new("http://127.0.0.1:50051");
    assert!(TransportConstruction::create_transport_from_config(&rejected).is_err());
}

/// @covers: create_transport_from_config
#[test]
fn test_create_transport_from_config_rejects_plaintext_when_tls_required_error() {
    let cfg = GrpcChannelConfig::new("http://127.0.0.1:50051");
    assert!(matches!(
        TransportConstruction::create_transport_from_config(&cfg),
        Err(GrpcChannelConfigError::PlaintextRejected(_))
    ));
}

/// @covers: create_transport_from_config
#[test]
fn test_create_transport_from_config_https_endpoint_needs_no_opt_in_edge() {
    let https = GrpcChannelConfig::new("https://127.0.0.1:50051");
    assert!(TransportConstruction::create_transport_from_config(&https).is_ok());

    // Contrast in the same test: the equivalent http:// endpoint without
    // allow_plaintext() must still be rejected.
    let http = GrpcChannelConfig::new("http://127.0.0.1:50051");
    assert!(TransportConstruction::create_transport_from_config(&http).is_err());
}

#[cfg(feature = "prost")]
mod prost_tests {
    use super::*;

    /// @covers: create_prost_transport_from_config
    #[test]
    fn test_create_prost_transport_from_config_valid_plaintext_config_happy() {
        let allowed = GrpcChannelConfig::new("http://127.0.0.1:50051").allow_plaintext();
        assert!(TransportConstruction::create_prost_transport_from_config(&allowed).is_ok());

        // Negative counterpart in the same test: the same endpoint without
        // allow_plaintext() must still be rejected.
        let rejected = GrpcChannelConfig::new("http://127.0.0.1:50051");
        assert!(TransportConstruction::create_prost_transport_from_config(&rejected).is_err());
    }

    /// @covers: create_prost_transport_from_config
    #[test]
    fn test_create_prost_transport_from_config_rejects_plaintext_when_tls_required_error() {
        let cfg = GrpcChannelConfig::new("http://127.0.0.1:50051");
        assert!(matches!(
            TransportConstruction::create_prost_transport_from_config(&cfg),
            Err(GrpcChannelConfigError::PlaintextRejected(_))
        ));
    }

    /// @covers: create_prost_transport_from_config
    #[test]
    fn test_create_prost_transport_from_config_https_endpoint_needs_no_opt_in_edge() {
        let https = GrpcChannelConfig::new("https://127.0.0.1:50051");
        assert!(TransportConstruction::create_prost_transport_from_config(&https).is_ok());

        // Contrast in the same test: the equivalent http:// endpoint without
        // allow_plaintext() must still be rejected.
        let http = GrpcChannelConfig::new("http://127.0.0.1:50051");
        assert!(TransportConstruction::create_prost_transport_from_config(&http).is_err());
    }
}

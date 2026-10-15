//! Integration tests for `TransportConstruction`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use edge_transport_grpc_egress_transport::{
    GrpcChannelConfig, GrpcChannelConfigError, GrpcEgressError, TransportConstruction,
};

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

/// @covers: create_lb_transport_from_config
#[tokio::test]
async fn test_create_lb_transport_from_config_one_backend_happy() {
    let cfg = swe_edge_loadbalancer::LoadbalancerConfig {
        strategy: swe_edge_loadbalancer::Strategy::RoundRobin,
        backends: vec![swe_edge_loadbalancer::BackendConfig {
            url: "http://127.0.0.1:50051".to_string(),
            weight: 1,
        }],
    };
    assert!(TransportConstruction::create_lb_transport_from_config(cfg).is_ok());
}

/// @covers: create_lb_transport_from_config
#[test]
fn test_create_lb_transport_from_config_rejects_empty_backends_error() {
    let cfg = swe_edge_loadbalancer::LoadbalancerConfig {
        strategy: swe_edge_loadbalancer::Strategy::RoundRobin,
        backends: vec![],
    };
    assert!(matches!(
        TransportConstruction::create_lb_transport_from_config(cfg),
        Err(GrpcEgressError::Unavailable(_))
    ));
}

/// @covers: create_lb_transport_from_config
#[test]
fn test_create_lb_transport_from_config_rejects_invalid_backend_url_edge() {
    let cfg = swe_edge_loadbalancer::LoadbalancerConfig {
        strategy: swe_edge_loadbalancer::Strategy::RoundRobin,
        backends: vec![swe_edge_loadbalancer::BackendConfig {
            url: "!! not a valid url !!".to_string(),
            weight: 1,
        }],
    };
    assert!(matches!(
        TransportConstruction::create_lb_transport_from_config(cfg),
        Err(GrpcEgressError::Unavailable(_))
    ));
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

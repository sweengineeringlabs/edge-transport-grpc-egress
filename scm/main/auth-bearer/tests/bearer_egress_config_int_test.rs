//! Integration tests for BearerEgressConfig.

use edge_transport_grpc_egress_auth_bearer::{BearerEgressConfig, BearerSecret};

/// @covers: BearerEgressConfig
#[test]
fn test_bearer_egress_config_fields_are_accessible() {
    let config = BearerEgressConfig {
        secret: BearerSecret::Hs256 {
            secret: b"test-key".to_vec(),
        },
        issuer: "issuer".into(),
        audience: "audience".into(),
        subject: "subject".into(),
        lifetime_seconds: 600,
    };
    assert_eq!(config.issuer, "issuer");
    assert_eq!(config.audience, "audience");
    assert_eq!(config.subject, "subject");
    assert_eq!(config.lifetime_seconds, 600);
}

/// @covers: BearerEgressConfig
#[test]
fn test_bearer_egress_config_clone_produces_equal_config() {
    let config = BearerEgressConfig {
        secret: BearerSecret::Hs256 {
            secret: b"clone-key".to_vec(),
        },
        issuer: "clone-issuer".into(),
        audience: "clone-audience".into(),
        subject: "clone-subject".into(),
        lifetime_seconds: 300,
    };
    let cloned = config.clone();
    assert_eq!(cloned.issuer, config.issuer);
    assert_eq!(cloned.lifetime_seconds, config.lifetime_seconds);
}

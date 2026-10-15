#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for the Validator trait and BearerEgressConfig validation.

use edge_transport_grpc_egress_auth_bearer::{BearerEgressConfig, BearerSecret};

// Import Validator via gateway re-export path.
// Validator is declared in api/traits/validator.rs and re-exported through gateway.
fn make_valid_config() -> BearerEgressConfig {
    BearerEgressConfig {
        secret: BearerSecret::Hs256 {
            secret: b"test-secret-key".to_vec(),
        },
        issuer: "test-issuer".into(),
        audience: "test-audience".into(),
        subject: "test-subject".into(),
        lifetime_seconds: 300,
    }
}

/// @covers: Validator::validate
#[test]
fn test_validate_valid_config_returns_ok() {
    use edge_transport_grpc_egress_auth_bearer::Validator;
    let config = make_valid_config();
    assert!(
        config.validate().is_ok(),
        "valid config must pass validation"
    );
}

/// @covers: Validator::validate
#[test]
fn test_validate_empty_issuer_returns_err() {
    use edge_transport_grpc_egress_auth_bearer::Validator;
    let mut config = make_valid_config();
    config.issuer = String::new();
    let result = config.validate();
    assert!(result.is_err(), "empty issuer must fail validation");
    assert!(
        result.unwrap_err().to_string().contains("issuer"),
        "error must mention issuer"
    );
}

/// @covers: Validator::validate
#[test]
fn test_validate_empty_audience_returns_err() {
    use edge_transport_grpc_egress_auth_bearer::Validator;
    let mut config = make_valid_config();
    config.audience = String::new();
    let result = config.validate();
    assert!(result.is_err(), "empty audience must fail validation");
    assert!(
        result.unwrap_err().to_string().contains("audience"),
        "error must mention audience"
    );
}

/// @covers: Validator::validate
#[test]
fn test_validate_empty_subject_returns_err() {
    use edge_transport_grpc_egress_auth_bearer::Validator;
    let mut config = make_valid_config();
    config.subject = String::new();
    let result = config.validate();
    assert!(result.is_err(), "empty subject must fail validation");
    assert!(
        result.unwrap_err().to_string().contains("subject"),
        "error must mention subject"
    );
}

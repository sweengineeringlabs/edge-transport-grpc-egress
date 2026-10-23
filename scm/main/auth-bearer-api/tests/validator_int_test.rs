//! Integration tests for the Validator trait and BearerEgressConfig validation.

use grpc_egress_auth_bearer_api::{BearerEgressConfig, BearerSecret, Validator};

// Import Validator via gateway re-export path.
// Validator is declared in api/traits/validator.rs and re-exported through gateway.
fn make_valid_config() -> Result<BearerEgressConfig, grpc_egress_auth_bearer_api::BearerAuthError> {
    BearerEgressConfig::try_new(
        BearerSecret::hs256(b"test-secret-key".to_vec())?,
        "test-issuer",
        "test-audience",
        "test-subject",
        300,
    )
}

/// @covers: Validator::validate
#[test]
fn test_validate_valid_config_returns_ok() -> Result<(), Box<dyn std::error::Error>> {
    let config = make_valid_config()?;
    assert!(
        config.validate().is_ok(),
        "valid config must pass validation"
    );
    assert!(
        BearerEgressConfig::try_new(BearerSecret::hs256(b"k".to_vec())?, "iss", "", "sub", 1)
            .is_err()
    );
    Ok(())
}

/// @covers: Validator::validate
#[test]
fn test_validate_empty_issuer_returns_err() -> Result<(), Box<dyn std::error::Error>> {
    let result =
        BearerEgressConfig::try_new(BearerSecret::hs256(b"k".to_vec())?, "", "aud", "sub", 1);
    assert!(result.is_err(), "empty issuer must fail validation");
    assert!(
        result
            .err()
            .ok_or("the call must fail")?
            .to_string()
            .contains("issuer"),
        "error must mention issuer"
    );
    Ok(())
}

/// @covers: Validator::validate
#[test]
fn test_validate_empty_audience_returns_err() -> Result<(), Box<dyn std::error::Error>> {
    let result =
        BearerEgressConfig::try_new(BearerSecret::hs256(b"k".to_vec())?, "iss", "", "sub", 1);
    assert!(result.is_err(), "empty audience must fail validation");
    assert!(
        result
            .err()
            .ok_or("the call must fail")?
            .to_string()
            .contains("audience"),
        "error must mention audience"
    );
    Ok(())
}

/// @covers: Validator::validate
#[test]
fn test_validate_empty_subject_returns_err() -> Result<(), Box<dyn std::error::Error>> {
    let result =
        BearerEgressConfig::try_new(BearerSecret::hs256(b"k".to_vec())?, "iss", "aud", "", 1);
    assert!(result.is_err(), "empty subject must fail validation");
    assert!(
        result
            .err()
            .ok_or("the call must fail")?
            .to_string()
            .contains("subject"),
        "error must mention subject"
    );
    Ok(())
}

#[test]
fn test_bearer_config_rejects_zero_lifetime_and_invalid_deserialization(
) -> Result<(), Box<dyn std::error::Error>> {
    assert!(BearerEgressConfig::try_new(
        BearerSecret::hs256(b"key".to_vec())?,
        "iss",
        "aud",
        "sub",
        0
    )
    .is_err());
    let text = r#"secret = { kind = "hs256", secret = [107] }
issuer = "iss"
audience = "aud"
subject = "sub"
lifetime_seconds = 0
"#;
    assert!(toml::from_str::<BearerEgressConfig>(text).is_err());
    Ok(())
}

//! Integration tests for BearerEgressConfig.

use grpc_egress_auth_bearer_api::{BearerEgressConfig, BearerSecret};

/// @covers: BearerEgressConfig
#[test]
fn test_bearer_egress_config_fields_are_accessible(
) -> Result<(), grpc_egress_auth_bearer_api::BearerAuthError> {
    let config = BearerEgressConfig::try_new(
        BearerSecret::hs256(b"test-key".to_vec())?,
        "issuer",
        "audience",
        "subject",
        600,
    )?;
    assert_eq!(config.issuer(), "issuer");
    assert_eq!(config.audience(), "audience");
    assert_eq!(config.subject(), "subject");
    assert_eq!(config.lifetime_seconds(), 600);
    Ok(())
}

/// @covers: BearerEgressConfig
#[test]
fn test_bearer_egress_config_clone_produces_equal_config(
) -> Result<(), grpc_egress_auth_bearer_api::BearerAuthError> {
    let config = BearerEgressConfig::try_new(
        BearerSecret::hs256(b"clone-key".to_vec())?,
        "clone-issuer",
        "clone-audience",
        "clone-subject",
        300,
    )?;
    let cloned = config.clone();
    assert_eq!(cloned.issuer(), config.issuer());
    assert_eq!(cloned.lifetime_seconds(), config.lifetime_seconds());
    Ok(())
}

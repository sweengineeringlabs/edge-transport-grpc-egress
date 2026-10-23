//! Integration tests for [`BearerSecret`].

use grpc_egress_auth_bearer::BearerSecret;
use grpc_egress_auth_bearer::BearerSecretGuard as _;

/// @covers: BearerSecret::ct_eq_hs256
#[test]
fn test_ct_eq_hs256_returns_true_for_identical_secrets(
) -> Result<(), grpc_egress_auth_bearer::BearerAuthError> {
    let a = BearerSecret::hs256(b"verysecret".to_vec())?;
    let b = BearerSecret::hs256(b"verysecret".to_vec())?;
    assert!(secret_guard().ct_eq_hs256(&a, &b));
    Ok(())
}

/// @covers: BearerSecret::ct_eq_hs256
#[test]
fn test_ct_eq_hs256_returns_false_for_different_secrets(
) -> Result<(), grpc_egress_auth_bearer::BearerAuthError> {
    let a = BearerSecret::hs256(b"alpha".to_vec())?;
    let b = BearerSecret::hs256(b"beta".to_vec())?;
    assert!(!secret_guard().ct_eq_hs256(&a, &b));
    Ok(())
}

/// @covers: BearerSecret::ct_eq_hs256
#[test]
fn test_ct_eq_hs256_returns_false_for_variant_mismatch(
) -> Result<(), grpc_egress_auth_bearer::BearerAuthError> {
    let a = BearerSecret::hs256(b"x".to_vec())?;
    let b = BearerSecret::rs256(vec![1])?;
    assert!(!secret_guard().ct_eq_hs256(&a, &b));
    Ok(())
}

#[test]
fn test_secret_construction_and_deserialization_reject_empty_key_material() {
    assert!(BearerSecret::hs256(Vec::new()).is_err());
    assert!(BearerSecret::rs256(Vec::new()).is_err());
    assert!(toml::from_str::<BearerSecret>("kind = \"hs256\"\nsecret = []\n").is_err());
    assert!(toml::from_str::<BearerSecret>("kind = \"rs256\"\n").is_err());
}

#[test]
fn test_secret_serde_round_trip_keeps_the_tagged_configuration_shape(
) -> Result<(), Box<dyn std::error::Error>> {
    let secret = BearerSecret::hs256(b"key".to_vec())?;
    let encoded = toml::to_string(&secret)?;
    assert!(encoded.contains("kind = \"hs256\""), "{encoded}");
    let decoded: BearerSecret = toml::from_str(&encoded)?;
    assert_eq!(decoded, secret);
    Ok(())
}

fn secret_guard() -> impl grpc_egress_auth_bearer::BearerSecretGuard {
    use grpc_egress_auth_bearer::BearerSecretGuardFactory;
    grpc_egress_auth_bearer_spi::StdBearerSecretGuardFactory
        .build(grpc_egress_auth_bearer::BuildBearerSecretGuardRequest)
}

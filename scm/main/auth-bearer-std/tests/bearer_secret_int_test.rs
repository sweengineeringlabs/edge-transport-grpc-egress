//! Integration tests for [`BearerSecret`].

use grpc_egress_auth_bearer::BearerSecret;
use grpc_egress_auth_bearer::BearerSecretGuard as _;

/// @covers: BearerSecret::ct_eq_hs256
#[test]
fn test_ct_eq_hs256_returns_true_for_identical_secrets() {
    let a = BearerSecret::Hs256 {
        secret: b"verysecret".to_vec(),
    };
    let b = BearerSecret::Hs256 {
        secret: b"verysecret".to_vec(),
    };
    assert!(secret_guard().ct_eq_hs256(&a, &b));
}

/// @covers: BearerSecret::ct_eq_hs256
#[test]
fn test_ct_eq_hs256_returns_false_for_different_secrets() {
    let a = BearerSecret::Hs256 {
        secret: b"alpha".to_vec(),
    };
    let b = BearerSecret::Hs256 {
        secret: b"beta".to_vec(),
    };
    assert!(!secret_guard().ct_eq_hs256(&a, &b));
}

/// @covers: BearerSecret::ct_eq_hs256
#[test]
fn test_ct_eq_hs256_returns_false_for_variant_mismatch() {
    let a = BearerSecret::Hs256 {
        secret: b"x".to_vec(),
    };
    let b = BearerSecret::Rs256 {
        private_pem: vec![],
    };
    assert!(!secret_guard().ct_eq_hs256(&a, &b));
}

fn secret_guard() -> impl grpc_egress_auth_bearer::BearerSecretGuard {
    use grpc_egress_auth_bearer::BearerSecretGuardFactory;
    grpc_egress_auth_bearer_std::StdBearerSecretGuardFactory
        .build(grpc_egress_auth_bearer::BuildBearerSecretGuardRequest)
}

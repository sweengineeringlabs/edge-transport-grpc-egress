//! Integration tests for [`BearerSecret`].

use edge_transport_grpc_egress_auth_bearer::BearerSecret;

/// @covers: BearerSecret::ct_eq_hs256
#[test]
fn test_ct_eq_hs256_returns_true_for_identical_secrets() {
    let a = BearerSecret::Hs256 {
        secret: b"verysecret".to_vec(),
    };
    let b = BearerSecret::Hs256 {
        secret: b"verysecret".to_vec(),
    };
    assert!(a.ct_eq_hs256(&b));
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
    assert!(!a.ct_eq_hs256(&b));
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
    assert!(!a.ct_eq_hs256(&b));
}

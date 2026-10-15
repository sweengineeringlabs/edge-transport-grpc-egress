//! Rule 95 dep-coverage test for the `subtle` crate.
//!
//! `subtle` provides constant-time comparison primitives used in
//! [`BearerSecret::ct_eq_hs256`] to guard against timing-oracle
//! attacks on symmetric shared-secret material.
//!
//! This file exercises that path through the public API so the
//! `subtle` dependency is demonstrably tested, not just linked.

use edge_transport_grpc_egress_auth_bearer::BearerSecret;

/// @covers: subtle::ConstantTimeEq — the dependency's constant-time primitive directly
#[test]
fn bearer_trait_constant_time_eq_reports_equality_int_test() {
    use subtle::ConstantTimeEq;
    let a: &[u8] = b"identical";
    let same: &[u8] = b"identical";
    let diff: &[u8] = b"different";
    assert_eq!(a.ct_eq(same).unwrap_u8(), 1);
    assert_eq!(a.ct_eq(diff).unwrap_u8(), 0);
}

/// @covers: subtle::ConstantTimeEq — equal secrets compare as equal
#[test]
fn bearer_struct_bearer_secret_ct_eq_hs256_equal_secrets_returns_true_int_test() {
    let a = BearerSecret::Hs256 {
        secret: b"shared-secret-value".to_vec(),
    };
    let b = BearerSecret::Hs256 {
        secret: b"shared-secret-value".to_vec(),
    };
    assert!(
        a.ct_eq_hs256(&b),
        "identical HS256 secrets must compare as equal via subtle::ConstantTimeEq",
    );
}

/// @covers: subtle::ConstantTimeEq — unequal secrets compare as unequal
#[test]
fn bearer_struct_bearer_secret_ct_eq_hs256_different_secrets_returns_false_int_test() {
    let a = BearerSecret::Hs256 {
        secret: b"correct-secret".to_vec(),
    };
    let b = BearerSecret::Hs256 {
        secret: b"wrong-secret".to_vec(),
    };
    assert!(
        !a.ct_eq_hs256(&b),
        "different HS256 secrets must compare as unequal via subtle::ConstantTimeEq",
    );
}

/// @covers: subtle::ConstantTimeEq — variant mismatch returns false (no timing leak)
#[test]
fn bearer_struct_bearer_secret_ct_eq_hs256_variant_mismatch_returns_false_int_test() {
    let hs = BearerSecret::Hs256 {
        secret: b"secret".to_vec(),
    };
    let rs = BearerSecret::Rs256 {
        private_pem: b"pem-bytes".to_vec(),
    };
    assert!(
        !hs.ct_eq_hs256(&rs),
        "cross-variant comparison must return false — algorithm mismatch is never equal",
    );
}

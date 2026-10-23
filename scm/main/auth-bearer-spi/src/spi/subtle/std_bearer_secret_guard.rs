//! `StdBearerSecretGuard` -- the constant-time comparison of [`BearerSecret`], with `subtle`.

use grpc_egress_auth_bearer::{BearerSecret, BearerSecretGuard};
use subtle::ConstantTimeEq;

/// Constant-time comparison of secret material.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct StdBearerSecretGuard;

impl BearerSecretGuard for StdBearerSecretGuard {
    fn ct_eq_hs256(&self, left: &BearerSecret, right: &BearerSecret) -> bool {
        match (left.hs256_secret(), right.hs256_secret()) {
            (Some(a), Some(b)) => a.ct_eq(b).into(),
            _ => false,
        }
    }
}

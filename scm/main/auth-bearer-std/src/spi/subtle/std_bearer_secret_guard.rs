//! `StdBearerSecretGuard` -- the constant-time comparison of [`BearerSecret`], with `subtle`.

use grpc_egress_auth_bearer::{BearerSecret, BearerSecretGuard};
use subtle::ConstantTimeEq;

/// Constant-time comparison of secret material.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct StdBearerSecretGuard;

impl BearerSecretGuard for StdBearerSecretGuard {
    fn ct_eq_hs256(&self, left: &BearerSecret, right: &BearerSecret) -> bool {
        match (left, right) {
            (BearerSecret::Hs256 { secret: a }, BearerSecret::Hs256 { secret: b }) => {
                a.as_slice().ct_eq(b.as_slice()).into()
            }
            _ => false,
        }
    }
}

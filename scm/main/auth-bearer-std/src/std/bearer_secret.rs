//! [`BearerSecretGuard`] -- the constant-time comparison of [`BearerSecret`], which is plain data in
//! the domain crate.

use grpc_egress_auth_bearer::BearerSecret;
use subtle::ConstantTimeEq;

/// Constant-time comparison of secret material. Bring the trait into scope to call
/// `secret.ct_eq_hs256(&other)`.
pub trait BearerSecretGuard {
    /// Constant-time equality on HS256 secrets. Returns `false` for different variants:
    /// an algorithm mismatch is never "equal".
    fn ct_eq_hs256(&self, other: &Self) -> bool;
}

impl BearerSecretGuard for BearerSecret {
    fn ct_eq_hs256(&self, other: &Self) -> bool {
        match (self, other) {
            (BearerSecret::Hs256 { secret: a }, BearerSecret::Hs256 { secret: b }) => {
                a.as_slice().ct_eq(b.as_slice()).into()
            }
            _ => false,
        }
    }
}

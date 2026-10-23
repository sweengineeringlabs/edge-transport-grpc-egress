//! `BearerSecretGuard` -- the constant-time comparison of secret material.

use crate::BearerSecret;

/// Constant-time comparison of [`BearerSecret`], which is plain data in this crate. Comparisons of secret material
/// MUST go through this interface and never through `==`.
pub trait BearerSecretGuard {
    /// Constant-time equality on HS256 secrets. Returns `false` for different variants: an algorithm mismatch is
    /// never "equal".
    fn ct_eq_hs256(&self, left: &BearerSecret, right: &BearerSecret) -> bool;
}

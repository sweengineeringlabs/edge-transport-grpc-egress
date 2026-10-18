//! `BearerSecretGuardFactory` -- creates the guard of secret comparisons.

use crate::{BearerSecretGuard, BuildBearerSecretGuardRequest};

/// Creates a [`BearerSecretGuard`].
pub trait BearerSecretGuardFactory {
    /// A guard that compares in constant time.
    fn build(&self, req: BuildBearerSecretGuardRequest) -> impl BearerSecretGuard + use<Self>;
}

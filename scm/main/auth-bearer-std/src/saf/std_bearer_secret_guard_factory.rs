//! `StdBearerSecretGuardFactory` -- creates the guard of secret comparisons.

use grpc_egress_auth_bearer::{
    BearerSecretGuard, BearerSecretGuardFactory, BuildBearerSecretGuardRequest,
};

use crate::spi::subtle::StdBearerSecretGuard;

/// Creates guards that compare secrets in constant time.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdBearerSecretGuardFactory;

impl BearerSecretGuardFactory for StdBearerSecretGuardFactory {
    fn build(&self, _req: BuildBearerSecretGuardRequest) -> impl BearerSecretGuard + use<> {
        StdBearerSecretGuard
    }
}

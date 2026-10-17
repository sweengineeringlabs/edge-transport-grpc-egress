//! `StdGrpcCallerIdentityFactory` -- binds a client to the identity of a caller.

use grpc_egress_transport::{BuildGrpcCallerIdentityRequest, GrpcCallerIdentityFactory};
use proto_pattern::{RawTransport, Transport};

use crate::std::ContextTransport;

/// Binds clients to caller identities.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdGrpcCallerIdentityFactory;

impl StdGrpcCallerIdentityFactory {
    /// A factory.
    pub fn new() -> Self {
        Self
    }
}

impl GrpcCallerIdentityFactory for StdGrpcCallerIdentityFactory {
    fn build<'a, 'c, T: Transport + RawTransport>(
        &self,
        req: BuildGrpcCallerIdentityRequest<'a, 'c, T>,
    ) -> impl Transport + RawTransport + use<'a, T> {
        ContextTransport::create(req.inner, req.identity)
    }
}

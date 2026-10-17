//! `StdGrpcSecurityContextFactory` -- binds a client to a security context.

use grpc_egress_transport::{BuildGrpcSecurityContextRequest, GrpcSecurityContextFactory};
use proto_pattern::{RawTransport, Transport};

use crate::std::ContextTransport;

/// Binds clients to security contexts.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdGrpcSecurityContextFactory;

impl StdGrpcSecurityContextFactory {
    /// A factory.
    pub fn new() -> Self {
        Self
    }
}

impl GrpcSecurityContextFactory for StdGrpcSecurityContextFactory {
    fn build<'a, 'c, T: Transport + RawTransport>(
        &self,
        req: BuildGrpcSecurityContextRequest<'a, 'c, T>,
    ) -> impl Transport + RawTransport + use<'a, T> {
        ContextTransport::create(req.inner, req.ctx)
    }
}

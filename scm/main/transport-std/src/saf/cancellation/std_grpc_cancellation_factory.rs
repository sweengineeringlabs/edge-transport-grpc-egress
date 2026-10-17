//! `StdGrpcCancellationFactory` -- binds a client to a cancellation token.

use grpc_egress_transport::{BuildGrpcCancellationRequest, GrpcCancellationFactory};
use proto_pattern::{RawTransport, Transport};

use crate::std::CancellableTransport;

/// Binds clients to cancellation tokens.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdGrpcCancellationFactory;

impl StdGrpcCancellationFactory {
    /// A factory.
    pub fn new() -> Self {
        Self
    }
}

impl GrpcCancellationFactory for StdGrpcCancellationFactory {
    fn build<'a, T: Transport + RawTransport>(
        &self,
        req: BuildGrpcCancellationRequest<'a, T>,
    ) -> impl Transport + RawTransport + use<'a, T> {
        CancellableTransport::create(req.inner, req.token)
    }
}

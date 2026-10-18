//! `StdGrpcCancellationFactory` -- binds a client to a cancellation signal.

use grpc_egress_transport::{
    BuildGrpcCancellationRequest, CancellationSignal, GrpcCancellationFactory,
};
use proto_pattern::{RawTransport, Transport};

use crate::std::CancellableTransport;

/// Binds clients to cancellation signals.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdGrpcCancellationFactory;

impl StdGrpcCancellationFactory {
    /// A factory.
    pub fn new() -> Self {
        Self
    }
}

impl GrpcCancellationFactory for StdGrpcCancellationFactory {
    fn build<'a, T: Transport + RawTransport, S: CancellationSignal + 'static>(
        &self,
        req: BuildGrpcCancellationRequest<'a, T, S>,
    ) -> impl Transport + RawTransport + use<'a, T, S> {
        CancellableTransport::create(req.inner, req.signal)
    }
}

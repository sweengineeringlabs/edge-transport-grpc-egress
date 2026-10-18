//! `TonicGrpcEgressBuilderSpiFactory` -- creates the builder of the client over the `proto-svc` engine.

use crate::spi::client::protosvc::GrpcEgressBuilderImpl;
use grpc_egress_transport::{
    BuildGrpcEgressBuilderRequest, GrpcEgressBuilder, GrpcEgressBuilderFactory,
};

/// Creates builders of the gRPC egress client over the `proto-svc` engine for one endpoint.
///
/// This is the public entry to building a client: hold a factory for the endpoint, call
/// [`GrpcEgressBuilderFactory::build`], set options on the builder it returns, then call
/// [`GrpcEgressBuilder::build`]. The factory only binds the endpoint and hands out builders; the
/// options and the work are the builder's.
pub struct TonicGrpcEgressBuilderSpiFactory {
    endpoint: String,
}

impl TonicGrpcEgressBuilderSpiFactory {
    /// A factory whose builders send to `endpoint`. The endpoint is checked by
    /// [`GrpcEgressBuilder::build`], not here.
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }
}

impl GrpcEgressBuilderFactory for TonicGrpcEgressBuilderSpiFactory {
    fn build(&self, _req: BuildGrpcEgressBuilderRequest) -> impl GrpcEgressBuilder + use<> {
        GrpcEgressBuilderImpl::new(self.endpoint.clone())
    }
}

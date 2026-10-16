//! `TonicGrpcEgressClient` -- what the builder builds: one endpoint or a balanced set.

use futures::future::BoxFuture;

use super::tonic_grpc_egress::TonicGrpcEgress;
use crate::spi::loadbalancer::tonic::TonicLbGrpcEgress;
use grpc_egress_transport::{
    CallStreamRequest, GrpcEgress, GrpcEgressResult, GrpcMessageStreamResponse, GrpcRequest,
    GrpcResponse, HealthCheckRequest,
};

/// The client a [`GrpcEgressBuilder`](grpc_egress_transport::GrpcEgressBuilder) builds: it sends to the one
/// endpoint, or to a backend picked from the load balancer when that option is set. A caller sees
/// one [`GrpcEgress`] either way.
pub(crate) enum TonicGrpcEgressClient {
    /// One endpoint.
    Direct(TonicGrpcEgress),
    /// A set of backends.
    Balanced(TonicLbGrpcEgress),
}

impl GrpcEgress for TonicGrpcEgressClient {
    fn call_unary(&self, request: GrpcRequest) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
        match self {
            Self::Direct(client) => client.call_unary(request),
            Self::Balanced(client) => client.call_unary(request),
        }
    }

    fn call_stream(
        &self,
        req: CallStreamRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcMessageStreamResponse>> {
        match self {
            Self::Direct(client) => client.call_stream(req),
            Self::Balanced(client) => client.call_stream(req),
        }
    }

    fn call_server_stream(
        &self,
        request: GrpcRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcMessageStreamResponse>> {
        match self {
            Self::Direct(client) => client.call_server_stream(request),
            Self::Balanced(client) => client.call_server_stream(request),
        }
    }

    fn call_client_stream(
        &self,
        req: CallStreamRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
        match self {
            Self::Direct(client) => client.call_client_stream(req),
            Self::Balanced(client) => client.call_client_stream(req),
        }
    }

    fn call_bidi_stream(
        &self,
        req: CallStreamRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcMessageStreamResponse>> {
        match self {
            Self::Direct(client) => client.call_bidi_stream(req),
            Self::Balanced(client) => client.call_bidi_stream(req),
        }
    }

    fn health_check(&self, req: HealthCheckRequest) -> BoxFuture<'_, GrpcEgressResult<()>> {
        match self {
            Self::Direct(client) => client.health_check(req),
            Self::Balanced(client) => client.health_check(req),
        }
    }
}

#[cfg(feature = "prost")]
impl grpc_egress_transport::GrpcEgressProstCodec for TonicGrpcEgressClient {}

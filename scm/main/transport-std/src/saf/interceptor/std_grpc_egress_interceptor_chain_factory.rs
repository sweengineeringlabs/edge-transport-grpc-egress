//! `StdGrpcEgressInterceptorChainFactory` -- creates an empty chain of interceptors.

use grpc_egress_transport::{
    BuildGrpcEgressInterceptorChainRequest, GrpcEgressInterceptorChain,
    GrpcEgressInterceptorChainFactory,
};

use crate::std::StdGrpcEgressInterceptorChain;

/// Creates chains of interceptors.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdGrpcEgressInterceptorChainFactory;

impl GrpcEgressInterceptorChainFactory for StdGrpcEgressInterceptorChainFactory {
    fn build(
        &self,
        _req: BuildGrpcEgressInterceptorChainRequest,
    ) -> impl GrpcEgressInterceptorChain + use<> {
        StdGrpcEgressInterceptorChain::default()
    }
}

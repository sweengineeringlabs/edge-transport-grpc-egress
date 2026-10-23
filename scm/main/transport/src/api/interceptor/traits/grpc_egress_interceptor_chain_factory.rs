//! `GrpcEgressInterceptorChainFactory` -- creates an empty chain of interceptors.

use crate::{BuildGrpcEgressInterceptorChainRequest, GrpcEgressInterceptorChain};

/// Creates a [`GrpcEgressInterceptorChain`].
pub trait GrpcEgressInterceptorChainFactory {
    /// An empty chain. Each call returns an independent chain.
    fn build(
        &self,
        req: BuildGrpcEgressInterceptorChainRequest,
    ) -> impl GrpcEgressInterceptorChain + Send + Sync + use<Self>;
}

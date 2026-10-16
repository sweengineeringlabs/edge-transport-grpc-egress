//! `StdBearerInterceptorFactory` -- creates the interceptor that signs a JWT and attaches it.

use grpc_egress_auth_bearer::BearerEgressConfig;
use grpc_egress_transport::{
    BuildGrpcEgressInterceptorRequest, GrpcEgressInterceptor, GrpcEgressInterceptorFactory,
};

use crate::spi::jsonwebtoken::BearerEgressInterceptor;

/// Creates interceptors that add `authorization: Bearer <jwt>` to every request, signed as one
/// [`BearerEgressConfig`] says.
pub struct StdBearerInterceptorFactory {
    config: BearerEgressConfig,
}

impl StdBearerInterceptorFactory {
    /// A factory whose interceptors sign with `config`.
    pub fn new(config: BearerEgressConfig) -> Self {
        Self { config }
    }
}

impl GrpcEgressInterceptorFactory for StdBearerInterceptorFactory {
    fn build(&self, _req: BuildGrpcEgressInterceptorRequest) -> impl GrpcEgressInterceptor + use<> {
        BearerEgressInterceptor::from_config(self.config.clone())
    }
}

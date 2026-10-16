//! W3C Trace Context outbound interceptor — value object declaration.

use grpc_egress_transport::TraceContextSource;

/// W3C Trace Context propagation interceptor for outbound gRPC.
///
/// Construct via [`TraceContextGrpcEgressInterceptor::pass_through`] or
/// [`TraceContextGrpcEgressInterceptor::with_static`].
#[derive(Clone)]
pub struct TraceContextGrpcEgressInterceptor {
    pub(crate) source: TraceContextSource,
}

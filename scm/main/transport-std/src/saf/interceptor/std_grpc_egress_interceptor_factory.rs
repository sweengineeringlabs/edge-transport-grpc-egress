//! `StdGrpcEgressInterceptorFactory` -- creates the trace-context interceptor.

use grpc_egress_transport::{
    BuildGrpcEgressInterceptorRequest, GrpcEgressInterceptor, GrpcEgressInterceptorFactory,
    TraceContextSource,
};

use crate::std::TraceContextGrpcEgressInterceptor;

/// Creates trace-context interceptors for one [`TraceContextSource`].
pub struct StdGrpcEgressInterceptorFactory {
    source: TraceContextSource,
}

impl StdGrpcEgressInterceptorFactory {
    /// A factory whose interceptors take the trace context from `source`.
    pub fn new(source: TraceContextSource) -> Self {
        Self { source }
    }
}

impl GrpcEgressInterceptorFactory for StdGrpcEgressInterceptorFactory {
    fn build(&self, _req: BuildGrpcEgressInterceptorRequest) -> impl GrpcEgressInterceptor + use<> {
        match self.source.clone() {
            TraceContextSource::PassThrough => TraceContextGrpcEgressInterceptor::pass_through(),
            TraceContextSource::Static {
                traceparent,
                tracestate,
            } => TraceContextGrpcEgressInterceptor::with_static(traceparent, tracestate),
        }
    }
}

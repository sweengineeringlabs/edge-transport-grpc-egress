//! W3C Trace Context outbound interceptor — value object declaration.

use grpc_egress_transport::{
    AfterCallRequest, GrpcEgressError, GrpcEgressInterceptor, GrpcRequest, TraceContextSource,
};

/// W3C Trace Context propagation interceptor for outbound gRPC.
///
/// Construct via [`TraceContextGrpcEgressInterceptor::pass_through`] or
/// [`TraceContextGrpcEgressInterceptor::with_static`].
#[derive(Clone)]
pub struct TraceContextGrpcEgressInterceptor {
    pub(crate) source: TraceContextSource,
}

const TRACEPARENT: &str = "traceparent";
const TRACESTATE: &str = "tracestate";

impl TraceContextGrpcEgressInterceptor {
    /// Propagates an upstream `traceparent` only — does not inject.
    pub fn pass_through() -> Self {
        Self {
            source: TraceContextSource::PassThrough,
        }
    }

    /// Inject a fixed `traceparent` when none is already present.
    pub fn with_static(traceparent: impl Into<String>, tracestate: Option<String>) -> Self {
        Self {
            source: TraceContextSource::Static {
                traceparent: traceparent.into(),
                tracestate,
            },
        }
    }
}

impl GrpcEgressInterceptor for TraceContextGrpcEgressInterceptor {
    fn before_call(&self, req: &mut GrpcRequest) -> Result<(), GrpcEgressError> {
        if req.metadata.contains_key(TRACEPARENT) {
            return Ok(());
        }
        match &self.source {
            TraceContextSource::PassThrough => Ok(()),
            TraceContextSource::Static {
                traceparent,
                tracestate,
            } => {
                req.metadata.insert(TRACEPARENT.into(), traceparent.clone());
                if let Some(state) = tracestate {
                    req.metadata.insert(TRACESTATE.into(), state.clone());
                }
                Ok(())
            }
        }
    }

    fn after_call(&self, _req: AfterCallRequest<'_>) -> Result<(), GrpcEgressError> {
        Ok(())
    }
}

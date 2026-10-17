//! W3C Trace Context outbound interceptor.

use grpc_egress_transport::{
    GrpcEgressInterceptor, InboundReply, OutboundCall, TraceContextSource,
};
use proto_pattern::CallError;

use crate::std::OutboundCallHeaders;

/// Injects the trace context of its source into every call that does not carry one.
#[derive(Clone)]
pub struct TraceContextGrpcEgressInterceptor {
    pub(crate) source: TraceContextSource,
}

const TRACEPARENT: &str = "traceparent";
const TRACESTATE: &str = "tracestate";

impl TraceContextGrpcEgressInterceptor {
    /// An interceptor that adds nothing: a `traceparent` the call carries passes through.
    pub fn pass_through() -> Self {
        Self {
            source: TraceContextSource::PassThrough,
        }
    }

    /// An interceptor that adds the `traceparent` (and the `tracestate`, when given) to a call that carries none.
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
    fn before_call(&self, call: &mut OutboundCall<'_>) -> Result<(), CallError> {
        if call.has_header(TRACEPARENT) {
            return Ok(());
        }
        match &self.source {
            TraceContextSource::PassThrough => Ok(()),
            TraceContextSource::Static {
                traceparent,
                tracestate,
            } => {
                call.add_header(TRACEPARENT, traceparent)?;
                if let Some(state) = tracestate {
                    call.add_header(TRACESTATE, state)?;
                }
                Ok(())
            }
        }
    }

    fn after_call(&self, _reply: &InboundReply<'_>) -> Result<(), CallError> {
        Ok(())
    }
}

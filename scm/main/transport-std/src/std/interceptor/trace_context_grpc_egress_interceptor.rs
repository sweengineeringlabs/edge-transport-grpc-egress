//! W3C Trace Context outbound interceptor.

use grpc_egress_transport_api::proto_pattern::CallError;
use grpc_egress_transport_api::{
    GrpcEgressInterceptor, InboundReply, OutboundCall, TraceContextSource,
};

/// Injects the trace context of its source into every call that does not carry one.
#[derive(Clone)]
pub(crate) struct TraceContextGrpcEgressInterceptor {
    source: TraceContextSource,
}

const TRACEPARENT: &str = "traceparent";
const TRACESTATE: &str = "tracestate";

impl From<TraceContextSource> for TraceContextGrpcEgressInterceptor {
    /// An interceptor that takes the trace context from `source`: `PassThrough` adds nothing (a `traceparent` the
    /// call carries passes through), `Static` adds the `traceparent` (and the `tracestate`, when given) to a call that
    /// carries none.
    fn from(source: TraceContextSource) -> Self {
        Self { source }
    }
}

impl GrpcEgressInterceptor for TraceContextGrpcEgressInterceptor {
    fn before_call(&self, call: &mut dyn OutboundCall) -> Result<(), CallError> {
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

    fn on_event(
        &self,
        _event: &grpc_egress_transport_api::proto_pattern::CallEvent<
            '_,
            grpc_egress_transport_api::proto_pattern::Metadata,
        >,
    ) {
    }
}

//! `impl` blocks for [`TraceContextGrpcEgressInterceptor`]. The type *declaration* lives in `api/`.

use crate::api::{
    AfterCallRequest, GrpcEgressError, GrpcEgressInterceptor, TraceContextGrpcEgressInterceptor,
};
use crate::api::{GrpcRequest, TraceContextSource};

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

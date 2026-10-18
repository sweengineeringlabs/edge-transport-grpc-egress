//! `StdGrpcEgressInterceptorChain` -- the standard [`GrpcEgressInterceptorChain`].

use std::sync::Arc;

use grpc_egress_transport::proto_pattern::CallError;
use grpc_egress_transport::{
    GrpcEgressInterceptor, GrpcEgressInterceptorChain, InboundReply, OutboundCall,
};

/// A registered chain of [`GrpcEgressInterceptor`]s.
///
/// Chain order = the order in which interceptors were added.
#[derive(Clone, Default)]
pub(crate) struct StdGrpcEgressInterceptorChain {
    interceptors: Vec<Arc<dyn GrpcEgressInterceptor>>,
}

impl GrpcEgressInterceptorChain for StdGrpcEgressInterceptorChain {
    fn push(mut self, interceptor: Arc<dyn GrpcEgressInterceptor>) -> Self {
        self.interceptors.push(interceptor);
        self
    }

    fn len(&self) -> usize {
        self.interceptors.len()
    }

    fn is_empty(&self) -> bool {
        self.interceptors.is_empty()
    }

    fn run_before(&self, call: &mut dyn OutboundCall) -> Result<(), CallError> {
        for interceptor in &self.interceptors {
            interceptor.before_call(call)?;
        }
        Ok(())
    }

    fn run_after(&self, reply: &InboundReply<'_>) -> Result<(), CallError> {
        for interceptor in &self.interceptors {
            interceptor.after_call(reply)?;
        }
        Ok(())
    }
}

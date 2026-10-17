//! `impl` block for [`GrpcEgressInterceptorChain`], declared in `grpc_egress_interceptor_chain.rs`
//! beside this file.

use crate::std::GrpcEgressInterceptorChain;
use std::sync::Arc;

use grpc_egress_transport::{GrpcEgressInterceptor, InboundReply, OutboundCall};
use proto_pattern::CallError;

impl GrpcEgressInterceptorChain {
    /// Construct an empty chain.
    pub fn new() -> Self {
        Self {
            interceptors: Vec::new(),
        }
    }

    /// Register `interceptor` at the end of the chain.
    pub fn push(mut self, interceptor: Arc<dyn GrpcEgressInterceptor>) -> Self {
        self.interceptors.push(interceptor);
        self
    }

    /// Number of registered interceptors.
    pub fn len(&self) -> usize {
        self.interceptors.len()
    }

    /// `true` when no interceptors are registered.
    pub fn is_empty(&self) -> bool {
        self.interceptors.is_empty()
    }

    /// Run every `before_call` in order until one fails or all succeed.
    ///
    /// # Errors
    ///
    /// The error of the first interceptor that fails; the ones after it do not run.
    pub fn run_before(&self, call: &mut OutboundCall<'_>) -> Result<(), CallError> {
        for interceptor in &self.interceptors {
            interceptor.before_call(call)?;
        }
        Ok(())
    }

    /// Run every `after_call` in order until one fails or all succeed.
    ///
    /// # Errors
    ///
    /// The error of the first interceptor that fails; the ones after it do not run.
    pub fn run_after(&self, reply: &InboundReply<'_>) -> Result<(), CallError> {
        for interceptor in &self.interceptors {
            interceptor.after_call(reply)?;
        }
        Ok(())
    }
}

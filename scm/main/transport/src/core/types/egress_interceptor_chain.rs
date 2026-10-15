//! `impl` block for [`GrpcEgressInterceptorChain`]. The type *declaration* lives in `api/`.
//!
//! Renamed from `grpc_egress_interceptor_chain.rs` (dropping the redundant
//! `grpc_` prefix) — see `core/channel_config.rs`'s doc comment for why.

use std::sync::Arc;

use crate::api::{
    AfterCallRequest, GrpcEgressError, GrpcEgressInterceptor, GrpcEgressInterceptorChain,
};
use crate::api::{GrpcRequest, GrpcResponse};

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
    pub fn run_before(&self, req: &mut GrpcRequest) -> Result<(), GrpcEgressError> {
        for interceptor in &self.interceptors {
            interceptor.before_call(req)?;
        }
        Ok(())
    }

    /// Run every `after_call` in order until one fails or all succeed.
    pub fn run_after(&self, resp: &mut GrpcResponse) -> Result<(), GrpcEgressError> {
        for interceptor in &self.interceptors {
            interceptor.after_call(AfterCallRequest { response: resp })?;
        }
        Ok(())
    }
}

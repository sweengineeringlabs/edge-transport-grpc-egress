//! `GrpcEgressInterceptorChain` — ordered chain of outbound interceptors.

use std::sync::Arc;

use crate::api::GrpcEgressInterceptor;

/// A registered chain of [`GrpcEgressInterceptor`]s.
///
/// Chain order = the order in which interceptors were added.
#[derive(Clone, Default)]
pub struct GrpcEgressInterceptorChain {
    pub(crate) interceptors: Vec<Arc<dyn GrpcEgressInterceptor>>,
}

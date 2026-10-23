//! `GrpcEgressInterceptorFactory` -- creates the trace-context interceptor of a client.

use crate::{BuildGrpcEgressInterceptorRequest, GrpcEgressInterceptor};

/// Creates a [`GrpcEgressInterceptor`] that injects the W3C trace context.
///
/// A factory only constructs: the source of the context is bound in the implementor's constructor,
/// and `build` returns the interceptor, never a trait object.
pub trait GrpcEgressInterceptorFactory {
    /// Build the interceptor.
    fn build(
        &self,
        req: BuildGrpcEgressInterceptorRequest,
    ) -> impl GrpcEgressInterceptor + use<Self>;
}

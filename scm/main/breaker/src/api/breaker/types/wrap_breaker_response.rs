//! Response for [`crate::api::BreakerDecorator::wrap`].

use std::marker::PhantomData;
use std::sync::Arc;

use edge_transport_grpc_egress::GrpcEgress;

/// Output of [`crate::api::BreakerDecorator::wrap`] — the
/// constructed circuit-breaker decorator, type-erased to its `GrpcEgress`
/// capability so api/ never names the concrete `GrpcBreakerClient` struct.
pub struct WrapBreakerResponse<T> {
    /// The wrapped client, decorated with circuit-breaker policy.
    pub client: Arc<dyn GrpcEgress>,
    /// Marker for the inner client type `T` this response was built from.
    pub _inner: PhantomData<T>,
}

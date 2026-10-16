//! Public type — the breaker decorator.
//!
//! Per SEA rule 160 the type *declaration* is here; the `GrpcEgress` impl
//! block lives in `core::breaker_egress`; the `Debug`, constructor, and
//! accessor impls live in `core::grpc_breaker_client`.

use std::sync::Arc;

use tokio::sync::Mutex;

use edge_transport_breaker_policy::BreakerNode;
use grpc_egress_breaker::GrpcBreakerConfig;

/// Decorator that wraps an inner [`GrpcEgress`](grpc_egress_transport::GrpcEgress) with a
/// three-state circuit breaker.
///
/// Construct via [`create_breaker_client`](crate::GrpcBreakerFacade::create_breaker_client)
/// (loads SWE baseline) or [`GrpcBreakerClient::new`] (caller-supplied config).
pub struct GrpcBreakerClient<T> {
    pub(crate) inner: T,
    pub(crate) config: Arc<GrpcBreakerConfig>,
    pub(crate) node: Arc<Mutex<BreakerNode>>,
}

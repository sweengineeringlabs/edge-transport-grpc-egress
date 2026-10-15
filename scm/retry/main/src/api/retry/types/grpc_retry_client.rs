//! Public type — the retry decorator that wraps any [`GrpcEgress`].
//!
//! Per SEA rule 160, the type *declaration* lives in api/.
//! The `GrpcEgress` impl block lives in `core/`.

use std::sync::Arc;

use crate::api::retry::types::grpc_retry_config::GrpcRetryConfig;

/// Decorator that wraps an inner [`GrpcEgress`](edge_transport_grpc_egress::GrpcEgress) with the
/// retry semantics described at the crate root.
///
/// `T` is the inner type; the wrapper is `T + 'static + Send + Sync`
/// so it can flow across `.await` boundaries inside the runtime.
///
/// Construct via [`GrpcRetryFacade`](crate::api::GrpcRetryFacade) (loads
/// the SWE baseline) or directly via [`GrpcRetryClient::new`].
pub struct GrpcRetryClient<T> {
    pub(crate) inner: T,
    pub(crate) config: Arc<GrpcRetryConfig>,
}

//! `GrpcBreakerClient` -- the breaker decorator.

use std::sync::Arc;

use edge_transport_breaker_policy::BreakerNode;
use futures::future::BoxFuture;
use grpc_egress_breaker_api::{
    BreakerDomainError, BreakerObservable, BreakerState, GrpcBreakerConfig, ObserveStateRequest,
    ObserveStateResponse, GRPC_BREAKER_CLIENT_LOG_TARGET,
};
use tokio::sync::Mutex;
use tracing::trace;

/// Decorator that wraps an inner client (a `Transport` and a `RawTransport`) with a three-state circuit breaker.
///
/// Built by `StdGrpcBreakerFactory`, which validates the policy first.
pub(crate) struct GrpcBreakerClient<T> {
    pub(crate) inner: T,
    pub(crate) config: Arc<GrpcBreakerConfig>,
    pub(crate) node: Arc<Mutex<BreakerNode>>,
}

impl<T> std::fmt::Debug for GrpcBreakerClient<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GrpcBreakerClient")
            .field("failure_threshold", &self.config.failure_threshold())
            .field("enabled", &self.config.enabled())
            .field("cool_down_seconds", &self.config.cool_down_seconds())
            .field(
                "half_open_probe_count",
                &self.config.half_open_probe_count(),
            )
            .finish()
    }
}

impl<T> From<(T, GrpcBreakerConfig)> for GrpcBreakerClient<T> {
    /// A breaker decorator around the client, guarded by the policy.
    fn from((inner, config): (T, GrpcBreakerConfig)) -> Self {
        Self {
            inner,
            config: Arc::new(config),
            node: Arc::new(Mutex::new(BreakerNode {
                state: BreakerState::Closed,
                consecutive_failures: 0,
                consecutive_successes: 0,
            })),
        }
    }
}

impl<T: Send + Sync> BreakerObservable for GrpcBreakerClient<T> {
    fn state(
        &self,
        _req: ObserveStateRequest,
    ) -> BoxFuture<'_, Result<ObserveStateResponse, BreakerDomainError>> {
        Box::pin(async move {
            let state = self.node.lock().await.state;
            trace!(target: GRPC_BREAKER_CLIENT_LOG_TARGET, ?state, "grpc-breaker: state observed");
            Ok(ObserveStateResponse::new(state))
        })
    }
}

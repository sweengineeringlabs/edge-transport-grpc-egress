//! `Debug`, constructor, accessors, and [`BreakerObservable`] for [`GrpcBreakerClient`].

use std::sync::Arc;

use futures::future::BoxFuture;
use tokio::sync::Mutex;
use tracing::trace;

use crate::api::{
    BreakerDomainError, BreakerNode, BreakerObservable, BreakerState, GrpcBreakerClient,
    GrpcBreakerConfig, ObserveStateRequest, ObserveStateResponse, GRPC_BREAKER_CLIENT_LOG_TARGET,
};

impl<T> std::fmt::Debug for GrpcBreakerClient<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GrpcBreakerClient")
            .field("failure_threshold", &self.config.failure_threshold)
            .field("cool_down_seconds", &self.config.cool_down_seconds)
            .field("half_open_probe_count", &self.config.half_open_probe_count)
            .finish()
    }
}

impl<T> GrpcBreakerClient<T> {
    /// Construct a new breaker decorator around `inner`.
    pub fn new(inner: T, config: GrpcBreakerConfig) -> Self {
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

    /// Borrow the active breaker policy.
    pub fn config(&self) -> &GrpcBreakerConfig {
        &self.config
    }

    /// Observe the current breaker state.  Returns a snapshot;
    /// the breaker may transition immediately after this call.
    pub async fn state(&self) -> BreakerState {
        self.node.lock().await.state
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
            Ok(ObserveStateResponse { state })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: new
    /// @covers: state
    #[test]
    fn test_new_starts_closed() {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        rt.block_on(async {
            let client = GrpcBreakerClient::new((), GrpcBreakerConfig::default());
            assert!(matches!(client.state().await, BreakerState::Closed));
        });
    }

    /// @covers: config
    #[test]
    fn test_config_returns_the_supplied_policy() {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        rt.block_on(async {
            let cfg = GrpcBreakerConfig {
                failure_threshold: 7,
                cool_down_seconds: 20,
                half_open_probe_count: 2,
            };
            let client = GrpcBreakerClient::new((), cfg);
            assert_eq!(client.config().failure_threshold, 7);
        });
    }

    /// @covers: state
    #[test]
    fn test_breaker_observable_state_matches_inherent_state() {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        rt.block_on(async {
            let client = GrpcBreakerClient::new((), GrpcBreakerConfig::default());
            let resp = BreakerObservable::state(&client, ObserveStateRequest)
                .await
                .expect("observe is infallible");
            assert!(matches!(resp.state, BreakerState::Closed));
        });
    }

    #[test]
    fn test_debug_includes_policy_fields() {
        let client = GrpcBreakerClient::new((), GrpcBreakerConfig::default());
        let s = format!("{client:?}");
        assert!(s.contains("failure_threshold"));
    }
}

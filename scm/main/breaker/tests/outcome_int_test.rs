#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Coverage stub for `src/api/breaker/outcome.rs`.
//!
//! `Outcome` is `pub(crate)` — not part of the public API.
//! The outcome drives the `record` side of the transition logic.
//! This stub verifies the public surface that depends on it compiles.

use std::collections::HashMap;
use std::time::Duration;

use edge_transport_grpc_egress::{
    GrpcEgress, GrpcEgressResult, GrpcRequest, GrpcResponse, HealthCheckRequest,
};
use edge_transport_grpc_egress_breaker::{BreakerState, GrpcBreakerClient, GrpcBreakerConfig};
use futures::future::BoxFuture;

/// Always-succeeding inner client so a real `Outcome::Success` is recorded.
struct AlwaysOk;
impl GrpcEgress for AlwaysOk {
    fn call_unary(&self, _r: GrpcRequest) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
        Box::pin(async {
            Ok(GrpcResponse {
                body: vec![],
                metadata: HashMap::new(),
            })
        })
    }
    fn health_check(&self, _req: HealthCheckRequest) -> BoxFuture<'_, GrpcEgressResult<()>> {
        Box::pin(async { Ok(()) })
    }
}

/// @covers: Outcome (internal) — BreakerState reflects recorded outcomes
#[tokio::test]
async fn breaker_enum_outcome_state_reflects_success_outcome_int_test() {
    // Outcome::Success keeps the breaker Closed. The public observable
    // is BreakerState; drive one real successful call through the
    // transition logic and confirm it stays Closed.
    let client = GrpcBreakerClient::new(AlwaysOk, GrpcBreakerConfig::default());
    let request = GrpcRequest::new("svc.Test/Method", vec![], Duration::from_secs(5));
    client.call_unary(request).await.expect("always succeeds");
    assert_eq!(client.state().await, BreakerState::Closed);
}

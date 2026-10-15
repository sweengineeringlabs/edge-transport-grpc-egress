#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for the `GrpcEgress` impl on [`GrpcRetryClient`]
//! in `src/core/retry_egress.rs`.

use std::collections::HashMap;

use edge_transport_grpc_egress::{
    CallStreamRequest, GrpcEgress, GrpcEgressError, GrpcEgressResult, GrpcRequest, GrpcResponse,
    HealthCheckRequest,
};
use edge_transport_grpc_egress_retry::{GrpcRetryClient, GrpcRetryConfig};
use futures::future::BoxFuture;

struct AlwaysFail;
impl GrpcEgress for AlwaysFail {
    fn call_unary(&self, _req: GrpcRequest) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
        Box::pin(async { Err(GrpcEgressError::Unavailable("down".into())) })
    }
    fn call_stream(
        &self,
        req: CallStreamRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<edge_transport_grpc_egress::GrpcMessageStreamResponse>>
    {
        Box::pin(async move { Ok(req.messages) })
    }
    fn health_check(&self, _req: HealthCheckRequest) -> BoxFuture<'_, GrpcEgressResult<()>> {
        Box::pin(async { Ok(()) })
    }
}

struct UnhealthyInner;
impl GrpcEgress for UnhealthyInner {
    fn call_unary(&self, _req: GrpcRequest) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
        Box::pin(async { Err(GrpcEgressError::Unavailable("down".into())) })
    }
    fn call_stream(
        &self,
        req: CallStreamRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<edge_transport_grpc_egress::GrpcMessageStreamResponse>>
    {
        Box::pin(async move { Ok(req.messages) })
    }
    fn health_check(&self, _req: HealthCheckRequest) -> BoxFuture<'_, GrpcEgressResult<()>> {
        Box::pin(async { Err(GrpcEgressError::Unavailable("unhealthy".into())) })
    }
}

fn no_retry_config() -> GrpcRetryConfig {
    GrpcRetryConfig::from_config(
        r#"
            max_attempts = 1
            initial_backoff_ms = 1
            backoff_multiplier = 1.0
            jitter_factor = 0.0
            max_backoff_ms = 1
            rate_limit_max_attempts = 1
            rate_limit_initial_backoff_ms = 1
            rate_limit_max_backoff_ms = 1
        "#,
    )
    .expect("valid config")
}

/// @covers: call_unary
#[tokio::test]
async fn test_call_unary_exhausts_single_attempt_and_reports_unavailable_happy() {
    let client = GrpcRetryClient::new(AlwaysFail, no_retry_config());
    let req = GrpcRequest::new("svc/M", vec![], std::time::Duration::from_secs(5));
    let result = client.call_unary(req).await;
    assert!(matches!(result, Err(GrpcEgressError::Unavailable(_))));
}

/// @covers: health_check
#[tokio::test]
async fn test_health_check_delegates_to_inner_error() {
    let healthy = GrpcRetryClient::new(AlwaysFail, no_retry_config());
    assert!(matches!(
        healthy.health_check(HealthCheckRequest).await,
        Ok(())
    ));
    // Negative counterpart: an unhealthy inner must surface as unhealthy —
    // proves this genuinely delegates rather than always returning Ok.
    let unhealthy = GrpcRetryClient::new(UnhealthyInner, no_retry_config());
    assert!(matches!(
        unhealthy.health_check(HealthCheckRequest).await,
        Err(GrpcEgressError::Unavailable(_))
    ));
}

/// @covers: call_stream
#[tokio::test]
async fn test_call_stream_delegates_to_inner_edge() {
    let client = GrpcRetryClient::new(AlwaysFail, no_retry_config());
    let messages = edge_transport_grpc_egress::GrpcMessageStreamResponse {
        stream: Box::pin(futures::stream::empty::<GrpcEgressResult<Vec<u8>>>()),
    };
    let result = client
        .call_stream(CallStreamRequest {
            method: "svc/M".into(),
            metadata: HashMap::new(),
            messages,
        })
        .await;
    assert!(result.is_ok(), "AlwaysFail's call_stream always succeeds");
}

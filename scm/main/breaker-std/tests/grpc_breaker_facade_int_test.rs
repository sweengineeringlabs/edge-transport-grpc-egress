#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`GrpcBreakerFacade`].

use grpc_egress_breaker_std::ApplicationConfigBuilderProvider;
use grpc_egress_transport_std::GrpcRequestFactory;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use futures::future::BoxFuture;
use grpc_egress_breaker::GrpcBreakerConfig;
use grpc_egress_breaker_std::GrpcBreakerFacade;
use grpc_egress_transport::{
    CallStreamRequest, GrpcEgress, GrpcEgressError, GrpcEgressResult, GrpcMessageStreamResponse,
    GrpcRequest, GrpcResponse, HealthCheckRequest,
};

struct CountingEgress(Arc<AtomicU32>);
impl GrpcEgress for CountingEgress {
    fn call_unary(&self, _req: GrpcRequest) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Err(GrpcEgressError::Unavailable("down".into())) })
    }
    fn call_stream(
        &self,
        req: CallStreamRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcMessageStreamResponse>> {
        Box::pin(async move { Ok(req.messages) })
    }
    fn health_check(&self, _req: HealthCheckRequest) -> BoxFuture<'_, GrpcEgressResult<()>> {
        Box::pin(async { Ok(()) })
    }
}

fn make_request() -> GrpcRequest {
    GrpcRequest::new("svc/M", vec![], std::time::Duration::from_secs(1))
}

/// @covers: create_config_builder
#[test]
fn test_create_config_builder_produces_a_working_loader_happy() {
    let loader = GrpcBreakerFacade::create_config_builder()
        .expect("create_config_builder is infallible")
        .build_loader()
        .expect("pre-seeded builder must build a valid loader");
    #[derive(serde::Deserialize, Default, PartialEq, Debug)]
    struct AbsentSectionProbe {
        marker: bool,
    }
    let err = loader
        .load_section::<AbsentSectionProbe>("grpc_breaker_facade_probe_section_absent")
        .expect_err("no config directory exists in the test environment");
    assert!(err
        .to_string()
        .contains("grpc_breaker_facade_probe_section_absent"));
}

/// @covers: wrap_breaker
#[test]
fn test_wrap_breaker_opens_after_threshold_error() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let calls = Arc::new(AtomicU32::new(0));
        let client = GrpcBreakerFacade::wrap_breaker(
            CountingEgress(calls.clone()),
            GrpcBreakerConfig {
                failure_threshold: 2,
                cool_down_seconds: 30,
                half_open_probe_count: 1,
            },
        )
        .expect("wrap_breaker is infallible");
        let _ = client.call_unary(make_request()).await;
        let _ = client.call_unary(make_request()).await;
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        let err = client
            .call_unary(make_request())
            .await
            .expect_err("breaker must reject once open");
        assert!(matches!(err, GrpcEgressError::Unavailable(_)));
        assert_eq!(
            calls.load(Ordering::SeqCst),
            2,
            "open state must not call inner"
        );
    });
}

/// @covers: create_breaker_client
#[test]
fn test_create_breaker_client_uses_default_policy_edge() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let calls = Arc::new(AtomicU32::new(0));
        let client = GrpcBreakerFacade::create_breaker_client(CountingEgress(calls.clone()))
            .expect("create_breaker_client is infallible");
        let default_cfg = GrpcBreakerConfig::default();
        for _ in 0..default_cfg.failure_threshold {
            let _ = client.call_unary(make_request()).await;
        }
        assert_eq!(calls.load(Ordering::SeqCst), default_cfg.failure_threshold);
        let err = client
            .call_unary(make_request())
            .await
            .expect_err("breaker must reject once the default threshold is reached");
        assert!(matches!(err, GrpcEgressError::Unavailable(_)));
    });
}

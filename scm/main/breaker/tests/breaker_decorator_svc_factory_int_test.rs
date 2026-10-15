#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`BreakerDecoratorFactory`].

use std::collections::HashMap;

use edge_transport_grpc_egress::{
    CallStreamRequest, GrpcEgress, GrpcEgressResult, GrpcMessageStreamResponse, GrpcRequest,
    GrpcResponse, HealthCheckRequest,
};
use edge_transport_grpc_egress_breaker::{
    BreakerDecoratorFactory, GrpcBreakerConfig, WrapBreakerRequest,
};
use futures::future::BoxFuture;

struct EchoGrpcEgress;
impl GrpcEgress for EchoGrpcEgress {
    fn call_unary(&self, _req: GrpcRequest) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
        Box::pin(async {
            Ok(GrpcResponse {
                body: b"factory-stub".to_vec(),
                metadata: HashMap::new(),
            })
        })
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

/// @covers: create
#[test]
fn test_create_produces_a_working_decorator_happy() {
    let decorator = BreakerDecoratorFactory::create::<EchoGrpcEgress>();
    let resp = decorator
        .wrap(WrapBreakerRequest {
            inner: EchoGrpcEgress,
            config: GrpcBreakerConfig::default(),
        })
        .expect("factory-produced decorator must wrap successfully");
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let result = rt.block_on(resp.client.call_unary(GrpcRequest::new(
        "svc/M",
        vec![],
        std::time::Duration::from_secs(1),
    )));
    assert_eq!(result.expect("call must succeed").body, b"factory-stub");
}

/// @covers: create
#[test]
fn test_create_zero_threshold_config_error() {
    let decorator = BreakerDecoratorFactory::create::<EchoGrpcEgress>();
    let resp = decorator
        .wrap(WrapBreakerRequest {
            inner: EchoGrpcEgress,
            config: GrpcBreakerConfig {
                failure_threshold: 0,
                cool_down_seconds: 0,
                half_open_probe_count: 0,
            },
        })
        .expect("wrap itself doesn't validate — that's Validator's job");
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let result = rt.block_on(resp.client.call_unary(GrpcRequest::new(
        "svc/M",
        vec![],
        std::time::Duration::from_secs(1),
    )));
    assert_eq!(result.expect("call must succeed").body, b"factory-stub");
}

/// @covers: create
#[test]
fn test_create_produces_independent_instances_edge() {
    let first = BreakerDecoratorFactory::create::<EchoGrpcEgress>();
    let second = BreakerDecoratorFactory::create::<EchoGrpcEgress>();
    let resp1 = first
        .wrap(WrapBreakerRequest {
            inner: EchoGrpcEgress,
            config: GrpcBreakerConfig::default(),
        })
        .expect("first must wrap");
    let resp2 = second
        .wrap(WrapBreakerRequest {
            inner: EchoGrpcEgress,
            config: GrpcBreakerConfig::default(),
        })
        .expect("second must wrap");
    assert!(!std::ptr::eq(
        std::sync::Arc::as_ptr(&resp1.client) as *const (),
        std::sync::Arc::as_ptr(&resp2.client) as *const ()
    ));
}

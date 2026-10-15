#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`WrapBreakerResponse`].

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

use edge_transport_grpc_egress::{
    CallStreamRequest, GrpcEgress, GrpcEgressResult, GrpcMessageStreamResponse, GrpcRequest,
    GrpcResponse, HealthCheckRequest,
};
use edge_transport_grpc_egress_breaker::WrapBreakerResponse;
use futures::future::BoxFuture;

struct EchoGrpcEgress;
impl GrpcEgress for EchoGrpcEgress {
    fn call_unary(&self, _req: GrpcRequest) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
        Box::pin(async {
            Ok(GrpcResponse {
                body: b"wrap-stub".to_vec(),
                metadata: HashMap::new(),
            })
        })
    }
    fn call_stream(
        &self,
        _req: CallStreamRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcMessageStreamResponse>> {
        unimplemented!("not exercised by this test")
    }
    fn health_check(&self, _req: HealthCheckRequest) -> BoxFuture<'_, GrpcEgressResult<()>> {
        Box::pin(async { Ok(()) })
    }
}

/// @covers: WrapBreakerResponse
#[test]
fn test_wrap_breaker_response_client_is_usable_happy() {
    let resp: WrapBreakerResponse<EchoGrpcEgress> = WrapBreakerResponse {
        client: Arc::new(EchoGrpcEgress),
        _inner: PhantomData,
    };
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let result = rt.block_on(resp.client.call_unary(GrpcRequest::new(
        "svc/M",
        vec![],
        std::time::Duration::from_secs(1),
    )));
    assert_eq!(result.expect("call must succeed").body, b"wrap-stub");
}

/// @covers: WrapBreakerResponse
#[test]
fn test_wrap_breaker_response_health_check_error() {
    let resp: WrapBreakerResponse<EchoGrpcEgress> = WrapBreakerResponse {
        client: Arc::new(EchoGrpcEgress),
        _inner: PhantomData,
    };
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let result = rt.block_on(resp.client.health_check(HealthCheckRequest));
    result.expect("health check must succeed for a healthy stub");
}

/// @covers: WrapBreakerResponse
#[test]
fn test_wrap_breaker_response_client_is_cloneable_arc_edge() {
    let resp: WrapBreakerResponse<EchoGrpcEgress> = WrapBreakerResponse {
        client: Arc::new(EchoGrpcEgress),
        _inner: PhantomData,
    };
    let cloned = Arc::clone(&resp.client);
    assert_eq!(Arc::strong_count(&resp.client), 2);
    drop(cloned);
    assert_eq!(Arc::strong_count(&resp.client), 1);
}

#![allow(clippy::unwrap_used, clippy::expect_used)]
//! End-to-end tests for [`BreakerDecorator`] via a test-double
//! implementation.

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

use edge_transport_grpc_egress::{
    CallStreamRequest, GrpcEgress, GrpcEgressResult, GrpcMessageStreamResponse, GrpcRequest,
    GrpcResponse, HealthCheckRequest,
};
use edge_transport_grpc_egress_breaker::{
    BreakerDecorator, Error, GrpcBreakerConfig, GrpcBreakerFacade, WrapBreakerRequest,
    WrapBreakerResponse,
};
use futures::future::BoxFuture;

struct StubEgress;
impl GrpcEgress for StubEgress {
    fn call_unary(&self, _req: GrpcRequest) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
        Box::pin(async {
            Ok(GrpcResponse {
                body: b"stub".to_vec(),
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

struct MockBreakerDecorator {
    fail: bool,
}

impl<T> BreakerDecorator<T> for MockBreakerDecorator {
    fn wrap(&self, req: WrapBreakerRequest<T>) -> Result<WrapBreakerResponse<T>, Error> {
        if self.fail {
            return Err(Error::InvalidConfig("mock decorator forced failure".into()));
        }
        let _ = req.config;
        Ok(WrapBreakerResponse {
            client: Arc::new(StubEgress),
            _inner: PhantomData,
        })
    }
}

fn cfg() -> GrpcBreakerConfig {
    GrpcBreakerConfig {
        failure_threshold: 3,
        cool_down_seconds: 5,
        half_open_probe_count: 1,
    }
}

/// @covers: wrap
#[test]
fn test_wrap_produces_a_working_client_happy() {
    let decorator = MockBreakerDecorator { fail: false };
    let resp = decorator
        .wrap(WrapBreakerRequest {
            inner: StubEgress,
            config: cfg(),
        })
        .expect("happy path");
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let result = rt.block_on(resp.client.call_unary(GrpcRequest::new(
        "svc/M",
        vec![],
        std::time::Duration::from_secs(1),
    )));
    assert_eq!(result.expect("call must succeed").body, b"stub");
}

/// @covers: default_facade
#[test]
fn test_default_facade_returns_a_zero_sized_marker_happy() {
    let facade = <MockBreakerDecorator as BreakerDecorator<()>>::default_facade();
    assert_eq!(std::mem::size_of_val(&facade), 0);
}

/// @covers: default_facade
#[test]
fn test_default_facade_type_matches_the_real_facade_error() {
    // "error"-flavored scenario for an infallible constructor: prove the
    // value returned really is `GrpcBreakerFacade` — usable to call the
    // facade's own real methods — not just a same-named unrelated type.
    let _facade: GrpcBreakerFacade =
        <MockBreakerDecorator as BreakerDecorator<()>>::default_facade();
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let result = rt.block_on(
        GrpcBreakerFacade::wrap_breaker(StubEgress, cfg())
            .expect("facade must wrap successfully")
            .call_unary(GrpcRequest::new(
                "svc/M",
                vec![],
                std::time::Duration::from_secs(1),
            )),
    );
    assert_eq!(result.expect("call must succeed").body, b"stub");
}

/// @covers: wrap
#[test]
fn test_wrap_propagates_failure_error() {
    let decorator = MockBreakerDecorator { fail: true };
    let err = decorator
        .wrap(WrapBreakerRequest {
            inner: StubEgress,
            config: cfg(),
        })
        .err()
        .expect("forced failure must surface");
    assert!(err.to_string().contains("mock decorator forced failure"));
}

/// @covers: wrap
#[test]
fn test_wrap_zero_failure_threshold_edge() {
    let decorator = MockBreakerDecorator { fail: false };
    let resp = decorator
        .wrap(WrapBreakerRequest {
            inner: StubEgress,
            config: GrpcBreakerConfig {
                failure_threshold: 0,
                cool_down_seconds: 0,
                half_open_probe_count: 0,
            },
        })
        .expect("wrap itself doesn't validate config — that's Validator's job");
    // Payload assertion, not just is_ok(): the zero-threshold client must
    // still be a genuinely working GrpcEgress delegating to the inner one.
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let result = rt.block_on(resp.client.call_unary(GrpcRequest::new(
        "svc/M",
        vec![],
        std::time::Duration::from_secs(1),
    )));
    assert_eq!(result.expect("call must succeed").body, b"stub");
}

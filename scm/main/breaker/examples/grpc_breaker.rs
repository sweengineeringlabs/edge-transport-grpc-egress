//! Reusable example: drive real calls through [`GrpcBreakerClient`] so the
//! circuit-breaker state machine (`DefaultBreakerTransition`, moved to the
//! shared `edge-transport-breaker` crate) actually executes — not just
//! constructs.
//!
//! [`GrpcBreakerClient`]: edge_transport_grpc_egress_breaker::GrpcBreakerClient
#![allow(
    clippy::expect_used,
    reason = "example code, not production library code"
)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use edge_transport_grpc_egress::{
    CallStreamRequest, GrpcEgress, GrpcEgressError, GrpcEgressResult, GrpcMessageStreamResponse,
    GrpcRequest, GrpcResponse, HealthCheckRequest,
};
use edge_transport_grpc_egress_breaker::{GrpcBreakerConfig, GrpcBreakerFacade};
use futures::future::BoxFuture;

/// An upstream whose success/failure and call count are controllable from
/// outside — lets this example prove the breaker really rejects calls
/// without reaching the upstream once open, not just that it compiles.
struct ControllableUpstream {
    calls: Arc<AtomicUsize>,
    should_fail: Arc<AtomicBool>,
}

impl GrpcEgress for ControllableUpstream {
    fn call_unary(&self, _req: GrpcRequest) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let fail = self.should_fail.load(Ordering::SeqCst);
        Box::pin(async move {
            if fail {
                Err(GrpcEgressError::Unavailable("upstream is down".into()))
            } else {
                Ok(GrpcResponse {
                    body: b"ok".to_vec(),
                    metadata: HashMap::new(),
                })
            }
        })
    }

    fn call_stream(
        &self,
        _req: CallStreamRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcMessageStreamResponse>> {
        unimplemented!("not exercised by this example")
    }

    fn health_check(&self, _req: HealthCheckRequest) -> BoxFuture<'_, GrpcEgressResult<()>> {
        Box::pin(async { Ok(()) })
    }
}

fn request() -> GrpcRequest {
    GrpcRequest::new("example.Service/Method", vec![], Duration::from_secs(1))
}

#[tokio::main]
async fn main() {
    let calls = Arc::new(AtomicUsize::new(0));
    let should_fail = Arc::new(AtomicBool::new(true));

    let upstream = ControllableUpstream {
        calls: calls.clone(),
        should_fail: should_fail.clone(),
    };
    let client = GrpcBreakerFacade::wrap_breaker(
        upstream,
        GrpcBreakerConfig {
            failure_threshold: 2,
            cool_down_seconds: 1,
            half_open_probe_count: 1,
        },
    )
    .expect("wrap_breaker must succeed for a valid config");

    // Two real failures reach the upstream and trip the breaker.
    for attempt in 1..=2 {
        let result = client.call_unary(request()).await;
        println!(
            "attempt {attempt}: {result:?} (upstream calls so far: {})",
            calls.load(Ordering::SeqCst)
        );
        assert!(matches!(result, Err(GrpcEgressError::Unavailable(_))));
    }

    // Third call: the circuit is now open. This is the real proof that
    // `DefaultBreakerTransition::admit` executed and changed state — the
    // call is rejected WITHOUT reaching the upstream (call count unchanged).
    let before = calls.load(Ordering::SeqCst);
    let result = client.call_unary(request()).await;
    let after = calls.load(Ordering::SeqCst);
    println!("attempt 3 (circuit open): {result:?}");
    assert!(matches!(result, Err(GrpcEgressError::Unavailable(_))));
    assert_eq!(
        before, after,
        "an open circuit must reject without reaching the upstream"
    );
    println!("confirmed: circuit rejected the call without invoking the upstream ({after} total upstream calls)");

    // Wait out the cool-down, let the upstream recover, and prove the
    // breaker really re-admits a half-open probe and closes again.
    tokio::time::sleep(Duration::from_secs(1)).await;
    should_fail.store(false, Ordering::SeqCst);
    let result = client.call_unary(request()).await;
    println!("attempt 4 (after cool-down, upstream recovered): {result:?}");
    assert!(
        result.is_ok(),
        "the half-open probe must reach the upstream and succeed"
    );
    println!("confirmed: breaker transitioned Open -> HalfOpen -> Closed for real");
}

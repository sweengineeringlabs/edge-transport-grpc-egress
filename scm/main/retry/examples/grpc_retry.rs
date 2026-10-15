//! Reusable example: drive a real call through [`GrpcRetryClient`] so the
//! backoff/attempt loop (delegating to the shared `edge-transport-retry`
//! crate's `BackoffScheduler`) actually executes — not just constructs.
//!
//! A `GrpcEgress` test double is enough to prove this; no real gRPC server
//! is needed since `GrpcRetryClient` decorates any `GrpcEgress` implementor.
//!
//! [`GrpcRetryClient`]: edge_transport_grpc_egress_retry::GrpcRetryClient

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use edge_transport_grpc_egress::{
    CallStreamRequest, GrpcEgress, GrpcEgressError, GrpcEgressResult, GrpcMessageStreamResponse,
    GrpcRequest, GrpcResponse, HealthCheckRequest,
};
use edge_transport_grpc_egress_retry::{GrpcRetryConfig, GrpcRetryFacade};
use futures::future::BoxFuture;

/// An upstream that fails the first `fail_until` calls, then succeeds —
/// lets this example prove the retry loop really re-invokes the upstream
/// after a real backoff sleep, not just that it compiles.
struct FlakyUpstream {
    calls: Arc<AtomicUsize>,
    fail_until: usize,
}

impl GrpcEgress for FlakyUpstream {
    fn call_unary(&self, _req: GrpcRequest) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
        let attempt = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        let fail_until = self.fail_until;
        Box::pin(async move {
            if attempt <= fail_until {
                Err(GrpcEgressError::Unavailable(format!(
                    "upstream still down (attempt {attempt})"
                )))
            } else {
                Ok(GrpcResponse {
                    body: format!("ok on attempt {attempt}").into_bytes(),
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

#[tokio::main]
async fn main() {
    let calls = Arc::new(AtomicUsize::new(0));
    let upstream = FlakyUpstream {
        calls: calls.clone(),
        fail_until: 2,
    };

    let client = GrpcRetryFacade::wrap_retry(
        upstream,
        GrpcRetryConfig {
            max_attempts: 3,
            initial_backoff_ms: 50,
            backoff_multiplier: 2.0,
            jitter_factor: 0.0,
            max_backoff_ms: 200,
            ..GrpcRetryConfig::default()
        },
    );

    // A single logical call. The retry layer's internal attempt loop must
    // re-invoke the upstream after a real backoff sleep on each retriable
    // failure — the deadline below leaves comfortable headroom for that.
    let request = GrpcRequest::new("example.Service/Method", vec![], Duration::from_secs(3));
    let started = std::time::Instant::now();
    let result = client.call_unary(request).await;
    let elapsed = started.elapsed();

    println!("result: {result:?}");
    println!(
        "upstream was called {} time(s) across {:?}",
        calls.load(Ordering::SeqCst),
        elapsed
    );

    assert!(
        result.is_ok(),
        "the retry layer must eventually succeed once the upstream recovers"
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        3,
        "the upstream must have been invoked on all 3 attempts (2 failures + 1 success)"
    );
    assert!(
        elapsed >= Duration::from_millis(50),
        "elapsed time must reflect at least one real backoff sleep, not an instant retry"
    );
    println!(
        "confirmed: BackoffScheduler::next_backoff really executed between attempts \
         (elapsed {elapsed:?} implies a genuine sleep, not a busy loop)"
    );
}

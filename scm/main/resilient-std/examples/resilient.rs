//! Reusable example: wrap an already-built client in retry and circuit breaker with
//! [`StdGrpcResilientFactory`].
//!
//! The decorator never builds the client: the composition root does that and applies the decorator
//! on top. A `GrpcEgress` test double stands in for the client, so no server is needed. The double
//! fails its first two calls and then succeeds; the retry layer inside the decorator must call it
//! again after a real backoff until it does.
//!
//! [`StdGrpcResilientFactory`]: grpc_egress_resilient_std::StdGrpcResilientFactory

use grpc_egress_transport_std::GrpcRequestFactory;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures::future::BoxFuture;
use grpc_egress_resilient::ResilienceConfig;
use grpc_egress_resilient::{BuildGrpcResilientRequest, GrpcResilientFactory};
use grpc_egress_resilient_std::StdGrpcResilientFactory;
use grpc_egress_transport::{
    GrpcEgress, GrpcEgressError, GrpcEgressResult, GrpcRequest, GrpcResponse, HealthCheckRequest,
};

/// An upstream that fails its first `fail_until` calls with `Unavailable`, then succeeds.
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

    fn health_check(&self, _req: HealthCheckRequest) -> BoxFuture<'_, GrpcEgressResult<()>> {
        Box::pin(async { Ok(()) })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let calls = Arc::new(AtomicUsize::new(0));
    let upstream = FlakyUpstream {
        calls: calls.clone(),
        fail_until: 2,
    };

    let config = ResilienceConfig {
        max_attempts: 3,
        initial_backoff_ms: 20,
        backoff_multiplier: 2.0,
        jitter_factor: 0.0,
        max_backoff_ms: 100,
        rate_limit_max_attempts: 1,
        rate_limit_initial_backoff_ms: 100,
        rate_limit_max_backoff_ms: 100,
        failure_threshold: 5,
        cool_down_seconds: 1,
        half_open_probe_count: 1,
    };
    let client = StdGrpcResilientFactory::new(config)
        .build(BuildGrpcResilientRequest { inner: upstream })?;

    let request = GrpcRequest::new("example.Service/Method", vec![], Duration::from_secs(3));
    let started = std::time::Instant::now();
    let response = client.call_unary(request).await?;
    let elapsed = started.elapsed();

    println!("response: {}", String::from_utf8_lossy(&response.body));
    println!(
        "upstream was called {} time(s) across {:?}",
        calls.load(Ordering::SeqCst),
        elapsed
    );

    assert_eq!(
        calls.load(Ordering::SeqCst),
        3,
        "two failures and one success must reach the upstream"
    );
    assert!(
        elapsed >= Duration::from_millis(20),
        "the retries must be separated by a real backoff"
    );
    Ok(())
}

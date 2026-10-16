//! Integration tests for the gRPC retry decorator.
//!
//! These exercise the public API end-to-end: a stub
//! [`GrpcEgress`] is wrapped with the retry client and the
//! retry loop is observed via call counters and elapsed time.

use grpc_egress_retry_std::GrpcRetryConfigProvider;
use grpc_egress_transport_std::GrpcRequestFactory;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::future::BoxFuture;
use grpc_egress_retry::{BuildGrpcRetryRequest, GrpcRetryConfig, GrpcRetryFactory};
use grpc_egress_retry_std::StdGrpcRetryFactory;
use grpc_egress_transport::{
    GrpcEgress, GrpcEgressError, GrpcEgressResult, GrpcRequest, GrpcResponse, GrpcStatusCode,
    HealthCheckRequest,
};

/// Stub `GrpcEgress` that returns a scripted sequence of
/// outcomes and counts how many times `call_unary` was invoked.
///
/// The wrapping `Arc` lets the test observe call counts after
/// the stub has been moved into the decorator.
struct ScriptedClient {
    calls: AtomicU32,
    /// Each entry is reused on subsequent calls past the script
    /// length; index = (call_count - 1).min(script.len() - 1).
    script: Vec<Outcome>,
}

#[derive(Clone, Debug)]
enum Outcome {
    Ok,
    Status(GrpcStatusCode, &'static str),
    Unavailable(&'static str),
    Internal(&'static str),
}

impl ScriptedClient {
    fn new(script: Vec<Outcome>) -> Self {
        Self {
            calls: AtomicU32::new(0),
            script,
        }
    }

    fn call_count(&self) -> u32 {
        self.calls.load(Ordering::SeqCst)
    }

    async fn dispatch(&self) -> GrpcEgressResult<GrpcResponse> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        let idx = (n as usize).min(self.script.len().saturating_sub(1));
        let outcome = self.script.get(idx).cloned().unwrap_or(Outcome::Ok);
        match outcome {
            Outcome::Ok => Ok(GrpcResponse {
                body: b"ok".to_vec(),
                metadata: HashMap::new(),
            }),
            Outcome::Status(code, msg) => Err(GrpcEgressError::Status(code, msg.into())),
            Outcome::Unavailable(msg) => Err(GrpcEgressError::Unavailable(msg.into())),
            Outcome::Internal(msg) => Err(GrpcEgressError::Internal(msg.into())),
        }
    }
}

/// Newtype that owns an `Arc<ScriptedClient>` and implements
/// [`GrpcEgress`].  Avoids needing a blanket impl on `Arc<T>`,
/// which would conflict with the orphan rule.
struct SharedClient<T> {
    inner: Arc<T>,
}

impl<T> SharedClient<T> {
    fn new(inner: Arc<T>) -> Self {
        Self { inner }
    }
}

impl GrpcEgress for SharedClient<ScriptedClient> {
    fn call_unary(&self, _request: GrpcRequest) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move { inner.dispatch().await })
    }

    fn health_check(&self, _req: HealthCheckRequest) -> BoxFuture<'_, GrpcEgressResult<()>> {
        Box::pin(async { Ok(()) })
    }
}

fn fast_config() -> Result<GrpcRetryConfig, grpc_egress_retry::Error> {
    GrpcRetryConfig::from_config(
        r#"
            max_attempts = 5
            initial_backoff_ms = 1
            backoff_multiplier = 1.0
            jitter_factor = 0.0
            max_backoff_ms = 1
            rate_limit_max_attempts = 2
            rate_limit_initial_backoff_ms = 1
            rate_limit_max_backoff_ms = 1
        "#,
    )
}

fn make_request(deadline: Duration) -> GrpcRequest {
    GrpcRequest::new("svc.Test/Method", b"hello".to_vec(), deadline)
}

fn wrap(inner: Arc<ScriptedClient>, config: GrpcRetryConfig) -> impl GrpcEgress {
    wrap_with(SharedClient::new(inner), config)
}

fn wrap_with<T: GrpcEgress + Send + Sync + 'static>(
    inner: T,
    config: GrpcRetryConfig,
) -> impl GrpcEgress {
    StdGrpcRetryFactory::new(config).build(BuildGrpcRetryRequest { inner })
}

/// @covers: GrpcRetryConfig::default — SWE default has positive max_attempts.
#[test]
fn test_grpc_retry_config_default_has_positive_max_attempts() {
    let cfg = GrpcRetryConfig::default();
    assert!(cfg.max_attempts >= 1);
}

/// @covers: GrpcRetryClient — first-call success short-circuits.
#[tokio::test(flavor = "multi_thread")]
async fn test_first_attempt_success_does_not_retry() -> Result<(), Box<dyn std::error::Error>> {
    let inner = Arc::new(ScriptedClient::new(vec![Outcome::Ok]));
    let client = wrap(inner.clone(), fast_config()?);
    let resp = client
        .call_unary(make_request(Duration::from_secs(5)))
        .await?;
    assert_eq!(resp.body, b"ok");
    assert_eq!(inner.call_count(), 1, "no retry on success");
    Ok(())
}

/// @covers: classify(PermissionDenied) — NEVER retried.
///
/// **Acceptance test gate**: hand a stub returning
/// `PermissionDenied`, assert call_count == 1.
#[tokio::test(flavor = "multi_thread")]
async fn test_permission_denied_is_never_retried() -> Result<(), Box<dyn std::error::Error>> {
    let inner = Arc::new(ScriptedClient::new(vec![Outcome::Status(
        GrpcStatusCode::PermissionDenied,
        "no",
    )]));
    let client = wrap(inner.clone(), fast_config()?);
    let err = client
        .call_unary(make_request(Duration::from_secs(5)))
        .await
        .err()
        .ok_or("must fail")?;
    match err {
        GrpcEgressError::Status(GrpcStatusCode::PermissionDenied, _) => {}
        other => panic!("expected PermissionDenied, got {other:?}"),
    }
    assert_eq!(
        inner.call_count(),
        1,
        "PermissionDenied must not be retried (called {} times)",
        inner.call_count(),
    );
    Ok(())
}

/// @covers: classify(Unauthenticated) — NEVER retried.
#[tokio::test(flavor = "multi_thread")]
async fn test_unauthenticated_is_never_retried() -> Result<(), Box<dyn std::error::Error>> {
    let inner = Arc::new(ScriptedClient::new(vec![Outcome::Status(
        GrpcStatusCode::Unauthenticated,
        "bad token",
    )]));
    let client = wrap(inner.clone(), fast_config()?);
    let err = client
        .call_unary(make_request(Duration::from_secs(5)))
        .await
        .err()
        .ok_or("must fail")?;
    assert!(matches!(
        err,
        GrpcEgressError::Status(GrpcStatusCode::Unauthenticated, _)
    ));
    assert_eq!(inner.call_count(), 1, "Unauthenticated must not be retried");
    Ok(())
}

/// @covers: classify(Unavailable) — retries until success.
#[tokio::test(flavor = "multi_thread")]
async fn test_unavailable_then_ok_succeeds_after_retry() -> Result<(), Box<dyn std::error::Error>> {
    let inner = Arc::new(ScriptedClient::new(vec![
        Outcome::Status(GrpcStatusCode::Unavailable, "blip"),
        Outcome::Status(GrpcStatusCode::Unavailable, "blip"),
        Outcome::Ok,
    ]));
    let client = wrap(inner.clone(), fast_config()?);
    let resp = client
        .call_unary(make_request(Duration::from_secs(5)))
        .await?;
    assert_eq!(resp.body, b"ok");
    assert_eq!(inner.call_count(), 3, "two retries before the Ok");
    Ok(())
}

/// @covers: classify(Unavailable) — exhausts attempts.
#[tokio::test(flavor = "multi_thread")]
async fn test_unavailable_throughout_exhausts_max_attempts(
) -> Result<(), Box<dyn std::error::Error>> {
    let inner = Arc::new(ScriptedClient::new(vec![Outcome::Status(
        GrpcStatusCode::Unavailable,
        "down",
    )]));
    let client = wrap(inner.clone(), fast_config()?);
    let err = client
        .call_unary(make_request(Duration::from_secs(5)))
        .await
        .err()
        .ok_or("never recovers")?;
    assert!(matches!(
        err,
        GrpcEgressError::Status(GrpcStatusCode::Unavailable, _)
    ));
    assert_eq!(inner.call_count(), 5, "exhausts max_attempts");
    Ok(())
}

/// @covers: classify(Internal) — terminal, no retry.
#[tokio::test(flavor = "multi_thread")]
async fn test_internal_is_terminal() -> Result<(), Box<dyn std::error::Error>> {
    let inner = Arc::new(ScriptedClient::new(vec![Outcome::Status(
        GrpcStatusCode::Internal,
        "bug",
    )]));
    let client = wrap(inner.clone(), fast_config()?);
    let _ = client
        .call_unary(make_request(Duration::from_secs(5)))
        .await
        .err()
        .ok_or("terminal")?;
    assert_eq!(inner.call_count(), 1, "Internal must not be retried");
    Ok(())
}

/// @covers: classify(DeadlineExceeded) — terminal.
#[tokio::test(flavor = "multi_thread")]
async fn test_deadline_exceeded_is_terminal() -> Result<(), Box<dyn std::error::Error>> {
    let inner = Arc::new(ScriptedClient::new(vec![Outcome::Status(
        GrpcStatusCode::DeadlineExceeded,
        "tick",
    )]));
    let client = wrap(inner.clone(), fast_config()?);
    let _ = client
        .call_unary(make_request(Duration::from_secs(5)))
        .await
        .err()
        .ok_or("terminal")?;
    assert_eq!(
        inner.call_count(),
        1,
        "DeadlineExceeded counts retry budget — not retried",
    );
    Ok(())
}

/// @covers: classify(transport Unavailable) → retried.
#[tokio::test(flavor = "multi_thread")]
async fn test_transport_unavailable_retries_then_internal_terminates(
) -> Result<(), Box<dyn std::error::Error>> {
    let inner = Arc::new(ScriptedClient::new(vec![
        Outcome::Unavailable("transport blip"),
        Outcome::Internal("not retryable"),
    ]));
    let client = wrap(inner.clone(), fast_config()?);
    let _ = client
        .call_unary(make_request(Duration::from_secs(5)))
        .await
        .err()
        .ok_or("Internal is terminal")?;
    assert_eq!(inner.call_count(), 2);
    Ok(())
}

/// @covers: classify(ResourceExhausted RateLimit) — retried on rate-limit track.
#[tokio::test(flavor = "multi_thread")]
async fn test_resource_exhausted_rate_limit_retries_on_rate_limit_track(
) -> Result<(), Box<dyn std::error::Error>> {
    let inner = Arc::new(ScriptedClient::new(vec![
        Outcome::Status(GrpcStatusCode::ResourceExhausted, "rate limit exceeded"),
        Outcome::Status(GrpcStatusCode::ResourceExhausted, "rate limit exceeded"),
        Outcome::Ok,
    ]));
    let cfg = GrpcRetryConfig::from_config(
        r#"
            max_attempts = 5
            initial_backoff_ms = 5
            backoff_multiplier = 1.0
            jitter_factor = 0.0
            max_backoff_ms = 100
            rate_limit_max_attempts = 2
            rate_limit_initial_backoff_ms = 1
            rate_limit_max_backoff_ms = 100
        "#,
    )?;
    let client = wrap_with(SharedClient::new(inner.clone()), cfg);
    let resp = client
        .call_unary(make_request(Duration::from_secs(5)))
        .await?;
    assert_eq!(resp.body, b"ok");
    assert_eq!(inner.call_count(), 3);
    Ok(())
}

/// @covers: classify(ResourceExhausted HardQuota) — never retried.
#[tokio::test(flavor = "multi_thread")]
async fn test_resource_exhausted_hard_quota_is_terminal() -> Result<(), Box<dyn std::error::Error>>
{
    let inner = Arc::new(ScriptedClient::new(vec![
        Outcome::Status(GrpcStatusCode::ResourceExhausted, "quota exceeded"),
        Outcome::Ok,
    ]));
    let client = wrap(inner.clone(), fast_config()?);
    let _ = client
        .call_unary(make_request(Duration::from_secs(5)))
        .await
        .err()
        .ok_or("HardQuota is terminal, must not succeed")?;
    assert_eq!(inner.call_count(), 1, "HardQuota must not retry");
    Ok(())
}

/// @covers: classify(ResourceExhausted Capacity) — retried on standard track.
#[tokio::test(flavor = "multi_thread")]
async fn test_resource_exhausted_capacity_retries_on_standard_track(
) -> Result<(), Box<dyn std::error::Error>> {
    let inner = Arc::new(ScriptedClient::new(vec![
        Outcome::Status(GrpcStatusCode::ResourceExhausted, "server overloaded"),
        Outcome::Ok,
    ]));
    let client = wrap(inner.clone(), fast_config()?);
    let resp = client
        .call_unary(make_request(Duration::from_secs(5)))
        .await?;
    assert_eq!(resp.body, b"ok");
    assert_eq!(inner.call_count(), 2);
    Ok(())
}

/// @covers: deadline budget — retry stops when budget exhausted.
///
/// **Acceptance test gate**: total attempts ≤ deadline / initial_backoff.
///
/// Slow stub: each call sleeps for ~50ms before failing with
/// `Unavailable`.  The total deadline is 200ms.  Even though the
/// config allows up to 100 attempts, the retry loop must abandon
/// when the deadline is exhausted.
#[tokio::test(flavor = "multi_thread")]
async fn test_retry_honors_caller_deadline_as_total_budget(
) -> Result<(), Box<dyn std::error::Error>> {
    struct Slow {
        calls: AtomicU32,
        per_call: Duration,
    }

    struct SharedSlow(Arc<Slow>);

    impl GrpcEgress for SharedSlow {
        fn call_unary(&self, _r: GrpcRequest) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
            let inner = Arc::clone(&self.0);
            Box::pin(async move {
                inner.calls.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(inner.per_call).await;
                Err(GrpcEgressError::Status(
                    GrpcStatusCode::Unavailable,
                    "slow blip".into(),
                ))
            })
        }
        fn health_check(&self, _req: HealthCheckRequest) -> BoxFuture<'_, GrpcEgressResult<()>> {
            Box::pin(async { Ok(()) })
        }
    }

    let stats = Arc::new(Slow {
        calls: AtomicU32::new(0),
        per_call: Duration::from_millis(50),
    });

    let cfg = GrpcRetryConfig::from_config(
        r#"
            max_attempts = 100
            initial_backoff_ms = 10
            backoff_multiplier = 1.0
            jitter_factor = 0.0
            max_backoff_ms = 10
            rate_limit_max_attempts = 2
            rate_limit_initial_backoff_ms = 10
            rate_limit_max_backoff_ms = 10
        "#,
    )?;
    let client = wrap_with(SharedSlow(stats.clone()), cfg);

    let total_budget = Duration::from_millis(200);
    let started = Instant::now();
    let _ = client
        .call_unary(make_request(total_budget))
        .await
        .err()
        .ok_or("budget exhausts")?;
    let elapsed = started.elapsed();

    let calls = stats.calls.load(Ordering::SeqCst);
    assert!(
        calls < 100,
        "calls ({calls}) reached max_attempts — deadline appears ignored",
    );
    let ceiling = 1 + (total_budget.as_millis() / 10) as u32;
    assert!(
        calls <= ceiling,
        "calls ({calls}) exceeded ceiling deadline / initial_backoff = {ceiling}",
    );
    assert!(
        elapsed < total_budget * 3,
        "retry loop ran for {elapsed:?}, far past budget {total_budget:?}",
    );
    Ok(())
}

/// @covers: GrpcRetryConfig::section_name
#[test]
fn test_grpc_retry_config_section_name_is_grpc_retry() {
    use swe_edge_configbuilder::ConfigSection as _;
    assert_eq!(GrpcRetryConfig::section_name(), "grpc_retry");
}

/// @covers: StdGrpcRetryFactory::build — the pause between attempts is the one the shared backoff
/// scheduler of `edge-transport-retry` computes for the policy.
#[tokio::test(flavor = "multi_thread")]
async fn test_unavailable_throughout_pauses_at_least_the_backoff_of_the_shared_scheduler(
) -> Result<(), Box<dyn std::error::Error>> {
    use edge_transport_retry::BackoffScheduler;

    let cfg = GrpcRetryConfig::from_config(
        r#"
            max_attempts = 3
            initial_backoff_ms = 40
            backoff_multiplier = 2.0
            jitter_factor = 0.0
            max_backoff_ms = 1000
            rate_limit_max_attempts = 2
            rate_limit_initial_backoff_ms = 1
            rate_limit_max_backoff_ms = 1
        "#,
    )?;
    let expected =
        BackoffScheduler::next_backoff(&cfg, 0, 0.0) + BackoffScheduler::next_backoff(&cfg, 1, 0.0);
    let inner = Arc::new(ScriptedClient::new(vec![Outcome::Unavailable("down")]));
    let client = wrap(inner.clone(), cfg);

    let started = Instant::now();
    let _ = client
        .call_unary(make_request(Duration::from_secs(5)))
        .await;
    let elapsed = started.elapsed();

    assert_eq!(inner.call_count(), 3, "every attempt reaches the client");
    assert!(
        elapsed >= expected,
        "paused {elapsed:?}, the scheduler says at least {expected:?}"
    );
    assert!(
        elapsed < expected + Duration::from_millis(1000),
        "paused {elapsed:?}, far more than the {expected:?} of the scheduler"
    );
    Ok(())
}

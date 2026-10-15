#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for the gRPC circuit-breaker decorator.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use edge_transport_grpc_egress::{
    GrpcEgress, GrpcEgressError, GrpcEgressResult, GrpcRequest, GrpcResponse, GrpcStatusCode,
    HealthCheckRequest,
};
use edge_transport_grpc_egress_breaker::{
    BreakerState, GrpcBreakerClient, GrpcBreakerConfig, GrpcBreakerFacade,
};
use futures::future::BoxFuture;

/// Stub `GrpcEgress` whose outcome the test toggles at runtime.
///
/// Tracks the call count so tests can verify that an Open
/// breaker doesn't reach the inner client.
struct ToggleClient {
    calls: AtomicU32,
    /// Numeric mode: 0 = Ok, 1 = Status(Unavailable), 2 = Status(Internal).
    mode: std::sync::atomic::AtomicU8,
}

impl ToggleClient {
    fn new(initial: u8) -> Self {
        Self {
            calls: AtomicU32::new(0),
            mode: std::sync::atomic::AtomicU8::new(initial),
        }
    }
    fn call_count(&self) -> u32 {
        self.calls.load(Ordering::SeqCst)
    }
    fn set_mode(&self, m: u8) {
        self.mode.store(m, Ordering::SeqCst);
    }
}

struct Shared(Arc<ToggleClient>);

impl GrpcEgress for Shared {
    fn call_unary(&self, _r: GrpcRequest) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
        let inner = Arc::clone(&self.0);
        Box::pin(async move {
            inner.calls.fetch_add(1, Ordering::SeqCst);
            match inner.mode.load(Ordering::SeqCst) {
                0 => Ok(GrpcResponse {
                    body: b"ok".to_vec(),
                    metadata: HashMap::new(),
                }),
                1 => Err(GrpcEgressError::Status(
                    GrpcStatusCode::Unavailable,
                    "down".into(),
                )),
                2 => Err(GrpcEgressError::Status(
                    GrpcStatusCode::Internal,
                    "bug".into(),
                )),
                3 => Err(GrpcEgressError::Status(
                    GrpcStatusCode::PermissionDenied,
                    "no".into(),
                )),
                _ => unreachable!(),
            }
        })
    }
    fn health_check(&self, _req: HealthCheckRequest) -> BoxFuture<'_, GrpcEgressResult<()>> {
        Box::pin(async { Ok(()) })
    }
}

fn fast_cfg() -> GrpcBreakerConfig {
    GrpcBreakerConfig::from_config(
        r#"
            failure_threshold = 3
            cool_down_seconds = 1
            half_open_probe_count = 1
        "#,
    )
    .unwrap()
}

fn make_request() -> GrpcRequest {
    GrpcRequest::new("svc.Test/Method", b"hi".to_vec(), Duration::from_secs(5))
}

/// @covers: GrpcBreakerConfig::default — SWE default has positive threshold.
#[test]
fn test_grpc_breaker_config_default_has_positive_failure_threshold() {
    let cfg = GrpcBreakerConfig::default();
    assert!(cfg.failure_threshold >= 1);
}

/// @covers: GrpcBreakerClient — passes through when Closed.
#[tokio::test(flavor = "multi_thread")]
async fn test_closed_state_passes_through_success() {
    let inner = Arc::new(ToggleClient::new(0));
    let client = GrpcBreakerClient::new(Shared(inner.clone()), fast_cfg());
    let resp = client.call_unary(make_request()).await.expect("ok");
    assert_eq!(resp.body, b"ok");
    assert_eq!(inner.call_count(), 1);
    assert_eq!(client.state().await, BreakerState::Closed);
}

/// @covers: state machine — N failures trip Open; Open
/// short-circuits without calling inner.
///
/// **Acceptance test gate**: tripped state returns `Unavailable`
/// immediately without calling inner (verify via call counter).
#[tokio::test(flavor = "multi_thread")]
async fn test_open_state_short_circuits_without_calling_inner() {
    let inner = Arc::new(ToggleClient::new(1)); // Unavailable
    let client = GrpcBreakerClient::new(Shared(inner.clone()), fast_cfg());

    // Trip it.
    for _ in 0..3 {
        let _ = client.call_unary(make_request()).await;
    }
    assert!(matches!(client.state().await, BreakerState::Open { .. }));
    assert_eq!(inner.call_count(), 3, "exactly the threshold trips it");

    // Subsequent calls must NOT reach inner.
    for _ in 0..5 {
        let err = client
            .call_unary(make_request())
            .await
            .expect_err("must reject");
        assert!(matches!(err, GrpcEgressError::Unavailable(_)));
    }
    assert_eq!(
        inner.call_count(),
        3,
        "Open state must NOT call inner ({} additional calls leaked)",
        inner.call_count() - 3,
    );
}

/// @covers: state machine — full lifecycle:
/// closed → N failures → open → cool-down → half-open → success → closed.
#[tokio::test(flavor = "multi_thread")]
async fn test_full_state_lifecycle_closed_open_half_open_closed() {
    let inner = Arc::new(ToggleClient::new(1)); // Unavailable
    let client = GrpcBreakerClient::new(Shared(inner.clone()), fast_cfg());

    // 1) Closed → Open after 3 failures.
    assert_eq!(client.state().await, BreakerState::Closed);
    for _ in 0..3 {
        let _ = client.call_unary(make_request()).await;
    }
    assert!(matches!(client.state().await, BreakerState::Open { .. }));

    // 2) Cool-down: while in Open, reject without calling inner.
    let calls_when_opened = inner.call_count();
    let _ = client.call_unary(make_request()).await;
    assert_eq!(
        inner.call_count(),
        calls_when_opened,
        "Open must short-circuit during cool-down",
    );

    // 3) Wait past cool-down (1s).  Switch the stub to Ok so
    //    the half-open probe succeeds.
    tokio::time::sleep(Duration::from_millis(1100)).await;
    inner.set_mode(0);

    // 4) Open → HalfOpen probe → Closed.
    let resp = client
        .call_unary(make_request())
        .await
        .expect("probe ok closes breaker");
    assert_eq!(resp.body, b"ok");
    assert_eq!(client.state().await, BreakerState::Closed);
}

/// @covers: state machine — half-open probe failure returns to Open.
#[tokio::test(flavor = "multi_thread")]
async fn test_half_open_probe_failure_returns_to_open() {
    let inner = Arc::new(ToggleClient::new(1));
    let client = GrpcBreakerClient::new(Shared(inner.clone()), fast_cfg());

    // Trip to Open.
    for _ in 0..3 {
        let _ = client.call_unary(make_request()).await;
    }
    assert!(matches!(client.state().await, BreakerState::Open { .. }));

    // Wait past cool-down so the next call promotes to HalfOpen,
    // but keep the stub failing so the probe fails.
    tokio::time::sleep(Duration::from_millis(1100)).await;

    let _ = client.call_unary(make_request()).await;
    assert!(
        matches!(client.state().await, BreakerState::Open { .. }),
        "failed probe must return to Open",
    );
}

/// @covers: failure_kind — auth failures do NOT trip the breaker.
#[tokio::test(flavor = "multi_thread")]
async fn test_permission_denied_does_not_trip_breaker() {
    let inner = Arc::new(ToggleClient::new(3)); // PermissionDenied
    let client = GrpcBreakerClient::new(Shared(inner.clone()), fast_cfg());

    for _ in 0..10 {
        let _ = client.call_unary(make_request()).await;
    }
    assert_eq!(
        client.state().await,
        BreakerState::Closed,
        "auth failures must not count toward the breaker threshold",
    );
    assert_eq!(inner.call_count(), 10);
}

/// @covers: failure_kind — Internal status counts as failure.
#[tokio::test(flavor = "multi_thread")]
async fn test_internal_counts_toward_threshold() {
    let inner = Arc::new(ToggleClient::new(2)); // Internal
    let client = GrpcBreakerClient::new(Shared(inner.clone()), fast_cfg());
    for _ in 0..3 {
        let _ = client.call_unary(make_request()).await;
    }
    assert!(matches!(client.state().await, BreakerState::Open { .. }));
}

/// @covers: state — Closed reset on intermittent success.
#[tokio::test(flavor = "multi_thread")]
async fn test_intermittent_success_keeps_breaker_closed() {
    let inner = Arc::new(ToggleClient::new(1));
    let client = GrpcBreakerClient::new(Shared(inner.clone()), fast_cfg());

    // 2 failures, then 1 success: counter resets.
    let _ = client.call_unary(make_request()).await;
    let _ = client.call_unary(make_request()).await;
    inner.set_mode(0);
    let _ = client.call_unary(make_request()).await;
    assert_eq!(client.state().await, BreakerState::Closed);

    // Now 2 more failures: still Closed (was reset).
    inner.set_mode(1);
    let _ = client.call_unary(make_request()).await;
    let _ = client.call_unary(make_request()).await;
    assert_eq!(client.state().await, BreakerState::Closed);
}

/// @covers: GrpcBreakerFacade::create_breaker_client
#[tokio::test(flavor = "multi_thread")]
async fn test_create_breaker_client_wraps_inner_with_default_config() {
    let inner = Arc::new(ToggleClient::new(1)); // Unavailable
    let client = GrpcBreakerFacade::create_breaker_client(Shared(inner.clone()))
        .expect("create_breaker_client is infallible");
    let default_cfg = GrpcBreakerConfig::default();

    // Drive exactly (default_cfg.failure_threshold - 1) failures: breaker
    // must still be closed (accepting calls), proving the DEFAULT
    // threshold — not some other value — was genuinely applied.
    for _ in 0..default_cfg.failure_threshold - 1 {
        let _ = client.call_unary(make_request()).await;
    }
    assert_eq!(
        inner.call_count(),
        default_cfg.failure_threshold - 1,
        "breaker must still be passing calls through below the default threshold"
    );

    // One more failure reaches the default threshold: breaker opens and the
    // NEXT call must short-circuit without reaching inner.
    let _ = client.call_unary(make_request()).await;
    let calls_at_threshold = inner.call_count();
    assert_eq!(calls_at_threshold, default_cfg.failure_threshold);
    let err = client
        .call_unary(make_request())
        .await
        .expect_err("breaker must reject once open");
    assert!(matches!(err, GrpcEgressError::Unavailable(_)));
    assert_eq!(
        inner.call_count(),
        calls_at_threshold,
        "Open state must not call inner"
    );
}

/// @covers: GrpcBreakerFacade::wrap_breaker
#[tokio::test(flavor = "multi_thread")]
async fn test_wrap_breaker_produces_client_with_supplied_config() {
    let cfg = GrpcBreakerConfig {
        failure_threshold: 3,
        cool_down_seconds: 5,
        half_open_probe_count: 1,
    };
    let inner = Arc::new(ToggleClient::new(1)); // Unavailable
    let client = GrpcBreakerFacade::wrap_breaker(Shared(inner.clone()), cfg)
        .expect("wrap_breaker is infallible");

    // The supplied threshold is 3 — two failures must not yet trip it.
    for _ in 0..2 {
        let _ = client.call_unary(make_request()).await;
    }
    assert_eq!(inner.call_count(), 2);

    // The third failure reaches the supplied threshold and opens the breaker.
    let _ = client.call_unary(make_request()).await;
    assert_eq!(inner.call_count(), 3);
    let err = client
        .call_unary(make_request())
        .await
        .expect_err("breaker must reject once open at the supplied threshold");
    assert!(matches!(err, GrpcEgressError::Unavailable(_)));
    assert_eq!(inner.call_count(), 3, "Open state must not call inner");
}

#[derive(serde::Deserialize, Default, PartialEq, Debug)]
struct AbsentSectionProbe {
    marker: bool,
}

/// @covers: GrpcBreakerFacade::create_config_builder
#[test]
fn test_create_config_builder_builds_loader() {
    let loader = GrpcBreakerFacade::create_config_builder()
        .expect("create_config_builder is infallible")
        .build_loader()
        .expect("a builder pre-seeded with name and version must build a valid loader");
    // In a test environment there is no application.toml at any configured
    // directory, so querying any section must fail with NotFound — proves
    // the loader is genuinely wired to the filesystem, not a no-op stub.
    let err = loader
        .load_section::<AbsentSectionProbe>("breaker_test_probe_section_that_does_not_exist")
        .expect_err("no config directory exists in the test environment");
    assert!(
        err.to_string()
            .contains("breaker_test_probe_section_that_does_not_exist"),
        "error must name the missing section, got: {err}"
    );
}

/// @covers: GrpcBreakerConfig::section_name
#[test]
fn test_grpc_breaker_config_section_name_is_grpc_breaker() {
    use swe_edge_configbuilder::ConfigSection as _;
    assert_eq!(GrpcBreakerConfig::section_name(), "grpc_breaker");
}

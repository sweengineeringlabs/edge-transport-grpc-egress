//! Integration tests for the resilient gRPC transport stack.
//!
//! The first group wraps a [`ScriptedClient`] (a client that answers from a script) in the real retry and
//! circuit-breaker decorators, so their logic runs against chosen response sequences: a real peer answers one way
//! for every call, and these cases need a failure at a chosen call. The second group runs the whole stack against
//! a real in-process gRPC peer and the real engine client.

use grpc_egress_transport::GrpcChannelConfigBuilder as _;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use grpc_egress_breaker::{
    BreakerDomainError, BreakerObservable, BreakerState, BuildGrpcBreakerRequest,
    GrpcBreakerConfig, GrpcBreakerFactory, ObserveStateRequest,
};
use grpc_egress_breaker_std::StdGrpcBreakerFactory;
use grpc_egress_resilient::{
    BuildGrpcResilientRequest, GrpcResilientFactory, ResilienceConfig, ResilientTransportError,
};
use grpc_egress_resilient_std::StdGrpcResilientFactory;
use grpc_egress_retry::{BuildGrpcRetryRequest, GrpcRetryConfig, GrpcRetryFactory};
use grpc_egress_retry_std::StdGrpcRetryFactory;
use grpc_egress_test_support::{CallFixture, ClientFixture, Fixture, PeerBehaviour, PeerServer};
use proto_pattern::{
    BidiStreamMethod, CallError, CallOptions, CallRequest, CallResponse, ClientStreamMethod,
    DecodeError, EncodeError, MessageStream, Metadata, Method, MethodPath, Payload, RawTransport,
    ResponseStream, ServerStreamMethod, StatusCode, Transport, UnaryMethod,
};

const PATH: &str = "/svc/Method";

// ── helpers ──────────────────────────────────────────────────────────────────

fn guard<T: Transport + RawTransport + Send + Sync + 'static>(
    inner: T,
    config: GrpcBreakerConfig,
) -> Result<impl Transport + RawTransport + BreakerObservable, BreakerDomainError> {
    StdGrpcBreakerFactory::new(config).build(BuildGrpcBreakerRequest { inner })
}

fn apply_resilience<T: Transport + RawTransport + Send + Sync + 'static>(
    inner: T,
    config: ResilienceConfig,
) -> Result<impl Transport + RawTransport + BreakerObservable, ResilientTransportError> {
    StdGrpcResilientFactory::new(config).build(BuildGrpcResilientRequest { inner })
}

async fn state_of(client: &impl BreakerObservable) -> Result<BreakerState, BreakerDomainError> {
    Ok(client.state(ObserveStateRequest).await?.state)
}

fn path() -> Result<MethodPath, Box<dyn std::error::Error>> {
    Ok(MethodPath::create(PATH)?)
}

fn req() -> CallRequest<Vec<u8>> {
    CallRequest::create(
        Vec::new(),
        CallOptions::create(Some(Duration::from_secs(10)), Metadata::default()),
    )
}

type Answer = Result<(), CallError>;

fn ok_response() -> Answer {
    Ok(())
}

fn status(code: StatusCode, message: &str) -> Answer {
    Err(CallError::Status {
        path: PATH.into(),
        code,
        message: message.to_string(),
        details: Vec::new(),
        trailers: Metadata::default(),
    })
}

fn unavailable() -> Answer {
    status(StatusCode::Unavailable, "upstream down")
}

fn hard_quota() -> Answer {
    status(StatusCode::ResourceExhausted, "quota exceeded")
}

fn rate_limited() -> Answer {
    status(StatusCode::ResourceExhausted, "rate limit exceeded")
}

fn valid_resilience() -> ResilienceConfig {
    ResilienceConfig {
        max_attempts: 3,
        initial_backoff_ms: 10,
        backoff_multiplier: 2.0,
        jitter_factor: 0.0,
        max_backoff_ms: 100,
        rate_limit_max_attempts: 2,
        rate_limit_initial_backoff_ms: 10,
        rate_limit_max_backoff_ms: 100,
        failure_threshold: 5,
        cool_down_seconds: 10,
        half_open_probe_count: 1,
    }
}

// ── ScriptedClient ─────────────────────────────────────────────────────────────

/// A client that counts its calls and answers them from a script, in order; past the end of the script it
/// succeeds. The decorators around it are the real ones; only the answers are scripted, so that a failure can be
/// made to happen at a chosen call. A typed call and a raw call take their answers from the same script.
struct ScriptedClient {
    hits: Arc<AtomicUsize>,
    queue: Arc<Mutex<VecDeque<Answer>>>,
}

impl ScriptedClient {
    fn with_responses(hits: Arc<AtomicUsize>, responses: Vec<Answer>) -> Self {
        Self {
            hits,
            queue: Arc::new(Mutex::new(responses.into_iter().collect())),
        }
    }

    fn answer(&self) -> Answer {
        self.hits.fetch_add(1, Ordering::SeqCst);
        self.queue
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop_front()
            .unwrap_or_else(ok_response)
    }

    fn refusal(path: &str) -> CallError {
        CallError::Transport {
            path: path.into(),
            code: StatusCode::Unimplemented,
            reason: "the scripted client does not stream".to_string(),
        }
    }
}

/// The response stream of a call the scripted client refuses.
struct NoStream {
    headers: Metadata,
}

impl<T: Send> MessageStream<T> for NoStream {
    async fn read_message(&mut self) -> Result<Option<T>, CallError> {
        Ok(None)
    }
}

impl<T: Send> ResponseStream<T> for NoStream {
    type Metadata = Metadata;

    fn get_headers(&self) -> &Metadata {
        &self.headers
    }

    async fn read_trailers(&mut self) -> Result<Metadata, CallError> {
        Ok(Metadata::default())
    }
}

impl RawTransport for ScriptedClient {
    type Metadata = Metadata;

    async fn call_raw(
        &self,
        _path: MethodPath,
        _request: CallRequest<Vec<u8>>,
    ) -> Result<CallResponse<Vec<u8>, Metadata>, CallError> {
        self.answer()?;
        Ok(CallResponse::create(
            Vec::new(),
            Metadata::default(),
            Metadata::default(),
        ))
    }

    async fn call_raw_server_stream(
        &self,
        path: MethodPath,
        _request: CallRequest<Vec<u8>>,
    ) -> Result<impl ResponseStream<Vec<u8>, Metadata = Metadata>, CallError> {
        Err::<NoStream, _>(Self::refusal(path.get_path()))
    }

    async fn call_raw_client_stream<S: MessageStream<Vec<u8>> + 'static>(
        &self,
        path: MethodPath,
        _requests: S,
        _options: CallOptions,
    ) -> Result<CallResponse<Vec<u8>, Metadata>, CallError> {
        Err(Self::refusal(path.get_path()))
    }

    async fn call_raw_bidi_stream<S: MessageStream<Vec<u8>> + 'static>(
        &self,
        path: MethodPath,
        _requests: S,
        _options: CallOptions,
    ) -> Result<impl ResponseStream<Vec<u8>, Metadata = Metadata>, CallError> {
        Err::<NoStream, _>(Self::refusal(path.get_path()))
    }
}

impl Transport for ScriptedClient {
    type Metadata = Metadata;

    async fn call<M: UnaryMethod>(
        &self,
        _request: CallRequest<M::Request>,
    ) -> Result<CallResponse<M::Response, Metadata>, CallError> {
        self.answer()?;
        let answer = M::Response::decode(b"ok").map_err(CallError::Decode)?;
        Ok(CallResponse::create(
            answer,
            Metadata::default(),
            Metadata::default(),
        ))
    }

    async fn call_server_stream<M: ServerStreamMethod>(
        &self,
        _request: CallRequest<M::Request>,
    ) -> Result<impl ResponseStream<M::Response, Metadata = Metadata>, CallError> {
        Err::<NoStream, _>(Self::refusal(M::PATH))
    }

    async fn call_client_stream<M: ClientStreamMethod, S: MessageStream<M::Request> + 'static>(
        &self,
        _requests: S,
        _options: CallOptions,
    ) -> Result<CallResponse<M::Response, Metadata>, CallError> {
        Err(Self::refusal(M::PATH))
    }

    async fn call_bidi_stream<M: BidiStreamMethod, S: MessageStream<M::Request> + 'static>(
        &self,
        _requests: S,
        _options: CallOptions,
    ) -> Result<impl ResponseStream<M::Response, Metadata = Metadata>, CallError> {
        Err::<NoStream, _>(Self::refusal(M::PATH))
    }
}

/// A text message, as a `Payload`: its encoding is its bytes.
#[derive(Debug, Clone, PartialEq)]
struct Text(String);

impl Payload for Text {
    const FULL_NAME: &'static str = "test.Text";

    fn encode_into(&self, out: &mut Vec<u8>) -> Result<(), EncodeError> {
        out.extend_from_slice(self.0.as_bytes());
        Ok(())
    }

    fn estimate_encoded_len(&self) -> usize {
        self.0.len()
    }

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        String::from_utf8(bytes.to_vec())
            .map(Text)
            .map_err(|e| DecodeError::create(Self::FULL_NAME, e.to_string()))
    }
}

struct Echo;

impl Method for Echo {
    type Request = Text;
    type Response = Text;
    const PATH: &'static str = PATH;
}

impl UnaryMethod for Echo {}

// Minimal retry config suitable for fast tests (1 ms backoff).
fn fast_retry(max_attempts: u32) -> GrpcRetryConfig {
    GrpcRetryConfig {
        max_attempts,
        initial_backoff_ms: 1,
        backoff_multiplier: 1.0,
        jitter_factor: 0.0,
        max_backoff_ms: 1,
        rate_limit_max_attempts: 2,
        rate_limit_initial_backoff_ms: 1,
        rate_limit_max_backoff_ms: 1,
    }
}

fn never_trips() -> GrpcBreakerConfig {
    GrpcBreakerConfig {
        failure_threshold: 100,
        cool_down_seconds: 60,
        half_open_probe_count: 1,
    }
}

// ── facade smoke tests ────────────────────────────────────────────────────────
//
// `StdGrpcResilientFactory` is a decorator only — it never builds a base client (that is the composition root's
// job, ADR-004 amendment). These tests wrap `ScriptedClient` rather than a real transport; the stack against a real
// peer is the second group of this file.

/// @covers: apply_resilience
#[tokio::test]
async fn test_apply_resilience_default_config_wraps_and_delegates_happy(
) -> Result<(), Box<dyn std::error::Error>> {
    let hits = Arc::new(AtomicUsize::new(0));
    let mock = ScriptedClient::with_responses(Arc::clone(&hits), vec![ok_response()]);
    let wrapped = apply_resilience(mock, ResilienceConfig::default())?;
    let result = wrapped.call_raw(path()?, req()).await;
    assert!(result.is_ok(), "the wrapped client must delegate through");
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    Ok(())
}

/// @covers: apply_resilience — a transient failure is genuinely retried
#[tokio::test]
async fn test_apply_resilience_retries_a_transient_failure_happy(
) -> Result<(), Box<dyn std::error::Error>> {
    let hits = Arc::new(AtomicUsize::new(0));
    let mock =
        ScriptedClient::with_responses(Arc::clone(&hits), vec![unavailable(), ok_response()]);
    let wrapped = apply_resilience(mock, valid_resilience())?;
    let result = wrapped.call_raw(path()?, req()).await;
    assert!(result.is_ok(), "must succeed on the retried attempt");
    assert_eq!(
        hits.load(Ordering::SeqCst),
        2,
        "expected exactly one retry after the first transient failure"
    );
    Ok(())
}

/// A typed call goes through the stack as a typed call and is retried like a raw one.
///
/// @covers: apply_resilience — typed unary
#[tokio::test]
async fn test_apply_resilience_retries_a_typed_transient_failure_happy(
) -> Result<(), Box<dyn std::error::Error>> {
    let hits = Arc::new(AtomicUsize::new(0));
    let mock =
        ScriptedClient::with_responses(Arc::clone(&hits), vec![unavailable(), ok_response()]);
    let wrapped = apply_resilience(mock, valid_resilience())?;
    let request = CallRequest::create(
        Text("typed".to_string()),
        CallOptions::create(Some(Duration::from_secs(10)), Metadata::default()),
    );

    let response = wrapped.call::<Echo>(request).await?;

    assert_eq!(response.get_message(), &Text("ok".to_string()));
    assert_eq!(hits.load(Ordering::SeqCst), 2);
    Ok(())
}

/// @covers: apply_resilience
#[test]
fn test_apply_resilience_invalid_resilience_returns_error_edge() {
    let hits = Arc::new(AtomicUsize::new(0));
    let mock = ScriptedClient::with_responses(hits, vec![ok_response()]);
    let mut r = valid_resilience();
    r.max_attempts = 0;
    assert!(matches!(
        apply_resilience(mock, r),
        Err(ResilientTransportError::InvalidResilience(_))
    ));
}

// ── retry amplification ───────────────────────────────────────────────────────

/// @covers: retry amplification — factor ≤ 1.5× under 20% failure rate
///
/// 10 client calls; 2 fail once then succeed on the first retry.
/// Expected server hits: 10 + 2 = 12 (1.2× amplification ≤ 1.5× threshold).
#[tokio::test]
async fn test_retry_amplification_factor_does_not_exceed_one_point_five_times(
) -> Result<(), Box<dyn std::error::Error>> {
    let hits = Arc::new(AtomicUsize::new(0));
    // Responses in pop order:
    // call 1: ok              → 1 hit
    // call 2: unavail → ok    → 2 hits (1 retry)
    // calls 3-8: ok ×6        → 6 hits
    // call 9: unavail → ok    → 2 hits (1 retry)
    // call 10: ok             → 1 hit
    // total: 12 hits for 10 calls → 1.2×
    let responses = vec![
        ok_response(),
        unavailable(),
        ok_response(),
        ok_response(),
        ok_response(),
        ok_response(),
        ok_response(),
        ok_response(),
        ok_response(),
        unavailable(),
        ok_response(),
        ok_response(),
    ];
    let mock = ScriptedClient::with_responses(Arc::clone(&hits), responses);
    let client = guard(
        StdGrpcRetryFactory::new(fast_retry(3)).build(BuildGrpcRetryRequest { inner: mock }),
        GrpcBreakerConfig {
            failure_threshold: 100,
            cool_down_seconds: 60,
            half_open_probe_count: 1,
        },
    )?;

    for _ in 0..10_usize {
        let _ = client.call_raw(path()?, req()).await;
    }

    let server_hits = hits.load(Ordering::SeqCst);
    let n_calls = 10_usize;
    let amplification = server_hits as f64 / n_calls as f64;
    assert!(
        amplification <= 1.5,
        "amplification {amplification:.2}× exceeds 1.5× ({server_hits} hits / {n_calls} calls)",
    );
    Ok(())
}

// ── retry exhaustion ─────────────────────────────────────────────────────────

/// @covers: retry — all attempts exhausted returns the last error
///
/// max_attempts=3, all 3 fail with Unavailable.
/// Server is hit exactly 3 times; result is Err(Status(Unavailable)).
#[tokio::test]
async fn test_unavailable_error_is_retried_up_to_max_attempts_then_fails(
) -> Result<(), Box<dyn std::error::Error>> {
    let hits = Arc::new(AtomicUsize::new(0));
    let responses = vec![unavailable(), unavailable(), unavailable()];
    let mock = ScriptedClient::with_responses(Arc::clone(&hits), responses);
    let client = guard(
        StdGrpcRetryFactory::new(fast_retry(3)).build(BuildGrpcRetryRequest { inner: mock }),
        never_trips(),
    )?;

    let result = client.call_raw(path()?, req()).await;
    assert!(
        result.is_err(),
        "expected error after exhausting all attempts"
    );
    assert_eq!(
        hits.load(Ordering::SeqCst),
        3,
        "expected exactly 3 server hits"
    );
    Ok(())
}

// ── hard quota — never retried ────────────────────────────────────────────────

/// @covers: retry — HardQuota ResourceExhausted is never retried
///
/// max_attempts=3 would allow up to 3 hits, but HardQuota is Terminal.
/// Server must be hit exactly once.
#[tokio::test]
async fn test_hard_quota_resource_exhausted_is_not_retried(
) -> Result<(), Box<dyn std::error::Error>> {
    let hits = Arc::new(AtomicUsize::new(0));
    let responses = vec![hard_quota(), ok_response(), ok_response()]; // extra ok's unused
    let mock = ScriptedClient::with_responses(Arc::clone(&hits), responses);
    let client = guard(
        StdGrpcRetryFactory::new(fast_retry(3)).build(BuildGrpcRetryRequest { inner: mock }),
        never_trips(),
    )?;

    let result = client.call_raw(path()?, req()).await;
    assert!(result.is_err(), "expected quota error");
    assert_eq!(
        hits.load(Ordering::SeqCst),
        1,
        "HardQuota must not be retried — server should see exactly 1 hit",
    );
    Ok(())
}

// ── rate-limit retry track ────────────────────────────────────────────────────

/// @covers: retry — rate-limit error uses rate_limit_max_attempts track
///
/// rate_limit_max_attempts=2, standard max_attempts=5.
/// Semantics: 1 initial call + rate_limit_max_attempts retries = 3 total server hits,
/// which is less than the standard max_attempts ceiling of 5.
#[tokio::test]
async fn test_rate_limit_error_uses_rate_limit_retry_track_not_standard(
) -> Result<(), Box<dyn std::error::Error>> {
    let hits = Arc::new(AtomicUsize::new(0));
    // Queue 5 rate-limit responses; only 3 should be consumed (1 + 2 retries).
    let responses = vec![
        rate_limited(),
        rate_limited(),
        rate_limited(),
        rate_limited(),
        rate_limited(),
    ];
    let mock = ScriptedClient::with_responses(Arc::clone(&hits), responses);
    let retry_cfg = GrpcRetryConfig {
        max_attempts: 5,
        initial_backoff_ms: 1,
        backoff_multiplier: 1.0,
        jitter_factor: 0.0,
        max_backoff_ms: 1,
        rate_limit_max_attempts: 2,
        rate_limit_initial_backoff_ms: 1,
        rate_limit_max_backoff_ms: 1,
    };
    let client = guard(
        StdGrpcRetryFactory::new(retry_cfg).build(BuildGrpcRetryRequest { inner: mock }),
        never_trips(),
    )?;

    let result = client.call_raw(path()?, req()).await;
    assert!(result.is_err());
    // rate_limit_max_attempts=2: 1 initial + 2 retries = 3 total server hits (not 5).
    assert_eq!(
        hits.load(Ordering::SeqCst),
        3,
        "rate-limit track: 1 initial + rate_limit_max_attempts(2) retries = 3 total hits",
    );
    Ok(())
}

// ── circuit breaker — opens at threshold ─────────────────────────────────────

/// @covers: circuit breaker — opens after failure_threshold consecutive failures
///
/// failure_threshold=3, max_attempts=1.
/// After 3 failing calls, breaker is Open.
/// 4th call fast-fails without touching the server.
#[tokio::test]
async fn test_circuit_breaker_opens_after_consecutive_failures_at_threshold(
) -> Result<(), Box<dyn std::error::Error>> {
    let hits = Arc::new(AtomicUsize::new(0));
    let responses = vec![unavailable(), unavailable(), unavailable(), unavailable()];
    let mock = ScriptedClient::with_responses(Arc::clone(&hits), responses);
    let client = guard(
        StdGrpcRetryFactory::new(fast_retry(1)).build(BuildGrpcRetryRequest { inner: mock }),
        GrpcBreakerConfig {
            failure_threshold: 3,
            cool_down_seconds: 60,
            half_open_probe_count: 1,
        },
    )?;

    for _ in 0..3 {
        let _ = client.call_raw(path()?, req()).await;
    }

    assert_eq!(
        hits.load(Ordering::SeqCst),
        3,
        "expected 3 server hits to trip the breaker"
    );
    assert!(
        matches!(state_of(&client).await?, BreakerState::Open { .. }),
        "breaker must be Open after threshold failures",
    );

    // 4th call: breaker rejects without server hit.
    let result = client.call_raw(path()?, req()).await;
    assert!(
        matches!(
            &result,
            Err(CallError::Transport {
                code: StatusCode::Unavailable,
                ..
            })
        ),
        "breaker must refuse with a transport failure of Unavailable when Open, got {:?}",
        result.as_ref().err(),
    );
    assert_eq!(
        hits.load(Ordering::SeqCst),
        3,
        "server hit count must not increase after breaker opens",
    );
    Ok(())
}

// ── circuit breaker — recovers via HalfOpen ──────────────────────────────────

/// @covers: circuit breaker — Closed→Open→HalfOpen→Closed transition
///
/// Trips the breaker (3 failures), waits for cool-down (1.1 s), then sends
/// a probe that succeeds — confirming the breaker closes again.
///
/// NOTE: This test takes ≈1.1 s due to the real-time cool-down wait.
#[tokio::test]
async fn test_circuit_breaker_closes_after_probe_succeeds_during_half_open(
) -> Result<(), Box<dyn std::error::Error>> {
    let hits = Arc::new(AtomicUsize::new(0));
    let responses = vec![
        unavailable(),
        unavailable(),
        unavailable(),
        ok_response(), // probe after cool-down
        ok_response(), // final call to confirm breaker is closed
    ];
    let mock = ScriptedClient::with_responses(Arc::clone(&hits), responses);
    let client = guard(
        StdGrpcRetryFactory::new(fast_retry(1)).build(BuildGrpcRetryRequest { inner: mock }),
        GrpcBreakerConfig {
            failure_threshold: 3,
            cool_down_seconds: 1,
            half_open_probe_count: 1,
        },
    )?;

    // Trip the breaker.
    for _ in 0..3 {
        let _ = client.call_raw(path()?, req()).await;
    }
    assert!(matches!(
        state_of(&client).await?,
        BreakerState::Open { .. }
    ));

    // Wait for cool-down to expire.
    tokio::time::sleep(Duration::from_millis(1100)).await;

    // Probe: should be admitted (HalfOpen), succeeds, closes breaker.
    let probe = client.call_raw(path()?, req()).await;
    assert!(probe.is_ok(), "probe should succeed");
    assert_eq!(
        state_of(&client).await?,
        BreakerState::Closed,
        "breaker must be Closed after probe"
    );

    // One more call confirming normal operation.
    let final_result = client.call_raw(path()?, req()).await;
    assert!(final_result.is_ok());
    assert_eq!(hits.load(Ordering::SeqCst), 5);
    Ok(())
}

// ── the stack against a real peer ─────────────────────────────────────────────

/// The engine client of `endpoint` inside the resilient stack.
fn resilient_over(
    endpoint: &str,
    config: ResilienceConfig,
) -> Result<impl Transport + RawTransport + BreakerObservable, Box<dyn std::error::Error>> {
    Fixture::ensure_tls_provider();
    let inner = ClientFixture::from_config(&config_builder(endpoint).allow_plaintext().build()?)?;
    Ok(apply_resilience(inner, config)?)
}

/// @covers: StdGrpcResilientFactory::build — the stack reaches a real peer and returns its answer
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_through_the_stack_reaches_the_peer_and_returns_its_answer(
) -> Result<(), Box<dyn std::error::Error>> {
    let peer = PeerServer::start(PeerBehaviour::Reply(b"pong".to_vec())).await?;
    let client = resilient_over(&peer.get_endpoint(), valid_resilience())?;

    let response = client
        .call_raw(
            CallFixture::path("pkg.Service/Method"),
            CallFixture::request(b"ping", Duration::from_secs(5)),
        )
        .await?;

    assert_eq!(response.get_message(), b"pong");
    assert_eq!(peer.get_recorded_requests().len(), 1);
    assert_eq!(state_of(&client).await?, BreakerState::Closed);
    Ok(())
}

/// A peer that answers `Unavailable` to every call trips the breaker; once it is open the stack refuses calls
/// itself and the peer sees no more of them.
///
/// @covers: circuit breaker — opens against a real peer and short-circuits
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_unavailable_peer_trips_the_breaker_and_the_peer_sees_no_more_calls(
) -> Result<(), Box<dyn std::error::Error>> {
    let peer = PeerServer::start(PeerBehaviour::ReplyStatus {
        status: "14".to_string(),
        message: Some("down".to_string()),
        retry_after_secs: None,
    })
    .await?;
    let config = ResilienceConfig {
        max_attempts: 1,
        failure_threshold: 3,
        ..valid_resilience()
    };
    let client = resilient_over(&peer.get_endpoint(), config)?;

    for _ in 0..3 {
        let result = client
            .call_raw(
                CallFixture::path("pkg.Service/Method"),
                CallFixture::request(b"x", Duration::from_secs(5)),
            )
            .await;
        assert!(
            matches!(
                result,
                Err(CallError::Status {
                    code: StatusCode::Unavailable,
                    ..
                })
            ),
            "the peer's own status reaches the caller while the breaker is closed"
        );
    }
    assert_eq!(peer.get_recorded_requests().len(), 3);
    assert!(matches!(
        state_of(&client).await?,
        BreakerState::Open { .. }
    ));

    let refused = client
        .call_raw(
            CallFixture::path("pkg.Service/Method"),
            CallFixture::request(b"x", Duration::from_secs(5)),
        )
        .await;

    assert!(
        matches!(
            &refused,
            Err(CallError::Transport {
                code: StatusCode::Unavailable,
                ..
            })
        ),
        "an open breaker refuses with a transport failure, got {:?}",
        refused.as_ref().err()
    );
    assert_eq!(
        peer.get_recorded_requests().len(),
        3,
        "the open breaker must not let a call through to the peer"
    );
    Ok(())
}

/// The retry inside the stack re-sends to a real peer, `max_attempts` times, before the failure reaches the breaker.
///
/// @covers: StdGrpcResilientFactory::build — retry inside the breaker, against a real peer
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_unavailable_peer_is_retried_max_attempts_times_through_the_stack(
) -> Result<(), Box<dyn std::error::Error>> {
    let peer = PeerServer::start(PeerBehaviour::ReplyStatus {
        status: "14".to_string(),
        message: Some("down".to_string()),
        retry_after_secs: None,
    })
    .await?;
    let config = ResilienceConfig {
        max_attempts: 3,
        initial_backoff_ms: 1,
        backoff_multiplier: 1.0,
        max_backoff_ms: 1,
        failure_threshold: 100,
        ..valid_resilience()
    };
    let client = resilient_over(&peer.get_endpoint(), config)?;

    let result = client
        .call_raw(
            CallFixture::path("pkg.Service/Method"),
            CallFixture::request(b"x", Duration::from_secs(5)),
        )
        .await;

    assert!(result.is_err());
    assert_eq!(peer.get_recorded_requests().len(), 3);
    Ok(())
}

fn config_builder(
    endpoint: impl Into<String>,
) -> impl grpc_egress_transport::GrpcChannelConfigBuilder {
    new_config_builder().with_endpoint(endpoint)
}

fn new_config_builder() -> impl grpc_egress_transport::GrpcChannelConfigBuilder {
    use grpc_egress_transport::GrpcChannelConfigBuilderFactory;
    grpc_egress_transport_std::StdGrpcChannelConfigBuilderFactory
        .build(grpc_egress_transport::BuildGrpcChannelConfigBuilderRequest)
}

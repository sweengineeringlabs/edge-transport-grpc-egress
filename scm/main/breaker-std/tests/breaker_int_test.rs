//! Integration tests for the gRPC circuit-breaker decorator: the state machine, driven through a scripted
//! transport whose outcome the test sets at run time, for every call shape, typed and raw.
//!
//! The scripted transport is a test double because a real peer cannot be told to fail on the third call and
//! recover on the fifth; what a real peer and the real engine do is in `breaker_engine_int_test.rs`.

use grpc_egress_breaker_std::GrpcBreakerConfigProvider;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use grpc_egress_breaker::{
    BreakerDomainError, BreakerObservable, BreakerState, BuildGrpcBreakerRequest,
    GrpcBreakerConfig, GrpcBreakerFactory, ObserveStateRequest,
};
use grpc_egress_breaker_std::StdGrpcBreakerFactory;
use proto_pattern::{
    BidiStreamMethod, CallError, CallOptions, CallRequest, CallResponse, ClientStreamMethod,
    DecodeError, EncodeError, MessageStream, Metadata, Method, MethodPath, Payload, RawTransport,
    ResponseStream, ServerStreamMethod, StatusCode, Transport, UnaryMethod,
};

/// A message that is its own bytes.
#[derive(Debug, PartialEq)]
struct Bytes(Vec<u8>);

impl Payload for Bytes {
    const FULL_NAME: &'static str = "test.Bytes";

    fn encode_into(&self, out: &mut Vec<u8>) -> Result<(), EncodeError> {
        out.extend_from_slice(&self.0);
        Ok(())
    }

    fn estimate_encoded_len(&self) -> usize {
        self.0.len()
    }

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        Ok(Self(bytes.to_vec()))
    }
}

/// One method of every kind, all on the path `/svc.Test/Method`.
struct Ping;

impl Method for Ping {
    type Request = Bytes;
    type Response = Bytes;
    const PATH: &'static str = "/svc.Test/Method";
}

impl UnaryMethod for Ping {}
impl ServerStreamMethod for Ping {}
impl ClientStreamMethod for Ping {}
impl BidiStreamMethod for Ping {}

/// What the scripted transport answers.
#[derive(Clone, Copy)]
enum Mode {
    Ok,
    Status(StatusCode),
    Transport(StatusCode),
    Decode,
    Encode,
}

/// The outcome of the scripted transport, and how many calls reached it.
struct Script {
    calls: AtomicU32,
    mode: Mutex<Mode>,
}

impl Script {
    fn new(initial: Mode) -> Arc<Self> {
        Arc::new(Self {
            calls: AtomicU32::new(0),
            mode: Mutex::new(initial),
        })
    }

    fn call_count(&self) -> u32 {
        self.calls.load(Ordering::SeqCst)
    }

    fn set_mode(&self, mode: Mode) {
        *self.mode.lock().unwrap_or_else(PoisonError::into_inner) = mode;
    }

    /// The outcome for a call to `path`: `ok` when the script says success.
    fn answer<R>(&self, path: &str, ok: impl FnOnce() -> R) -> Result<R, CallError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mode = *self.mode.lock().unwrap_or_else(PoisonError::into_inner);
        match mode {
            Mode::Ok => Ok(ok()),
            Mode::Status(code) => Err(CallError::Status {
                path: path.into(),
                code,
                message: "scripted".to_string(),
                details: Vec::new(),
                trailers: Metadata::default(),
            }),
            Mode::Transport(code) => Err(CallError::Transport {
                path: path.into(),
                code,
                reason: "scripted".to_string(),
            }),
            Mode::Decode => Err(CallError::Decode(DecodeError::create(
                "test.Bytes",
                "scripted",
            ))),
            Mode::Encode => Err(CallError::Encode(EncodeError::create(
                "test.Bytes",
                "scripted",
            ))),
        }
    }
}

/// The stream a scripted open returns; nothing is ever read from it.
struct ScriptedStream(Metadata);

impl<T: Send> MessageStream<T> for ScriptedStream {
    async fn read_message(&mut self) -> Result<Option<T>, CallError> {
        Ok(None)
    }
}

impl<T: Send> ResponseStream<T> for ScriptedStream {
    type Metadata = Metadata;

    fn get_headers(&self) -> &Metadata {
        &self.0
    }

    async fn read_trailers(&mut self) -> Result<Metadata, CallError> {
        Ok(Metadata::default())
    }
}

/// A transport that does what its [`Script`] says, for every call shape, typed and raw.
struct ScriptedTransport(Arc<Script>);

fn ok_response<R>(message: R) -> CallResponse<R, Metadata> {
    CallResponse::create(message, Metadata::default(), Metadata::default())
}

impl Transport for ScriptedTransport {
    type Metadata = Metadata;

    async fn call<M: UnaryMethod>(
        &self,
        _request: CallRequest<M::Request>,
    ) -> Result<CallResponse<M::Response, Metadata>, CallError> {
        let response = M::Response::decode(b"ok").map_err(CallError::Decode)?;
        self.0.answer(M::PATH, || ok_response(response))
    }

    async fn call_server_stream<M: ServerStreamMethod>(
        &self,
        _request: CallRequest<M::Request>,
    ) -> Result<impl ResponseStream<M::Response, Metadata = Metadata>, CallError> {
        self.0
            .answer(M::PATH, || ScriptedStream(Metadata::default()))
    }

    async fn call_client_stream<M: ClientStreamMethod, S: MessageStream<M::Request> + 'static>(
        &self,
        _requests: S,
        _options: CallOptions,
    ) -> Result<CallResponse<M::Response, Metadata>, CallError> {
        let response = M::Response::decode(b"ok").map_err(CallError::Decode)?;
        self.0.answer(M::PATH, || ok_response(response))
    }

    async fn call_bidi_stream<M: BidiStreamMethod, S: MessageStream<M::Request> + 'static>(
        &self,
        _requests: S,
        _options: CallOptions,
    ) -> Result<impl ResponseStream<M::Response, Metadata = Metadata>, CallError> {
        self.0
            .answer(M::PATH, || ScriptedStream(Metadata::default()))
    }
}

impl RawTransport for ScriptedTransport {
    type Metadata = Metadata;

    async fn call_raw(
        &self,
        path: MethodPath,
        _request: CallRequest<Vec<u8>>,
    ) -> Result<CallResponse<Vec<u8>, Metadata>, CallError> {
        self.0
            .answer(path.get_path(), || ok_response(b"ok".to_vec()))
    }

    async fn call_raw_server_stream(
        &self,
        path: MethodPath,
        _request: CallRequest<Vec<u8>>,
    ) -> Result<impl ResponseStream<Vec<u8>, Metadata = Metadata>, CallError> {
        self.0
            .answer(path.get_path(), || ScriptedStream(Metadata::default()))
    }

    async fn call_raw_client_stream<S: MessageStream<Vec<u8>> + 'static>(
        &self,
        path: MethodPath,
        _requests: S,
        _options: CallOptions,
    ) -> Result<CallResponse<Vec<u8>, Metadata>, CallError> {
        self.0
            .answer(path.get_path(), || ok_response(b"ok".to_vec()))
    }

    async fn call_raw_bidi_stream<S: MessageStream<Vec<u8>> + 'static>(
        &self,
        path: MethodPath,
        _requests: S,
        _options: CallOptions,
    ) -> Result<impl ResponseStream<Vec<u8>, Metadata = Metadata>, CallError> {
        self.0
            .answer(path.get_path(), || ScriptedStream(Metadata::default()))
    }
}

/// A request stream with nothing in it.
struct EmptyRequests;

impl<T: Send> MessageStream<T> for EmptyRequests {
    async fn read_message(&mut self) -> Result<Option<T>, CallError> {
        Ok(None)
    }
}

/// Every shape of call, typed and raw.
#[derive(Clone, Copy, Debug)]
enum Shape {
    RawUnary,
    RawServerStream,
    RawClientStream,
    RawBidiStream,
    TypedUnary,
    TypedServerStream,
    TypedClientStream,
    TypedBidiStream,
}

const ALL_SHAPES: [Shape; 8] = [
    Shape::RawUnary,
    Shape::RawServerStream,
    Shape::RawClientStream,
    Shape::RawBidiStream,
    Shape::TypedUnary,
    Shape::TypedServerStream,
    Shape::TypedClientStream,
    Shape::TypedBidiStream,
];

fn path() -> MethodPath {
    MethodPath::create(Ping::PATH).unwrap_or_else(|e| panic!("{e}"))
}

fn raw_request() -> CallRequest<Vec<u8>> {
    CallRequest::create(b"hi".to_vec(), CallOptions::default())
}

fn typed_request() -> CallRequest<Bytes> {
    CallRequest::create(Bytes(b"hi".to_vec()), CallOptions::default())
}

/// Make one call of `shape` and report only whether it was admitted and succeeded.
async fn invoke<C: Transport + RawTransport>(client: &C, shape: Shape) -> Result<(), CallError> {
    match shape {
        Shape::RawUnary => client.call_raw(path(), raw_request()).await.map(|_| ()),
        Shape::RawServerStream => client
            .call_raw_server_stream(path(), raw_request())
            .await
            .map(|_| ()),
        Shape::RawClientStream => client
            .call_raw_client_stream(path(), EmptyRequests, CallOptions::default())
            .await
            .map(|_| ()),
        Shape::RawBidiStream => client
            .call_raw_bidi_stream(path(), EmptyRequests, CallOptions::default())
            .await
            .map(|_| ()),
        Shape::TypedUnary => client.call::<Ping>(typed_request()).await.map(|_| ()),
        Shape::TypedServerStream => client
            .call_server_stream::<Ping>(typed_request())
            .await
            .map(|_| ()),
        Shape::TypedClientStream => client
            .call_client_stream::<Ping, _>(EmptyRequests, CallOptions::default())
            .await
            .map(|_| ()),
        Shape::TypedBidiStream => client
            .call_bidi_stream::<Ping, _>(EmptyRequests, CallOptions::default())
            .await
            .map(|_| ()),
    }
}

fn fast_cfg() -> Result<GrpcBreakerConfig, BreakerDomainError> {
    GrpcBreakerConfig::from_config(
        r#"
            failure_threshold = 3
            cool_down_seconds = 1
            half_open_probe_count = 1
        "#,
    )
}

fn build(
    script: &Arc<Script>,
    config: GrpcBreakerConfig,
) -> Result<impl Transport + RawTransport + BreakerObservable, BreakerDomainError> {
    StdGrpcBreakerFactory::new(config).build(BuildGrpcBreakerRequest {
        inner: ScriptedTransport(Arc::clone(script)),
    })
}

async fn state_of(client: &impl BreakerObservable) -> Result<BreakerState, BreakerDomainError> {
    Ok(client.state(ObserveStateRequest).await?.state)
}

fn is_open(state: BreakerState) -> bool {
    matches!(state, BreakerState::Open { .. })
}

/// @covers: GrpcBreakerConfig::default — SWE default has positive threshold.
#[test]
fn test_grpc_breaker_config_default_has_positive_failure_threshold() {
    let cfg = GrpcBreakerConfig::default();
    assert!(cfg.failure_threshold >= 1);
}

/// @covers: StdGrpcBreakerFactory::build — a closed breaker passes every shape through.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_closed_state_passes_every_shape_through_to_the_inner_client(
) -> Result<(), Box<dyn std::error::Error>> {
    for shape in ALL_SHAPES {
        let script = Script::new(Mode::Ok);
        let client = build(&script, fast_cfg()?)?;

        invoke(&client, shape).await?;

        assert_eq!(
            script.call_count(),
            1,
            "{shape:?} must reach the inner client"
        );
        assert_eq!(state_of(&client).await?, BreakerState::Closed, "{shape:?}");
    }
    Ok(())
}

/// @covers: state machine — N failures trip Open; Open short-circuits without calling inner, for every shape.
///
/// **Acceptance test gate**: tripped state returns `Unavailable` immediately without calling inner (verify via call
/// counter).
#[tokio::test(flavor = "multi_thread")]
async fn test_call_open_state_short_circuits_every_shape_without_calling_inner(
) -> Result<(), Box<dyn std::error::Error>> {
    for shape in ALL_SHAPES {
        let script = Script::new(Mode::Status(StatusCode::Unavailable));
        let client = build(&script, fast_cfg()?)?;

        for attempt in 1..=3 {
            let failure = invoke(&client, shape).await.err();
            assert!(
                matches!(
                    failure,
                    Some(CallError::Status {
                        code: StatusCode::Unavailable,
                        ..
                    })
                ),
                "{shape:?} attempt {attempt} must carry the peer's own failure, got {failure:?}"
            );
        }
        assert!(is_open(state_of(&client).await?), "{shape:?}");
        assert_eq!(
            script.call_count(),
            3,
            "{shape:?}: exactly the threshold trips it"
        );

        for _ in 0..5 {
            let refused = invoke(&client, shape).await.err();
            match refused {
                Some(CallError::Transport {
                    path,
                    code: StatusCode::Unavailable,
                    reason,
                }) => {
                    assert_eq!(path.as_ref(), Ping::PATH, "{shape:?}");
                    assert!(reason.contains("circuit open"), "{shape:?}: {reason}");
                }
                other => {
                    panic!("{shape:?}: expected a refusal from the open circuit, got {other:?}")
                }
            }
        }
        assert_eq!(
            script.call_count(),
            3,
            "{shape:?}: Open state must NOT call inner"
        );
    }
    Ok(())
}

/// @covers: state machine — full lifecycle:
/// closed → N failures → open → cool-down → half-open → success → closed.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_full_state_lifecycle_closed_open_half_open_closed(
) -> Result<(), Box<dyn std::error::Error>> {
    let script = Script::new(Mode::Status(StatusCode::Unavailable));
    let client = build(&script, fast_cfg()?)?;

    // 1) Closed → Open after 3 failures.
    assert_eq!(state_of(&client).await?, BreakerState::Closed);
    for _ in 0..3 {
        let _ = invoke(&client, Shape::RawUnary).await;
    }
    assert!(is_open(state_of(&client).await?));

    // 2) Cool-down: while in Open, reject without calling inner.
    let calls_when_opened = script.call_count();
    let _ = invoke(&client, Shape::RawUnary).await;
    assert_eq!(
        script.call_count(),
        calls_when_opened,
        "Open must short-circuit during cool-down",
    );

    // 3) Wait past cool-down (1s). Switch the script to Ok so the half-open probe succeeds.
    tokio::time::sleep(Duration::from_millis(1100)).await;
    script.set_mode(Mode::Ok);

    // 4) Open → HalfOpen probe → Closed.
    invoke(&client, Shape::RawUnary).await?;
    assert_eq!(state_of(&client).await?, BreakerState::Closed);
    Ok(())
}

/// @covers: state machine — a stream open is the probe of a half-open breaker, and a successful one closes it.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_half_open_probe_that_is_a_stream_open_closes_the_breaker_on_success(
) -> Result<(), Box<dyn std::error::Error>> {
    let script = Script::new(Mode::Status(StatusCode::Unavailable));
    let client = build(&script, fast_cfg()?)?;
    for _ in 0..3 {
        let _ = invoke(&client, Shape::RawServerStream).await;
    }
    assert!(is_open(state_of(&client).await?));

    tokio::time::sleep(Duration::from_millis(1100)).await;
    script.set_mode(Mode::Ok);
    invoke(&client, Shape::TypedBidiStream).await?;

    assert_eq!(state_of(&client).await?, BreakerState::Closed);
    Ok(())
}

/// @covers: state machine — half-open probe failure returns to Open.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_half_open_probe_failure_returns_to_open(
) -> Result<(), Box<dyn std::error::Error>> {
    let script = Script::new(Mode::Status(StatusCode::Unavailable));
    let client = build(&script, fast_cfg()?)?;

    for _ in 0..3 {
        let _ = invoke(&client, Shape::RawUnary).await;
    }
    assert!(is_open(state_of(&client).await?));

    // Wait past cool-down so the next call promotes to HalfOpen, but keep the script failing so the probe fails.
    tokio::time::sleep(Duration::from_millis(1100)).await;

    let _ = invoke(&client, Shape::RawUnary).await;
    assert!(
        is_open(state_of(&client).await?),
        "failed probe must return to Open",
    );
    Ok(())
}

/// @covers: failure_kind — the failures that are NOT the peer's fault do not trip the breaker, in any shape.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_failures_that_are_not_the_peers_fault_do_not_trip_the_breaker(
) -> Result<(), Box<dyn std::error::Error>> {
    let not_faults = [
        (
            "PermissionDenied",
            Mode::Status(StatusCode::PermissionDenied),
        ),
        ("Unauthenticated", Mode::Status(StatusCode::Unauthenticated)),
        (
            "ResourceExhausted",
            Mode::Status(StatusCode::ResourceExhausted),
        ),
        ("InvalidArgument", Mode::Status(StatusCode::InvalidArgument)),
        ("NotFound", Mode::Status(StatusCode::NotFound)),
        (
            "status DeadlineExceeded",
            Mode::Status(StatusCode::DeadlineExceeded),
        ),
        (
            "transport DeadlineExceeded",
            Mode::Transport(StatusCode::DeadlineExceeded),
        ),
        (
            "transport Cancelled",
            Mode::Transport(StatusCode::Cancelled),
        ),
        (
            "transport InvalidArgument",
            Mode::Transport(StatusCode::InvalidArgument),
        ),
        ("encode failure", Mode::Encode),
    ];
    for (name, mode) in not_faults {
        for shape in [Shape::RawUnary, Shape::TypedServerStream] {
            let script = Script::new(mode);
            let client = build(&script, fast_cfg()?)?;

            for _ in 0..10 {
                let _ = invoke(&client, shape).await;
            }

            assert_eq!(
                state_of(&client).await?,
                BreakerState::Closed,
                "{name} through {shape:?} must not count toward the breaker threshold"
            );
            assert_eq!(script.call_count(), 10, "{name} through {shape:?}");
        }
    }
    Ok(())
}

/// @covers: failure_kind — the failures that ARE the peer's fault trip the breaker.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_failures_that_are_the_peers_fault_trip_the_breaker(
) -> Result<(), Box<dyn std::error::Error>> {
    let faults = [
        ("status Unavailable", Mode::Status(StatusCode::Unavailable)),
        ("status Internal", Mode::Status(StatusCode::Internal)),
        (
            "transport Unavailable",
            Mode::Transport(StatusCode::Unavailable),
        ),
        ("transport Unknown", Mode::Transport(StatusCode::Unknown)),
        ("transport Internal", Mode::Transport(StatusCode::Internal)),
        ("decode failure", Mode::Decode),
    ];
    for (name, mode) in faults {
        let script = Script::new(mode);
        let client = build(&script, fast_cfg()?)?;

        for _ in 0..3 {
            let _ = invoke(&client, Shape::RawUnary).await;
        }

        assert!(
            is_open(state_of(&client).await?),
            "{name} must count toward the breaker threshold"
        );
    }
    Ok(())
}

/// @covers: state — Closed reset on intermittent success.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_intermittent_success_keeps_breaker_closed(
) -> Result<(), Box<dyn std::error::Error>> {
    let script = Script::new(Mode::Status(StatusCode::Unavailable));
    let client = build(&script, fast_cfg()?)?;

    // 2 failures, then 1 success: counter resets.
    let _ = invoke(&client, Shape::RawUnary).await;
    let _ = invoke(&client, Shape::RawUnary).await;
    script.set_mode(Mode::Ok);
    let _ = invoke(&client, Shape::RawUnary).await;
    assert_eq!(state_of(&client).await?, BreakerState::Closed);

    // Now 2 more failures: still Closed (was reset).
    script.set_mode(Mode::Status(StatusCode::Unavailable));
    let _ = invoke(&client, Shape::RawUnary).await;
    let _ = invoke(&client, Shape::RawUnary).await;
    assert_eq!(state_of(&client).await?, BreakerState::Closed);
    Ok(())
}

/// @covers: StdGrpcBreakerFactory::build
#[tokio::test(flavor = "multi_thread")]
async fn test_build_with_default_config_opens_at_the_default_threshold(
) -> Result<(), Box<dyn std::error::Error>> {
    let script = Script::new(Mode::Status(StatusCode::Unavailable));
    let client = build(&script, GrpcBreakerConfig::default())?;
    let default_cfg = GrpcBreakerConfig::default();

    // Drive exactly (default_cfg.failure_threshold - 1) failures: breaker must still be closed (accepting
    // calls), proving the DEFAULT threshold — not some other value — was genuinely applied.
    for _ in 0..default_cfg.failure_threshold - 1 {
        let _ = invoke(&client, Shape::RawUnary).await;
    }
    assert_eq!(
        script.call_count(),
        default_cfg.failure_threshold - 1,
        "breaker must still be passing calls through below the default threshold"
    );

    // One more failure reaches the default threshold: breaker opens and the NEXT call must short-circuit without
    // reaching inner.
    let _ = invoke(&client, Shape::RawUnary).await;
    let calls_at_threshold = script.call_count();
    assert_eq!(calls_at_threshold, default_cfg.failure_threshold);
    let err = invoke(&client, Shape::RawUnary)
        .await
        .err()
        .ok_or("breaker must reject once open")?;
    assert!(matches!(
        err,
        CallError::Transport {
            code: StatusCode::Unavailable,
            ..
        }
    ));
    assert_eq!(
        script.call_count(),
        calls_at_threshold,
        "Open state must not call inner"
    );
    Ok(())
}

/// @covers: StdGrpcBreakerFactory::build
#[tokio::test(flavor = "multi_thread")]
async fn test_build_with_supplied_config_opens_at_the_supplied_threshold(
) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = GrpcBreakerConfig {
        failure_threshold: 3,
        cool_down_seconds: 5,
        half_open_probe_count: 1,
    };
    let script = Script::new(Mode::Status(StatusCode::Unavailable));
    let client = build(&script, cfg)?;

    // The supplied threshold is 3 — two failures must not yet trip it.
    for _ in 0..2 {
        let _ = invoke(&client, Shape::RawUnary).await;
    }
    assert_eq!(script.call_count(), 2);

    // The third failure reaches the supplied threshold and opens the breaker.
    let _ = invoke(&client, Shape::RawUnary).await;
    assert_eq!(script.call_count(), 3);
    let err = invoke(&client, Shape::RawUnary)
        .await
        .err()
        .ok_or("breaker must reject once open at the supplied threshold")?;
    assert!(matches!(
        err,
        CallError::Transport {
            code: StatusCode::Unavailable,
            ..
        }
    ));
    assert_eq!(script.call_count(), 3, "Open state must not call inner");
    Ok(())
}

/// @covers: GrpcBreakerConfig::section_name
#[test]
fn test_grpc_breaker_config_section_name_is_grpc_breaker() {
    use swe_edge_configbuilder::ConfigSection as _;
    assert_eq!(GrpcBreakerConfig::section_name(), "grpc_breaker");
}

/// @covers: StdGrpcBreakerFactory::build — after every failed call the state is the one the shared
/// state machine of `edge-transport-breaker` computes.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_state_after_each_failed_call_follows_the_shared_state_machine(
) -> Result<(), Box<dyn std::error::Error>> {
    use edge_transport_breaker::DefaultBreakerTransition;
    use edge_transport_breaker_policy::{BreakerTransition, Outcome, RecordOutcomeRequest};

    let config = fast_cfg()?;
    let script = Script::new(Mode::Status(StatusCode::Unavailable));
    let client = build(&script, config.clone())?;
    let mut state = BreakerState::Closed;
    let (mut failures, mut successes) = (0, 0);

    for call in 1..=config.failure_threshold {
        let recorded = DefaultBreakerTransition.record(RecordOutcomeRequest {
            state,
            consecutive_failures: failures,
            consecutive_successes: successes,
            config: config.clone().into(),
            outcome: Outcome::Failure,
        })?;
        state = recorded.state;
        failures = recorded.consecutive_failures;
        successes = recorded.consecutive_successes;

        let _ = invoke(&client, Shape::RawUnary).await;

        // `Open` carries the instant it opened at, which differs by microseconds between the two clocks; the
        // kind of state is what the machine decides.
        assert_eq!(
            std::mem::discriminant(&state_of(&client).await?),
            std::mem::discriminant(&state),
            "after failed call {call}"
        );
    }
    assert!(
        is_open(state),
        "the threshold of {} failures must open the circuit",
        config.failure_threshold
    );
    Ok(())
}

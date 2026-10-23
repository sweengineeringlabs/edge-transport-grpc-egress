//! Integration tests for the gRPC retry decorator against a scripted client.
//!
//! A real peer answers one way for every call, so the cases that need a different answer on each attempt (fail
//! twice, then succeed) are made with a scripted client that implements both contract traits and counts its calls.
//! The cases a real peer can make are in `retry_peer_int_test.rs`.

use grpc_egress_retry_api::proto_pattern::{
    BidiStreamMethod, CallBidiStreamRequest, CallBidiStreamResponse, CallClientStreamRequest,
    CallClientStreamResponse, CallError, CallOptions, CallRawBidiStreamRequest,
    CallRawBidiStreamResponse, CallRawClientStreamRequest, CallRawClientStreamResponse,
    CallRawRequest, CallRawRequestBuilder, CallRawResponse, CallRawServerStreamRequest,
    CallRawServerStreamResponse, CallRequest, CallResponse, CallServerStreamRequest,
    CallServerStreamResponse, ClientStreamMethod, DecodeError, EncodeError, GetHeadersRequest,
    GetHeadersResponse, GetValuesRequest, GetValuesResponse, MessageStream, Metadata,
    MetadataEntry, MetadataView, Method, MethodPath, Payload, RawTransport, ReadMessageRequest,
    ReadMessageResponse, ReadTrailersRequest, ReadTrailersResponse, ResponseStream,
    ServerStreamMethod, StatusCode, Transport, UnaryMethod,
};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use grpc_egress_retry_api::GrpcRetryConfigProvider as _;
use grpc_egress_retry_api::{BuildGrpcRetryRequest, GrpcRetryConfig, GrpcRetryFactory};
use grpc_egress_retry_spi::StdGrpcRetryFactory;

const OK_BODY: &[u8] = b"ok";
const PATH: &str = "/svc.Test/Method";

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

/// A message that cannot be encoded.
struct Unencodable;

impl Payload for Unencodable {
    const FULL_NAME: &'static str = "test.Unencodable";

    fn encode_into(&self, _out: &mut Vec<u8>) -> Result<(), EncodeError> {
        Err(EncodeError::create(Self::FULL_NAME, "cannot be encoded"))
    }

    fn estimate_encoded_len(&self) -> usize {
        0
    }

    fn decode(_bytes: &[u8]) -> Result<Self, DecodeError> {
        Ok(Unencodable)
    }
}

struct Echo;

impl Method for Echo {
    type Request = Text;
    type Response = Text;
    const PATH: &'static str = PATH;
}

impl UnaryMethod for Echo {}

struct SendUnencodable;

impl Method for SendUnencodable {
    type Request = Unencodable;
    type Response = Text;
    const PATH: &'static str = PATH;
}

impl UnaryMethod for SendUnencodable {}

#[derive(Clone, Debug)]
enum Outcome {
    Ok,
    Status(StatusCode, &'static str),
    Transport(StatusCode, &'static str),
}

/// What the scripted client saw of one attempt.
#[derive(Clone, Debug, PartialEq)]
struct Attempt {
    timeout: Option<Duration>,
    body: Vec<u8>,
    tokens: Vec<String>,
}

/// A client that answers with a script (the last entry repeats), counts its calls, and records what each attempt
/// carried. It implements both contract traits: a typed call and a raw call follow the same script.
struct ScriptedTransport {
    calls: Arc<AtomicU32>,
    attempts: Arc<Mutex<Vec<Attempt>>>,
    script: Vec<Outcome>,
    per_call: Duration,
}

/// What a test keeps of a scripted client after it has been moved into the decorator.
struct Observer {
    calls: Arc<AtomicU32>,
    attempts: Arc<Mutex<Vec<Attempt>>>,
}

impl Observer {
    fn call_count(&self) -> u32 {
        self.calls.load(Ordering::SeqCst)
    }

    fn attempts(&self) -> Vec<Attempt> {
        self.attempts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl ScriptedTransport {
    fn new(script: Vec<Outcome>) -> (Self, Observer) {
        Self::slow(script, Duration::ZERO)
    }

    /// A client whose every call takes `per_call` before it answers.
    fn slow(script: Vec<Outcome>, per_call: Duration) -> (Self, Observer) {
        let calls = Arc::new(AtomicU32::new(0));
        let attempts = Arc::new(Mutex::new(Vec::new()));
        let observer = Observer {
            calls: Arc::clone(&calls),
            attempts: Arc::clone(&attempts),
        };
        (
            Self {
                calls,
                attempts,
                script,
                per_call,
            },
            observer,
        )
    }

    async fn dispatch(&self, options: &CallOptions, body: Vec<u8>) -> Result<(), CallError> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        self.attempts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(Attempt {
                timeout: options.get_timeout(),
                body,
                tokens: options
                    .get_metadata()
                    .get_values(GetValuesRequest::new("x-token"))
                    .map(GetValuesResponse::into_values)
                    .unwrap_or_default()
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
            });
        if !self.per_call.is_zero() {
            tokio::time::sleep(self.per_call).await;
        }
        let idx = (n as usize).min(self.script.len().saturating_sub(1));
        match self.script.get(idx).cloned().unwrap_or(Outcome::Ok) {
            Outcome::Ok => Ok(()),
            Outcome::Status(code, message) => Err(CallError::Status {
                path: PATH.into(),
                code,
                message: message.to_string(),
                details: Vec::new(),
                trailers: Metadata::default(),
            }),
            Outcome::Transport(code, reason) => Err(CallError::Transport {
                path: PATH.into(),
                code,
                reason: reason.to_string(),
            }),
        }
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
    async fn read_message(
        &mut self,
        _request: ReadMessageRequest,
    ) -> Result<ReadMessageResponse<T>, CallError> {
        Ok(ReadMessageResponse::new(None))
    }
}

impl<T: Send> ResponseStream<T> for NoStream {
    type Metadata = Metadata;

    fn get_headers(
        &self,
        _request: GetHeadersRequest,
    ) -> Result<GetHeadersResponse<'_, Metadata>, CallError> {
        Ok(GetHeadersResponse::new(&self.headers))
    }

    async fn read_trailers(
        &mut self,
        _request: ReadTrailersRequest,
    ) -> Result<ReadTrailersResponse<Metadata>, CallError> {
        Ok(ReadTrailersResponse::new(Metadata::default()))
    }
}

impl Transport for ScriptedTransport {
    type Metadata = Metadata;

    async fn call<M: UnaryMethod>(
        &self,
        request: CallRequest<M::Request>,
    ) -> Result<CallResponse<M::Response, Metadata>, CallError> {
        let (message, options) = request.into_parts();
        let mut body = Vec::new();
        message.encode_into(&mut body).map_err(CallError::Encode)?;
        self.dispatch(&options, body).await?;
        let answer = M::Response::decode(OK_BODY).map_err(CallError::Decode)?;
        Ok(CallResponse::new(
            answer,
            Metadata::default(),
            Metadata::default(),
        ))
    }

    async fn call_server_stream<M: ServerStreamMethod>(
        &self,
        request: CallServerStreamRequest<M::Request>,
    ) -> Result<
        CallServerStreamResponse<impl ResponseStream<M::Response, Metadata = Metadata>>,
        CallError,
    > {
        let (message, options) = request.into_parts();
        let _request = CallRequest::new(message, options);
        { Err::<NoStream, _>(Self::refusal(M::PATH)) }.map(CallServerStreamResponse::new)
    }

    async fn call_client_stream<M: ClientStreamMethod, S: MessageStream<M::Request> + 'static>(
        &self,
        request: CallClientStreamRequest<S>,
    ) -> Result<CallClientStreamResponse<M::Response, Metadata>, CallError> {
        let (_requests, _options) = request.into_parts();
        { Err(Self::refusal(M::PATH)) }.map(CallClientStreamResponse::new)
    }

    async fn call_bidi_stream<M: BidiStreamMethod, S: MessageStream<M::Request> + 'static>(
        &self,
        request: CallBidiStreamRequest<S>,
    ) -> Result<
        CallBidiStreamResponse<impl ResponseStream<M::Response, Metadata = Metadata>>,
        CallError,
    > {
        let (_requests, _options) = request.into_parts();
        { Err::<NoStream, _>(Self::refusal(M::PATH)) }.map(CallBidiStreamResponse::new)
    }
}

impl RawTransport for ScriptedTransport {
    type Metadata = Metadata;

    async fn call_raw(
        &self,
        request: CallRawRequest,
    ) -> Result<CallRawResponse<Metadata>, CallError> {
        let (_path, message, options) = request.into_parts();
        let body = message;
        self.dispatch(&options, body).await?;
        Ok(CallRawResponse::new(CallResponse::new(
            OK_BODY.to_vec(),
            Metadata::default(),
            Metadata::default(),
        )))
    }

    async fn call_raw_server_stream(
        &self,
        request: CallRawServerStreamRequest,
    ) -> Result<
        CallRawServerStreamResponse<impl ResponseStream<Vec<u8>, Metadata = Metadata>>,
        CallError,
    > {
        let (path, message, options) = request.into_parts();
        let _request = CallRequest::new(message, options);
        { Err::<NoStream, _>(Self::refusal(path.get_path())) }.map(CallRawServerStreamResponse::new)
    }

    async fn call_raw_client_stream<S: MessageStream<Vec<u8>> + 'static>(
        &self,
        request: CallRawClientStreamRequest<S>,
    ) -> Result<CallRawClientStreamResponse<Metadata>, CallError> {
        let (path, _requests, _options) = request.into_parts();
        { Err(Self::refusal(path.get_path())) }.map(CallRawClientStreamResponse::new)
    }

    async fn call_raw_bidi_stream<S: MessageStream<Vec<u8>> + 'static>(
        &self,
        request: CallRawBidiStreamRequest<S>,
    ) -> Result<
        CallRawBidiStreamResponse<impl ResponseStream<Vec<u8>, Metadata = Metadata>>,
        CallError,
    > {
        let (path, _requests, _options) = request.into_parts();
        { Err::<NoStream, _>(Self::refusal(path.get_path())) }.map(CallRawBidiStreamResponse::new)
    }
}

fn fast_config() -> Result<GrpcRetryConfig, grpc_egress_retry_api::Error> {
    retry_provider().parse_config(
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

fn wrap(inner: ScriptedTransport, config: GrpcRetryConfig) -> impl Transport + RawTransport {
    StdGrpcRetryFactory::new(config).build(BuildGrpcRetryRequest { inner })
}

fn path() -> Result<MethodPath, Box<dyn std::error::Error>> {
    Ok(MethodPath::create(PATH)?)
}

fn raw_request(timeout: Option<Duration>) -> CallRequest<Vec<u8>> {
    CallRequest::new(
        b"hello".to_vec(),
        CallOptions::create(timeout, Metadata::default()),
    )
}

fn typed_request(text: &str, timeout: Option<Duration>) -> CallRequest<Text> {
    CallRequest::new(
        Text(text.to_string()),
        CallOptions::create(timeout, Metadata::default()),
    )
}

const FIVE_SECONDS: Option<Duration> = Some(Duration::from_secs(5));

/// @covers: GrpcRetryConfig::default â€” SWE default has positive max_attempts.
#[test]
fn test_grpc_retry_config_default_has_positive_max_attempts() {
    let cfg = GrpcRetryConfig::default();
    assert!(cfg.max_attempts() >= 1);
}

/// @covers: GrpcRetryClient â€” first-call success short-circuits.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_first_attempt_success_does_not_retry(
) -> Result<(), Box<dyn std::error::Error>> {
    let (inner, seen) = ScriptedTransport::new(vec![Outcome::Ok]);
    let client = wrap(inner, fast_config()?);
    let response = client
        .call_raw({
            let (message, options) = raw_request(FIVE_SECONDS).into_parts();
            CallRawRequestBuilder::new()
                .with_path(path()?)
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)?;
    assert_eq!(response.get_message(), OK_BODY);
    assert_eq!(seen.call_count(), 1, "no retry on success");
    Ok(())
}

/// @covers: classify(PermissionDenied) â€” NEVER retried.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_permission_denied_is_never_retried() -> Result<(), Box<dyn std::error::Error>>
{
    let (inner, seen) =
        ScriptedTransport::new(vec![Outcome::Status(StatusCode::PermissionDenied, "no")]);
    let client = wrap(inner, fast_config()?);
    let err = client
        .call_raw({
            let (message, options) = raw_request(FIVE_SECONDS).into_parts();
            CallRawRequestBuilder::new()
                .with_path(path()?)
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)
        .err()
        .ok_or("must fail")?;
    assert!(
        matches!(
            &err,
            CallError::Status {
                code: StatusCode::PermissionDenied,
                ..
            }
        ),
        "expected PermissionDenied, got {err:?}"
    );
    assert_eq!(seen.call_count(), 1, "PermissionDenied must not be retried");
    Ok(())
}

/// @covers: classify(Unauthenticated) â€” NEVER retried.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_unauthenticated_is_never_retried() -> Result<(), Box<dyn std::error::Error>>
{
    let (inner, seen) = ScriptedTransport::new(vec![Outcome::Status(
        StatusCode::Unauthenticated,
        "bad token",
    )]);
    let client = wrap(inner, fast_config()?);
    let err = client
        .call_raw({
            let (message, options) = raw_request(FIVE_SECONDS).into_parts();
            CallRawRequestBuilder::new()
                .with_path(path()?)
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)
        .err()
        .ok_or("must fail")?;
    assert!(matches!(
        err,
        CallError::Status {
            code: StatusCode::Unauthenticated,
            ..
        }
    ));
    assert_eq!(seen.call_count(), 1, "Unauthenticated must not be retried");
    Ok(())
}

/// @covers: classify(Unavailable) â€” retries until success.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_unavailable_then_ok_succeeds_after_retry(
) -> Result<(), Box<dyn std::error::Error>> {
    let (inner, seen) = ScriptedTransport::new(vec![
        Outcome::Status(StatusCode::Unavailable, "blip"),
        Outcome::Status(StatusCode::Unavailable, "blip"),
        Outcome::Ok,
    ]);
    let client = wrap(inner, fast_config()?);
    let response = client
        .call_raw({
            let (message, options) = raw_request(FIVE_SECONDS).into_parts();
            CallRawRequestBuilder::new()
                .with_path(path()?)
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)?;
    assert_eq!(response.get_message(), OK_BODY);
    assert_eq!(seen.call_count(), 3, "two retries before the Ok");
    Ok(())
}

/// @covers: classify(Unavailable) â€” exhausts attempts.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_unavailable_throughout_exhausts_max_attempts(
) -> Result<(), Box<dyn std::error::Error>> {
    let (inner, seen) =
        ScriptedTransport::new(vec![Outcome::Status(StatusCode::Unavailable, "down")]);
    let client = wrap(inner, fast_config()?);
    let err = client
        .call_raw({
            let (message, options) = raw_request(FIVE_SECONDS).into_parts();
            CallRawRequestBuilder::new()
                .with_path(path()?)
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)
        .err()
        .ok_or("never recovers")?;
    assert!(matches!(
        err,
        CallError::Status {
            code: StatusCode::Unavailable,
            ..
        }
    ));
    assert_eq!(seen.call_count(), 5, "exhausts max_attempts");
    Ok(())
}

/// @covers: classify(Internal) â€” terminal, no retry.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_internal_is_terminal() -> Result<(), Box<dyn std::error::Error>> {
    let (inner, seen) = ScriptedTransport::new(vec![Outcome::Status(StatusCode::Internal, "bug")]);
    let client = wrap(inner, fast_config()?);
    let _ = client
        .call_raw({
            let (message, options) = raw_request(FIVE_SECONDS).into_parts();
            CallRawRequestBuilder::new()
                .with_path(path()?)
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)
        .err()
        .ok_or("terminal")?;
    assert_eq!(seen.call_count(), 1, "Internal must not be retried");
    Ok(())
}

/// @covers: classify(DeadlineExceeded) â€” terminal.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_deadline_exceeded_is_terminal() -> Result<(), Box<dyn std::error::Error>> {
    let (inner, seen) =
        ScriptedTransport::new(vec![Outcome::Status(StatusCode::DeadlineExceeded, "tick")]);
    let client = wrap(inner, fast_config()?);
    let _ = client
        .call_raw({
            let (message, options) = raw_request(FIVE_SECONDS).into_parts();
            CallRawRequestBuilder::new()
                .with_path(path()?)
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)
        .err()
        .ok_or("terminal")?;
    assert_eq!(
        seen.call_count(),
        1,
        "DeadlineExceeded counts retry budget â€” not retried",
    );
    Ok(())
}

/// @covers: classify(transport Unavailable) â†’ retried; an Internal after it ends the loop.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_transport_unavailable_retries_then_internal_terminates(
) -> Result<(), Box<dyn std::error::Error>> {
    let (inner, seen) = ScriptedTransport::new(vec![
        Outcome::Transport(StatusCode::Unavailable, "transport blip"),
        Outcome::Status(StatusCode::Internal, "not retryable"),
    ]);
    let client = wrap(inner, fast_config()?);
    let _ = client
        .call_raw({
            let (message, options) = raw_request(FIVE_SECONDS).into_parts();
            CallRawRequestBuilder::new()
                .with_path(path()?)
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)
        .err()
        .ok_or("Internal is terminal")?;
    assert_eq!(seen.call_count(), 2);
    Ok(())
}

/// @covers: classify(ResourceExhausted RateLimit) â€” retried on rate-limit track.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_resource_exhausted_rate_limit_retries_on_rate_limit_track(
) -> Result<(), Box<dyn std::error::Error>> {
    let (inner, seen) = ScriptedTransport::new(vec![
        Outcome::Status(StatusCode::ResourceExhausted, "rate limit exceeded"),
        Outcome::Status(StatusCode::ResourceExhausted, "rate limit exceeded"),
        Outcome::Ok,
    ]);
    let cfg = retry_provider().parse_config(
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
    let client = wrap(inner, cfg);
    let response = client
        .call_raw({
            let (message, options) = raw_request(FIVE_SECONDS).into_parts();
            CallRawRequestBuilder::new()
                .with_path(path()?)
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)?;
    assert_eq!(response.get_message(), OK_BODY);
    assert_eq!(seen.call_count(), 3);
    Ok(())
}

/// @covers: classify(ResourceExhausted HardQuota) â€” never retried.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_resource_exhausted_hard_quota_is_terminal(
) -> Result<(), Box<dyn std::error::Error>> {
    let (inner, seen) = ScriptedTransport::new(vec![
        Outcome::Status(StatusCode::ResourceExhausted, "quota exceeded"),
        Outcome::Ok,
    ]);
    let client = wrap(inner, fast_config()?);
    let _ = client
        .call_raw({
            let (message, options) = raw_request(FIVE_SECONDS).into_parts();
            CallRawRequestBuilder::new()
                .with_path(path()?)
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)
        .err()
        .ok_or("HardQuota is terminal, must not succeed")?;
    assert_eq!(seen.call_count(), 1, "HardQuota must not retry");
    Ok(())
}

/// @covers: classify(ResourceExhausted Capacity) â€” retried on standard track.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_resource_exhausted_capacity_retries_on_standard_track(
) -> Result<(), Box<dyn std::error::Error>> {
    let (inner, seen) = ScriptedTransport::new(vec![
        Outcome::Status(StatusCode::ResourceExhausted, "server overloaded"),
        Outcome::Ok,
    ]);
    let client = wrap(inner, fast_config()?);
    let response = client
        .call_raw({
            let (message, options) = raw_request(FIVE_SECONDS).into_parts();
            CallRawRequestBuilder::new()
                .with_path(path()?)
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)?;
    assert_eq!(response.get_message(), OK_BODY);
    assert_eq!(seen.call_count(), 2);
    Ok(())
}

/// @covers: deadline budget â€” retry stops when budget exhausted.
///
/// **Acceptance test gate**: total attempts â‰¤ timeout / initial_backoff.
///
/// Each call takes ~50ms before failing with `Unavailable`. The total timeout is 200ms. Even though the config
/// allows up to 100 attempts, the retry loop must abandon when the timeout is exhausted.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_timeout_is_the_total_budget_of_all_attempts(
) -> Result<(), Box<dyn std::error::Error>> {
    let (inner, seen) = ScriptedTransport::slow(
        vec![Outcome::Status(StatusCode::Unavailable, "slow blip")],
        Duration::from_millis(50),
    );
    let cfg = retry_provider().parse_config(
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
    let client = wrap(inner, cfg);

    let total_budget = Duration::from_millis(200);
    let started = Instant::now();
    let _ = client
        .call_raw({
            let (message, options) = raw_request(Some(total_budget)).into_parts();
            CallRawRequestBuilder::new()
                .with_path(path()?)
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)
        .err()
        .ok_or("budget exhausts")?;
    let elapsed = started.elapsed();

    let calls = seen.call_count();
    assert!(
        calls < 100,
        "calls ({calls}) reached max_attempts â€” the timeout appears ignored",
    );
    let ceiling = 1 + (total_budget.as_millis() / 10) as u32;
    assert!(
        calls <= ceiling,
        "calls ({calls}) exceeded ceiling timeout / initial_backoff = {ceiling}",
    );
    assert!(
        elapsed < total_budget * 3,
        "retry loop ran for {elapsed:?}, far past budget {total_budget:?}",
    );
    Ok(())
}

/// Each attempt is given what is left of the call's timeout, never the whole of it again.
///
/// @covers: GrpcRetryClient â€” per-attempt timeout.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_each_attempt_gets_the_timeout_that_is_left(
) -> Result<(), Box<dyn std::error::Error>> {
    let (inner, seen) = ScriptedTransport::slow(
        vec![Outcome::Status(StatusCode::Unavailable, "blip")],
        Duration::from_millis(30),
    );
    let client = wrap(inner, fast_config()?);
    let budget = Duration::from_secs(5);

    let _ = client
        .call_raw({
            let (message, options) = raw_request(Some(budget)).into_parts();
            CallRawRequestBuilder::new()
                .with_path(path()?)
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response);

    let timeouts: Vec<Duration> = seen
        .attempts()
        .iter()
        .map(|a| a.timeout.unwrap_or(Duration::MAX))
        .collect();
    assert_eq!(timeouts.len(), 5, "every attempt reached the client");
    assert!(timeouts.iter().all(|t| *t <= budget), "{timeouts:?}");
    assert!(
        timeouts.windows(2).all(|w| w[1] < w[0]),
        "each attempt must get less than the one before, got {timeouts:?}"
    );
    Ok(())
}

/// A call that carries no timeout has no total budget: only `max_attempts` bounds the loop, and the attempts carry
/// no timeout either.
///
/// @covers: GrpcRetryClient â€” a call without a timeout.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_without_a_timeout_is_bounded_by_max_attempts_alone(
) -> Result<(), Box<dyn std::error::Error>> {
    let (inner, seen) =
        ScriptedTransport::new(vec![Outcome::Status(StatusCode::Unavailable, "down")]);
    let client = wrap(inner, fast_config()?);

    let _ = client
        .call_raw({
            let (message, options) = raw_request(None).into_parts();
            CallRawRequestBuilder::new()
                .with_path(path()?)
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)
        .err()
        .ok_or("never recovers")?;

    assert_eq!(seen.call_count(), 5);
    assert!(
        seen.attempts().iter().all(|a| a.timeout.is_none()),
        "no timeout was given, none must be invented"
    );
    Ok(())
}

/// The headers and the bytes of the request are the same on every attempt.
///
/// @covers: GrpcRetryClient::call_raw â€” what an attempt carries.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_every_attempt_carries_the_same_headers_and_bytes(
) -> Result<(), Box<dyn std::error::Error>> {
    let (inner, seen) = ScriptedTransport::new(vec![
        Outcome::Status(StatusCode::Unavailable, "blip"),
        Outcome::Status(StatusCode::Unavailable, "blip"),
        Outcome::Ok,
    ]);
    let client = wrap(inner, fast_config()?);
    let metadata = Metadata::create(vec![MetadataEntry::create("x-token", "abc")?]);
    let request = CallRequest::new(
        b"payload".to_vec(),
        CallOptions::create(FIVE_SECONDS, metadata),
    );

    client
        .call_raw({
            let (message, options) = request.into_parts();
            CallRawRequestBuilder::new()
                .with_path(path()?)
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)?;

    let attempts = seen.attempts();
    assert_eq!(attempts.len(), 3);
    for attempt in &attempts {
        assert_eq!(attempt.body, b"payload");
        assert_eq!(attempt.tokens, ["abc"]);
    }
    Ok(())
}

/// A typed unary call is retried, and the message of every attempt is the one the caller gave.
///
/// @covers: GrpcRetryClient::call â€” typed unary retry.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_typed_unavailable_then_ok_retries_with_the_same_message(
) -> Result<(), Box<dyn std::error::Error>> {
    let (inner, seen) = ScriptedTransport::new(vec![
        Outcome::Status(StatusCode::Unavailable, "blip"),
        Outcome::Transport(StatusCode::Unavailable, "refused"),
        Outcome::Ok,
    ]);
    let client = wrap(inner, fast_config()?);

    let response = client
        .call::<Echo>(typed_request("hello typed", FIVE_SECONDS))
        .await?;

    assert_eq!(response.get_message(), &Text("ok".to_string()));
    let attempts = seen.attempts();
    assert_eq!(attempts.len(), 3);
    assert!(
        attempts.iter().all(|a| a.body == b"hello typed"),
        "every attempt must carry the message of the caller, got {attempts:?}"
    );
    Ok(())
}

/// @covers: GrpcRetryClient::call â€” a typed failure that is terminal is not retried.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_typed_invalid_argument_is_not_retried() -> Result<(), Box<dyn std::error::Error>>
{
    let (inner, seen) =
        ScriptedTransport::new(vec![Outcome::Status(StatusCode::InvalidArgument, "bad")]);
    let client = wrap(inner, fast_config()?);

    let err = client
        .call::<Echo>(typed_request("x", FIVE_SECONDS))
        .await
        .err()
        .ok_or("must fail")?;

    assert!(matches!(
        err,
        CallError::Status {
            code: StatusCode::InvalidArgument,
            ..
        }
    ));
    assert_eq!(seen.call_count(), 1);
    Ok(())
}

/// With one attempt allowed nothing can be retried, so the call is forwarded without encoding the message.
///
/// @covers: GrpcRetryClient::call â€” one attempt.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_typed_with_one_attempt_allowed_forwards_the_call_once(
) -> Result<(), Box<dyn std::error::Error>> {
    let (inner, seen) =
        ScriptedTransport::new(vec![Outcome::Status(StatusCode::Unavailable, "down")]);
    let config = retry_provider().parse_config(
        r#"
            max_attempts = 1
            initial_backoff_ms = 1
            backoff_multiplier = 1.0
            jitter_factor = 0.0
            max_backoff_ms = 1
            rate_limit_max_attempts = 1
            rate_limit_initial_backoff_ms = 1
            rate_limit_max_backoff_ms = 1
        "#,
    )?;
    let client = wrap(inner, config);

    let err = client
        .call::<Echo>(typed_request("x", FIVE_SECONDS))
        .await
        .err()
        .ok_or("must fail")?;

    assert!(matches!(
        err,
        CallError::Status {
            code: StatusCode::Unavailable,
            ..
        }
    ));
    assert_eq!(seen.call_count(), 1, "one attempt is all that is allowed");
    Ok(())
}

/// A message that cannot be encoded is refused before any attempt: nothing reaches the client.
///
/// @covers: GrpcRetryClient::call â€” an encode failure.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_typed_message_that_cannot_be_encoded_fails_before_any_attempt(
) -> Result<(), Box<dyn std::error::Error>> {
    let (inner, seen) = ScriptedTransport::new(vec![Outcome::Ok]);
    let client = wrap(inner, fast_config()?);
    let request = CallRequest::new(
        Unencodable,
        CallOptions::create(FIVE_SECONDS, Metadata::default()),
    );

    let err = client
        .call::<SendUnencodable>(request)
        .await
        .err()
        .ok_or("an unencodable message must fail")?;

    assert!(matches!(err, CallError::Encode(_)), "got {err:?}");
    assert_eq!(seen.call_count(), 0, "nothing may reach the client");
    Ok(())
}

/// @covers: GrpcRetryConfig::section_name
#[test]
fn test_grpc_retry_config_section_name_is_grpc_retry() {
    use swe_edge_configbuilder::ConfigSection as _;
    assert_eq!(GrpcRetryConfig::section_name(), "grpc_retry");
}

fn retry_provider() -> impl grpc_egress_retry_api::GrpcRetryConfigProvider {
    use grpc_egress_retry_api::GrpcRetryConfigProviderFactory;
    grpc_egress_retry_spi::StdGrpcRetryConfigProviderFactory
        .build(grpc_egress_retry_api::BuildGrpcRetryConfigProviderRequest)
}

//! The retry decorator against a real in-process gRPC peer: what the peer sees of a retried call, a call that is not
//! retried, and the three streaming shapes the decorator forwards.

use std::collections::VecDeque;
use std::error::Error;
use std::time::{Duration, Instant};

use edge_transport_retry::BackoffScheduler;
use grpc_egress_retry::{BuildGrpcRetryRequest, GrpcRetryConfig, GrpcRetryFactory};
use grpc_egress_retry_std::{GrpcRetryConfigProvider, StdGrpcRetryFactory};
use grpc_egress_test_support::{CallFixture, ClientFixture, Fixture, PeerBehaviour, PeerServer};
use grpc_egress_transport::GrpcChannelConfig;
use grpc_egress_transport_std::GrpcChannelConfigFactory;
use proto_pattern::{
    CallError, CallOptions, CallRequest, DecodeError, EncodeError, MessageStream, Method, Payload,
    RawTransport, ServerStreamMethod, StatusCode, Transport, UnaryMethod,
};

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
    const PATH: &'static str = "/svc.Test/Method";
}

impl UnaryMethod for Echo {}

struct Watch;

impl Method for Watch {
    type Request = Text;
    type Response = Text;
    const PATH: &'static str = "/svc.Test/Watch";
}

impl ServerStreamMethod for Watch {}

/// Request messages read from a list.
struct ListSource {
    messages: VecDeque<Vec<u8>>,
}

impl ListSource {
    fn of(messages: &[&[u8]]) -> Self {
        Self {
            messages: messages.iter().map(|m| m.to_vec()).collect(),
        }
    }
}

impl MessageStream<Vec<u8>> for ListSource {
    async fn read_message(&mut self) -> Result<Option<Vec<u8>>, CallError> {
        Ok(self.messages.pop_front())
    }
}

fn config(
    max_attempts: u32,
    initial_backoff_ms: u64,
    rate_limit_max_attempts: u32,
    rate_limit_max_backoff_ms: u64,
) -> Result<GrpcRetryConfig, grpc_egress_retry::Error> {
    GrpcRetryConfig::from_config(&format!(
        r#"
            max_attempts = {max_attempts}
            initial_backoff_ms = {initial_backoff_ms}
            backoff_multiplier = 1.0
            jitter_factor = 0.0
            max_backoff_ms = {initial_backoff_ms}
            rate_limit_max_attempts = {rate_limit_max_attempts}
            rate_limit_initial_backoff_ms = 1
            rate_limit_max_backoff_ms = {rate_limit_max_backoff_ms}
        "#
    ))
}

fn fast_config(max_attempts: u32) -> Result<GrpcRetryConfig, grpc_egress_retry::Error> {
    config(max_attempts, 1, 1, 1)
}

/// The engine client of `endpoint`, wrapped with retry as `config` says.
fn retrying(
    endpoint: &str,
    config: GrpcRetryConfig,
) -> Result<impl Transport + RawTransport, Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let inner = ClientFixture::from_config(&GrpcChannelConfig::new(endpoint).allow_plaintext())?;
    Ok(StdGrpcRetryFactory::new(config).build(BuildGrpcRetryRequest { inner }))
}

fn status_peer(status: &str, message: &str, retry_after_secs: Option<u64>) -> PeerBehaviour {
    PeerBehaviour::ReplyStatus {
        status: status.to_string(),
        message: Some(message.to_string()),
        retry_after_secs,
    }
}

const FIVE_SECONDS: Duration = Duration::from_secs(5);

/// @covers: GrpcRetryClient::call_raw — a status that is the caller's fault is not retried.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_invalid_argument_status_is_not_retried_and_the_peer_sees_one_request(
) -> Result<(), Box<dyn Error>> {
    let peer = PeerServer::start(status_peer("3", "bad", None)).await?;
    let client = retrying(&peer.get_endpoint(), fast_config(5)?)?;

    let err = client
        .call_raw(
            CallFixture::path("svc.Test/Method"),
            CallFixture::request(b"x", FIVE_SECONDS),
        )
        .await
        .err()
        .ok_or("must fail")?;

    assert!(
        matches!(
            &err,
            CallError::Status {
                code: StatusCode::InvalidArgument,
                ..
            }
        ),
        "got {err:?}"
    );
    assert_eq!(peer.get_recorded_requests().len(), 1);
    Ok(())
}

/// @covers: GrpcRetryClient::call_raw — an Unavailable status is retried up to max_attempts.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_unavailable_status_throughout_reaches_the_peer_max_attempts_times(
) -> Result<(), Box<dyn Error>> {
    let peer = PeerServer::start(status_peer("14", "down", None)).await?;
    let client = retrying(&peer.get_endpoint(), fast_config(3)?)?;

    let err = client
        .call_raw(
            CallFixture::path("svc.Test/Method"),
            CallFixture::request(b"x", FIVE_SECONDS),
        )
        .await
        .err()
        .ok_or("never recovers")?;

    assert!(
        matches!(
            &err,
            CallError::Status {
                code: StatusCode::Unavailable,
                ..
            }
        ),
        "got {err:?}"
    );
    assert_eq!(peer.get_recorded_requests().len(), 3);
    Ok(())
}

/// @covers: GrpcRetryClient::call_raw — a hard quota is terminal.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_hard_quota_resource_exhausted_is_terminal_and_the_peer_sees_one_request(
) -> Result<(), Box<dyn Error>> {
    let peer = PeerServer::start(status_peer("8", "quota exceeded", None)).await?;
    let client = retrying(&peer.get_endpoint(), fast_config(5)?)?;

    let err = client
        .call_raw(
            CallFixture::path("svc.Test/Method"),
            CallFixture::request(b"x", FIVE_SECONDS),
        )
        .await
        .err()
        .ok_or("must fail")?;

    assert!(matches!(
        err,
        CallError::Status {
            code: StatusCode::ResourceExhausted,
            ..
        }
    ));
    assert_eq!(peer.get_recorded_requests().len(), 1);
    Ok(())
}

/// The peer asks the caller to wait one second (`retry-after`): the pause before the retry is that long, though the
/// configured rate-limit backoff is a millisecond.
///
/// @covers: GrpcRetryClient::call_raw — the retry-after trailer.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_rate_limit_waits_for_the_retry_after_the_peer_asked(
) -> Result<(), Box<dyn Error>> {
    let peer = PeerServer::start(status_peer("8", "rate limit exceeded", Some(1))).await?;
    let client = retrying(&peer.get_endpoint(), config(5, 1, 1, 5000)?)?;

    let started = Instant::now();
    let err = client
        .call_raw(
            CallFixture::path("svc.Test/Method"),
            CallFixture::request(b"x", Duration::from_secs(10)),
        )
        .await
        .err()
        .ok_or("the peer limits every call")?;
    let elapsed = started.elapsed();

    assert!(matches!(
        err,
        CallError::Status {
            code: StatusCode::ResourceExhausted,
            ..
        }
    ));
    assert_eq!(peer.get_recorded_requests().len(), 2, "one retry");
    assert!(
        elapsed >= Duration::from_millis(900),
        "the peer asked for 1 s, the retry came after {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_secs(4),
        "waited {elapsed:?}, far more than the 1 s asked"
    );
    Ok(())
}

/// The control of the test above: with no `retry-after` the pause is the configured backoff, a few milliseconds.
///
/// @covers: GrpcRetryClient::call_raw — a rate limit with no hint.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_rate_limit_without_a_hint_waits_only_the_configured_backoff(
) -> Result<(), Box<dyn Error>> {
    let peer = PeerServer::start(status_peer("8", "rate limit exceeded", None)).await?;
    let client = retrying(&peer.get_endpoint(), config(5, 1, 1, 5000)?)?;

    let started = Instant::now();
    let _ = client
        .call_raw(
            CallFixture::path("svc.Test/Method"),
            CallFixture::request(b"x", Duration::from_secs(10)),
        )
        .await
        .err()
        .ok_or("the peer limits every call")?;
    let elapsed = started.elapsed();

    assert_eq!(peer.get_recorded_requests().len(), 2, "one retry");
    assert!(
        elapsed < Duration::from_millis(600),
        "no hint was given, yet the retry came after {elapsed:?}"
    );
    Ok(())
}

/// The next backoff does not fit in the call's timeout, so the decorator gives up after the first attempt.
///
/// @covers: GrpcRetryClient::call_raw — the timeout is the total budget.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_timeout_shorter_than_the_next_backoff_stops_after_one_attempt(
) -> Result<(), Box<dyn Error>> {
    let peer = PeerServer::start(status_peer("14", "down", None)).await?;
    let client = retrying(&peer.get_endpoint(), config(100, 300, 1, 1)?)?;

    let started = Instant::now();
    let err = client
        .call_raw(
            CallFixture::path("svc.Test/Method"),
            CallFixture::request(b"x", Duration::from_millis(200)),
        )
        .await
        .err()
        .ok_or("the peer is down")?;
    let elapsed = started.elapsed();

    assert!(matches!(
        err,
        CallError::Status {
            code: StatusCode::Unavailable,
            ..
        }
    ));
    assert_eq!(peer.get_recorded_requests().len(), 1, "no backoff fits");
    assert!(
        elapsed < Duration::from_millis(300),
        "gave up after {elapsed:?}, it must not sleep a backoff that cannot fit"
    );
    Ok(())
}

/// A refused connection is retried, and the pause between attempts is the one the shared backoff scheduler of
/// `edge-transport-retry` computes for the policy.
///
/// @covers: StdGrpcRetryFactory::build — the pause between attempts.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_connection_refused_pauses_at_least_the_backoff_of_the_shared_scheduler(
) -> Result<(), Box<dyn Error>> {
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
    let closed = Fixture::reserve_closed_port()?;

    // What one attempt costs on this machine: a refused connection is not instant everywhere (loopback takes about
    // two seconds on Windows), so the upper bound is relative to it and not a number.
    let single_attempt = {
        let client = retrying(&closed.get_endpoint(), fast_config(1)?)?;
        let started = Instant::now();
        let _ = client
            .call_raw(
                CallFixture::path("svc.Test/Method"),
                CallFixture::request(b"x", FIVE_SECONDS),
            )
            .await;
        started.elapsed()
    };
    let client = retrying(&closed.get_endpoint(), cfg)?;

    let started = Instant::now();
    let err = client
        .call_raw(
            CallFixture::path("svc.Test/Method"),
            CallFixture::request(b"x", FIVE_SECONDS),
        )
        .await
        .err()
        .ok_or("nothing listens")?;
    let elapsed = started.elapsed();

    assert!(
        matches!(&err, CallError::Transport { .. }),
        "a refused connection is a transport failure, got {err:?}"
    );
    assert!(
        elapsed >= expected,
        "paused {elapsed:?}, the scheduler says at least {expected:?}"
    );
    let ceiling = single_attempt * 3 + expected + Duration::from_millis(1500);
    assert!(
        elapsed < ceiling,
        "took {elapsed:?} for 3 attempts of {single_attempt:?} and pauses of {expected:?}: more than {ceiling:?}"
    );
    Ok(())
}

/// A typed call is retried and the peer receives the encoded message of the caller on every attempt.
///
/// @covers: GrpcRetryClient::call — the message of each attempt.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_typed_unavailable_status_throughout_sends_the_encoded_message_on_every_attempt(
) -> Result<(), Box<dyn Error>> {
    let peer = PeerServer::start(status_peer("14", "down", None)).await?;
    let client = retrying(&peer.get_endpoint(), fast_config(3)?)?;
    let request = CallRequest::create(
        Text("typed payload".to_string()),
        CallOptions::create(Some(FIVE_SECONDS), proto_pattern::Metadata::default()),
    );

    let err = client
        .call::<Echo>(request)
        .await
        .err()
        .ok_or("never recovers")?;

    assert!(matches!(
        err,
        CallError::Status {
            code: StatusCode::Unavailable,
            ..
        }
    ));
    let requests = peer.get_recorded_requests();
    assert_eq!(requests.len(), 3);
    for request in &requests {
        assert_eq!(request.get_path(), "/svc.Test/Method");
        let frame = request.get_frames().first().ok_or("no message")?;
        assert_eq!(frame.get_payload(), b"typed payload");
    }
    Ok(())
}

/// @covers: GrpcRetryClient::call — a typed call that succeeds is answered once, decoded.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_typed_answered_once_returns_the_decoded_message() -> Result<(), Box<dyn Error>> {
    let peer = PeerServer::start(PeerBehaviour::Reply(b"typed answer".to_vec())).await?;
    let client = retrying(&peer.get_endpoint(), fast_config(3)?)?;
    let request = CallRequest::create(
        Text("question".to_string()),
        CallOptions::create(Some(FIVE_SECONDS), proto_pattern::Metadata::default()),
    );

    let response = client.call::<Echo>(request).await?;

    assert_eq!(response.get_message(), &Text("typed answer".to_string()));
    assert_eq!(peer.get_recorded_requests().len(), 1);
    Ok(())
}

/// @covers: GrpcRetryClient::call_raw_server_stream — forwarded to the inner client.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_server_stream_forwards_and_returns_the_peers_message(
) -> Result<(), Box<dyn Error>> {
    let peer = PeerServer::start(PeerBehaviour::Reply(b"one".to_vec())).await?;
    let client = retrying(&peer.get_endpoint(), fast_config(3)?)?;

    let mut stream = client
        .call_raw_server_stream(
            CallFixture::path("svc.Test/Watch"),
            CallFixture::request(b"x", FIVE_SECONDS),
        )
        .await?;

    assert_eq!(stream.read_message().await?, Some(b"one".to_vec()));
    assert_eq!(stream.read_message().await?, None);
    assert_eq!(peer.get_recorded_requests().len(), 1);
    Ok(())
}

/// @covers: GrpcRetryClient::call_server_stream — forwarded to the inner client, typed.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_server_stream_typed_forwards_and_returns_the_decoded_message(
) -> Result<(), Box<dyn Error>> {
    let peer = PeerServer::start(PeerBehaviour::Reply(b"typed one".to_vec())).await?;
    let client = retrying(&peer.get_endpoint(), fast_config(3)?)?;
    let request = CallRequest::create(
        Text("x".to_string()),
        CallOptions::create(Some(FIVE_SECONDS), proto_pattern::Metadata::default()),
    );

    let mut stream = client.call_server_stream::<Watch>(request).await?;

    assert_eq!(
        stream.read_message().await?,
        Some(Text("typed one".to_string()))
    );
    assert_eq!(stream.read_message().await?, None);
    Ok(())
}

/// @covers: GrpcRetryClient::call_raw_client_stream — forwarded to the inner client.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_client_stream_forwards_every_request_message_and_returns_the_answer(
) -> Result<(), Box<dyn Error>> {
    let peer = PeerServer::start(PeerBehaviour::Reply(b"joined".to_vec())).await?;
    let client = retrying(&peer.get_endpoint(), fast_config(3)?)?;

    let response = client
        .call_raw_client_stream(
            CallFixture::path("svc.Test/Join"),
            ListSource::of(&[b"a", b"b"]),
            CallOptions::create(Some(FIVE_SECONDS), proto_pattern::Metadata::default()),
        )
        .await?;

    assert_eq!(response.get_message(), b"joined");
    let requests = peer.get_recorded_requests();
    assert_eq!(requests.len(), 1);
    let payloads: Vec<&[u8]> = requests[0]
        .get_frames()
        .iter()
        .map(|f| f.get_payload())
        .collect();
    assert_eq!(payloads, [&b"a"[..], &b"b"[..]]);
    Ok(())
}

/// @covers: GrpcRetryClient::call_raw_bidi_stream — forwarded to the inner client.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_bidi_stream_forwards_and_returns_the_peers_echo(
) -> Result<(), Box<dyn Error>> {
    let peer = PeerServer::start(PeerBehaviour::Echo).await?;
    let client = retrying(&peer.get_endpoint(), fast_config(3)?)?;

    let mut stream = client
        .call_raw_bidi_stream(
            CallFixture::path("svc.Test/Chat"),
            ListSource::of(&[b"one", b"two"]),
            CallOptions::create(Some(FIVE_SECONDS), proto_pattern::Metadata::default()),
        )
        .await?;

    assert_eq!(stream.read_message().await?, Some(b"one".to_vec()));
    assert_eq!(stream.read_message().await?, Some(b"two".to_vec()));
    assert_eq!(stream.read_message().await?, None);
    Ok(())
}

/// A stream cannot be re-sent, so a stream that fails with a retryable status is not retried: the peer sees one request.
///
/// @covers: GrpcRetryClient::call_raw_server_stream — streams are not retried.
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_server_stream_unavailable_status_is_not_retried(
) -> Result<(), Box<dyn Error>> {
    let peer = PeerServer::start(status_peer("14", "down", None)).await?;
    let client = retrying(&peer.get_endpoint(), fast_config(5)?)?;

    let failure = match client
        .call_raw_server_stream(
            CallFixture::path("svc.Test/Watch"),
            CallFixture::request(b"x", FIVE_SECONDS),
        )
        .await
    {
        Err(error) => error,
        Ok(mut stream) => stream
            .read_message()
            .await
            .err()
            .ok_or("the peer answers every call with a failure")?,
    };

    assert!(matches!(
        failure,
        CallError::Status {
            code: StatusCode::Unavailable,
            ..
        }
    ));
    assert_eq!(
        peer.get_recorded_requests().len(),
        1,
        "a stream is not retried"
    );
    Ok(())
}

//! What a client sends and reads back, against a real `tonic` server (`tonic_peer`): metadata with several values
//! under one key and binary entries (issue 18 task 5), the status of every failure the old client reported as a
//! string (task 6), the cancellation of a call by dropping it and by a token, and the end of a stream by a token.
//!
//! The client of before kept one text value per key in a `HashMap<String, String>`: two values under one key, and a
//! binary (`-bin`) entry, could not survive a call; the tests of section A fail with that type.

mod tonic_peer;

use grpc_egress_transport::GrpcChannelConfigBuilder as _;
use std::error::Error;
use std::sync::Arc;
use std::time::{Duration, Instant};

use grpc_egress_test_support::{CallFixture, ClosedPort, Fixture};
use grpc_egress_transport::{
    BuildGrpcCancellationRequest, BuildGrpcEgressBuilderRequest, GrpcCancellationFactory,
    GrpcEgressBuilder, GrpcEgressBuilderFactory,
};
use grpc_egress_transport_std::{
    StdGrpcCancellationFactory, TokenCancellation, TonicGrpcEgressBuilderSpiFactory,
};
use proto_pattern::{
    BinaryMetadataEntry, CallError, CallOptions, CallRequest, MessageStream, Metadata,
    MetadataEntry, MetadataView, Method, Payload, RawTransport, ServerStreamMethod, StatusCode,
    Transport, UnaryMethod,
};
use proto_test_peer::{Count, Say};
use tokio_util::sync::CancellationToken;

use tonic_peer::TonicPeer;

const SHORT: Duration = Duration::from_secs(5);

/// `/meta.Peer/Reflect`, typed.
struct Reflect;

impl Method for Reflect {
    type Request = Say;
    type Response = Say;
    const PATH: &'static str = "/meta.Peer/Reflect";
}

impl UnaryMethod for Reflect {}

/// `/meta.Peer/Tick`, typed: the peer streams 1 and 2 and then stays silent.
struct Tick;

impl Method for Tick {
    type Request = Say;
    type Response = Count;
    const PATH: &'static str = "/meta.Peer/Tick";
}

impl ServerStreamMethod for Tick {}

/// What the tests send and read.
struct Fixtures;

impl Fixtures {
    fn client(peer: &TonicPeer) -> Result<impl Transport + RawTransport, Box<dyn Error>> {
        Fixture::ensure_tls_provider();
        Ok(TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
            .build(BuildGrpcEgressBuilderRequest)
            .with_plaintext_allowed()
            .build()?)
    }

    fn say(say: &Say) -> Vec<u8> {
        let mut bytes = Vec::new();
        if let Err(e) = say.encode_into(&mut bytes) {
            panic!("a Say always encodes: {e}");
        }
        bytes
    }

    fn entry(key: &str, value: &str) -> Result<MetadataEntry, Box<dyn Error>> {
        Ok(MetadataEntry::create(key, value)?)
    }

    /// The text and the binary values of every header of `metadata` under `key`.
    fn binary_values<M: MetadataView>(metadata: &M, key: &str) -> Vec<Vec<u8>> {
        metadata
            .get_binary_entries()
            .flatten()
            .filter(|e| e.get_key() == key)
            .map(|e| e.get_value().to_vec())
            .collect()
    }
}

// ---------------------------------------------------------------- A. metadata

/// @covers: Metadata
#[tokio::test]
async fn test_call_raw_two_values_under_one_key_reach_the_peer_as_two_values(
) -> Result<(), Box<dyn Error>> {
    let peer = TonicPeer::start().await;
    let client = Fixtures::client(&peer)?;
    let metadata = Metadata::create(vec![
        Fixtures::entry("x-multi", "one")?,
        Fixtures::entry("x-multi", "two")?,
    ]);

    client
        .call_raw(
            CallFixture::path("meta.Peer/Reflect"),
            CallFixture::request_with(&Fixtures::say(&Say::of("hi")), SHORT, metadata),
        )
        .await?;

    let seen = peer.get_seen_requests();
    let request = seen.first().ok_or("the peer saw no request")?;
    assert_eq!(request.get_values("x-multi"), ["one", "two"]);
    Ok(())
}

/// @covers: Metadata
#[tokio::test]
async fn test_call_typed_two_values_under_one_key_reach_the_peer_as_two_values(
) -> Result<(), Box<dyn Error>> {
    let peer = TonicPeer::start().await;
    let client = Fixtures::client(&peer)?;
    let metadata = Metadata::create(vec![
        Fixtures::entry("x-multi", "one")?,
        Fixtures::entry("x-multi", "two")?,
    ]);

    client
        .call::<Reflect>(CallRequest::create(
            Say::of("hi"),
            CallOptions::create(Some(SHORT), metadata),
        ))
        .await?;

    let seen = peer.get_seen_requests();
    let request = seen.first().ok_or("the peer saw no request")?;
    assert_eq!(request.get_values("x-multi"), ["one", "two"]);
    Ok(())
}

/// @covers: Metadata
#[tokio::test]
async fn test_call_raw_binary_entries_of_every_padding_length_reach_the_peer_as_their_bytes(
) -> Result<(), Box<dyn Error>> {
    let peer = TonicPeer::start().await;
    let client = Fixtures::client(&peer)?;
    // Not UTF-8, and of the four lengths whose base64 needs 0, 1 or 2 padding characters.
    let full = [0xffu8, 0x00, 0xfe, 0x01];
    for length in 1..=full.len() {
        let bytes = &full[..length];
        let metadata = Metadata::create_with_binary(
            Vec::new(),
            vec![BinaryMetadataEntry::create("x-blob-bin", bytes)?],
        );

        client
            .call_raw(
                CallFixture::path("meta.Peer/Reflect"),
                CallFixture::request_with(&Fixtures::say(&Say::of("hi")), SHORT, metadata),
            )
            .await?;

        let seen = peer.get_seen_requests();
        let request = seen.last().ok_or("the peer saw no request")?;
        assert_eq!(
            request.get_binary_values("x-blob-bin"),
            [bytes],
            "{length} bytes"
        );
    }
    Ok(())
}

/// @covers: Metadata
#[tokio::test]
async fn test_call_raw_response_headers_keep_two_values_under_one_key() -> Result<(), Box<dyn Error>>
{
    let peer = TonicPeer::start().await;
    let client = Fixtures::client(&peer)?;

    let response = client
        .call_raw(
            CallFixture::path("meta.Peer/Reflect"),
            CallFixture::request(&Fixtures::say(&Say::of("hi")), SHORT),
        )
        .await?;

    assert_eq!(
        response.get_headers().get_values("x-multi"),
        ["first", "second"]
    );
    Ok(())
}

/// @covers: Metadata
#[tokio::test]
async fn test_call_raw_response_binary_headers_arrive_as_their_bytes() -> Result<(), Box<dyn Error>>
{
    let peer = TonicPeer::start().await;
    let client = Fixtures::client(&peer)?;

    let response = client
        .call_raw(
            CallFixture::path("meta.Peer/Reflect"),
            CallFixture::request(&Fixtures::say(&Say::of("hi")), SHORT),
        )
        .await?;

    assert_eq!(
        Fixtures::binary_values(response.get_headers(), "x-blob-bin"),
        [vec![0xff, 0x00, 0xfe, 0x01], vec![0x80]]
    );
    Ok(())
}

/// @covers: Metadata
#[tokio::test]
async fn test_call_raw_error_status_trailers_keep_two_values_a_binary_entry_and_the_details(
) -> Result<(), Box<dyn Error>> {
    let peer = TonicPeer::start().await;
    let client = Fixtures::client(&peer)?;

    let error = client
        .call_raw(
            CallFixture::path("meta.Peer/Fail"),
            CallFixture::request(&Fixtures::say(&Say::failing(10)), SHORT),
        )
        .await
        .err()
        .ok_or("the peer fails the call")?;

    match error {
        CallError::Status {
            code,
            message,
            details,
            trailers,
            ..
        } => {
            assert_eq!(code, StatusCode::Aborted);
            assert_eq!(message, "the peer says no");
            assert_eq!(details, vec![9, 8, 7]);
            assert_eq!(trailers.get_values("x-multi"), ["first", "second"]);
            assert_eq!(
                Fixtures::binary_values(&trailers, "x-blob-bin"),
                [vec![0xff, 0x00, 0xfe, 0x01], vec![0x80]]
            );
            Ok(())
        }
        other => Err(format!("expected a status error, got {other:?}").into()),
    }
}

// ---------------------------------------------------------------- B. status, one test per old error variant

/// Old `ConnectionFailed` and `Unavailable`.
///
/// @covers: CallError::Transport
#[tokio::test]
async fn test_call_raw_to_a_refused_connection_returns_transport_unavailable(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let closed = ClosedPort::reserve()?;
    let config = config_builder(closed.get_endpoint())
        .allow_plaintext()
        .build()?;
    let client = grpc_egress_test_support::ClientFixture::from_config(&config)?;

    let error = client
        .call_raw(
            CallFixture::path("meta.Peer/Reflect"),
            CallFixture::request(b"", SHORT),
        )
        .await
        .err()
        .ok_or("nothing listens on the port")?;

    assert!(
        matches!(
            &error,
            CallError::Transport { code: StatusCode::Unavailable, path, .. }
                if path.as_ref() == "/meta.Peer/Reflect"
        ),
        "{error:?}"
    );
    Ok(())
}

/// Old `Timeout`: a passed deadline is a status, whoever noticed it.
///
/// @covers: CallError::Status
#[tokio::test]
async fn test_call_raw_slower_than_its_timeout_returns_status_deadline_exceeded(
) -> Result<(), Box<dyn Error>> {
    let peer = TonicPeer::start().await;
    let client = Fixtures::client(&peer)?;

    let started = Instant::now();
    let error = client
        .call_raw(
            CallFixture::path("meta.Peer/Hold"),
            CallFixture::request(
                &Fixtures::say(&Say::slow(30_000)),
                Duration::from_millis(200),
            ),
        )
        .await
        .err()
        .ok_or("the call must pass its timeout")?;

    assert!(
        matches!(
            &error,
            CallError::Status {
                code: StatusCode::DeadlineExceeded,
                ..
            }
        ),
        "{error:?}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "the 200 ms timeout must end the call, took {:?}",
        started.elapsed()
    );
    Ok(())
}

/// Old `Cancelled`: the peer ends the call with `CANCELLED`.
///
/// @covers: CallError::Status
#[tokio::test]
async fn test_call_raw_peer_that_cancels_the_call_returns_status_cancelled(
) -> Result<(), Box<dyn Error>> {
    let peer = TonicPeer::start().await;
    let client = Fixtures::client(&peer)?;

    let error = client
        .call_raw(
            CallFixture::path("meta.Peer/Cancelled"),
            CallFixture::request(&Fixtures::say(&Say::of("x")), SHORT),
        )
        .await
        .err()
        .ok_or("the peer cancels the call")?;

    assert!(
        matches!(
            &error,
            CallError::Status { code: StatusCode::Cancelled, message, .. }
                if message == "the peer cancelled the call"
        ),
        "{error:?}"
    );
    Ok(())
}

/// Old `Status(ResourceExhausted)` for an oversize response: the engine reports `tonic`'s own code, `OutOfRange`.
///
/// @covers: CallError::Status
#[tokio::test]
async fn test_call_raw_response_over_the_decoding_limit_returns_status_out_of_range(
) -> Result<(), Box<dyn Error>> {
    let peer = TonicPeer::start().await;
    Fixture::ensure_tls_provider();
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_max_decoding_message_size(64)
        .build()?;

    let error = client
        .call_raw(
            CallFixture::path("meta.Peer/Reflect"),
            CallFixture::request(&Fixtures::say(&Say::of(&"x".repeat(200))), SHORT),
        )
        .await
        .err()
        .ok_or("a 200-byte answer must break a 64-byte limit")?;

    assert!(
        matches!(
            &error,
            CallError::Status {
                code: StatusCode::OutOfRange,
                ..
            }
        ),
        "{error:?}"
    );
    Ok(())
}

/// A request over the encoding limit never leaves the client: an encode failure, `RESOURCE_EXHAUSTED`.
///
/// @covers: CallError::Encode
#[tokio::test]
async fn test_call_raw_request_over_the_encoding_limit_fails_before_it_reaches_the_peer(
) -> Result<(), Box<dyn Error>> {
    let peer = TonicPeer::start().await;
    Fixture::ensure_tls_provider();
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_max_encoding_message_size(64)
        .build()?;

    let error = client
        .call_raw(
            CallFixture::path("meta.Peer/Reflect"),
            CallFixture::request(&[7u8; 200], SHORT),
        )
        .await
        .err()
        .ok_or("a 200-byte request must break a 64-byte limit")?;

    assert!(
        matches!(&error, CallError::Encode(e) if e.is_too_large()),
        "{error:?}"
    );
    assert_eq!(error.get_status_code(), StatusCode::ResourceExhausted);
    assert!(
        peer.get_seen_requests().is_empty(),
        "the peer must not see the request"
    );
    Ok(())
}

/// Old `Status` for each code: the code, the message, the details and the trailers of the peer pass through.
///
/// @covers: CallError::Status
#[tokio::test]
async fn test_call_raw_every_error_status_code_passes_through_with_message_details_and_trailers(
) -> Result<(), Box<dyn Error>> {
    let peer = TonicPeer::start().await;
    let client = Fixtures::client(&peer)?;
    for code in StatusCode::ALL.into_iter().filter(|c| *c != StatusCode::Ok) {
        let error = client
            .call_raw(
                CallFixture::path("meta.Peer/Fail"),
                CallFixture::request(&Fixtures::say(&Say::failing(code.get_code())), SHORT),
            )
            .await
            .err()
            .ok_or("the peer fails the call")?;

        match error {
            CallError::Status {
                code: got,
                message,
                details,
                trailers,
                ..
            } => {
                assert_eq!(got, code, "the code was not carried through");
                assert_eq!(message, "the peer says no", "{code}");
                assert_eq!(details, vec![9, 8, 7], "{code}");
                assert_eq!(
                    trailers.get_values("x-multi"),
                    ["first", "second"],
                    "{code}"
                );
            }
            other => return Err(format!("{code}: expected a status error, got {other:?}").into()),
        }
    }
    Ok(())
}

/// Old `Internal` for a response with no `grpc-status`: a peer that answers HTTP 200 with a message and ends the
/// stream with no status in the headers or the trailers. The engine reports a status error with code
/// `UNKNOWN` ("missing grpc-status" is `tonic`'s reading of it); the call does not succeed.
///
/// @covers: CallError::Status
#[tokio::test]
async fn test_call_raw_response_with_no_grpc_status_does_not_succeed() -> Result<(), Box<dyn Error>>
{
    Fixture::ensure_tls_provider();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                break;
            };
            tokio::spawn(async move {
                let io = hyper_util::rt::TokioIo::new(stream);
                let _ =
                    hyper::server::conn::http2::Builder::new(hyper_util::rt::TokioExecutor::new())
                        .serve_connection(
                            io,
                            hyper::service::service_fn(
                                |_req: http::Request<hyper::body::Incoming>| async {
                                    let mut frame = vec![0u8, 0, 0, 0, 2];
                                    frame.extend_from_slice(b"ok");
                                    http::Response::builder()
                                        .status(200)
                                        .header("content-type", "application/grpc")
                                        .body(http_body_util::Full::new(bytes::Bytes::from(frame)))
                                },
                            ),
                        )
                        .await;
            });
        }
    });
    let client = grpc_egress_test_support::ClientFixture::from_config(
        &config_builder(format!("http://{address}"))
            .allow_plaintext()
            .build()?,
    )?;

    let result = client
        .call_raw(
            CallFixture::path("pkg.Service/Method"),
            CallFixture::request(b"", SHORT),
        )
        .await;

    let error = result
        .err()
        .ok_or("a response with no status is no success")?;
    assert!(
        matches!(
            &error,
            CallError::Status {
                code: StatusCode::Unknown,
                ..
            }
        ),
        "{error:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------- C. cancellation

/// Dropping the future of a call cancels it: the peer's handler ends.
///
/// @covers: Transport::call_raw
#[tokio::test]
async fn test_call_raw_dropping_the_future_of_a_slow_call_cancels_it_at_the_peer(
) -> Result<(), Box<dyn Error>> {
    let peer = TonicPeer::start().await;
    let client = Fixtures::client(&peer)?;
    let mut call = Box::pin(client.call_raw(
        CallFixture::path("meta.Peer/Hold"),
        CallFixture::request(&Fixtures::say(&Say::slow(60_000)), Duration::from_secs(120)),
    ));

    tokio::select! {
        _ = &mut call => return Err("a 60 s call cannot have finished".into()),
        holding = peer.wait_until(SHORT, |p| p.get_holding() == 1) => {
            assert!(holding, "the peer never started the call");
        }
    }
    drop(call);

    let released = peer
        .wait_until(SHORT, |p| p.get_holding() == 0 && p.get_released() == 1)
        .await;
    assert!(
        released,
        "the peer's handler must end when the client drops the call"
    );
    Ok(())
}

/// A fired token ends a slow call at once with `Cancelled`, and the peer sees the call end.
///
/// @covers: GrpcCancellationFactory::build
#[tokio::test]
async fn test_call_raw_through_a_fired_token_fails_cancelled_long_before_the_peer_answers(
) -> Result<(), Box<dyn Error>> {
    let peer = TonicPeer::start().await;
    let client = Fixtures::client(&peer)?;
    let token = CancellationToken::new();
    let view = StdGrpcCancellationFactory::new().build(BuildGrpcCancellationRequest {
        inner: &client,
        signal: TokenCancellation::from(token.clone()),
    });
    let canceller = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        token.cancel();
    });

    let started = Instant::now();
    let error = view
        .call_raw(
            CallFixture::path("meta.Peer/Hold"),
            CallFixture::request(&Fixtures::say(&Say::slow(60_000)), Duration::from_secs(120)),
        )
        .await
        .err()
        .ok_or("the token fires first")?;
    canceller.await?;

    assert!(
        matches!(
            &error,
            CallError::Transport { code: StatusCode::Cancelled, path, .. }
                if path.as_ref() == "/meta.Peer/Hold"
        ),
        "{error:?}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "the token must end the call at once, took {:?}",
        started.elapsed()
    );
    assert!(
        peer.wait_until(SHORT, |p| p.get_released() == 1).await,
        "the peer must see the call end"
    );
    Ok(())
}

/// A token that never fires changes nothing: the call returns the peer's answer.
///
/// @covers: GrpcCancellationFactory::build
#[tokio::test]
async fn test_call_typed_through_a_token_that_is_not_fired_returns_the_peers_answer(
) -> Result<(), Box<dyn Error>> {
    let peer = TonicPeer::start().await;
    let client = Fixtures::client(&peer)?;
    let view = StdGrpcCancellationFactory::new().build(BuildGrpcCancellationRequest {
        inner: &client,
        signal: TokenCancellation::from(CancellationToken::new()),
    });

    let response = view
        .call::<Reflect>(CallRequest::create(
            Say::of("hello"),
            CallOptions::create(Some(SHORT), Metadata::default()),
        ))
        .await?;

    assert_eq!(response.get_message(), &Say::of("hello"));
    Ok(())
}

/// A fired token ends a stream in the middle: the messages read before are kept, the next read fails `Cancelled`.
///
/// @covers: GrpcCancellationFactory::build
#[tokio::test]
async fn test_call_server_stream_through_a_token_fired_mid_stream_keeps_the_messages_read_and_then_fails_cancelled(
) -> Result<(), Box<dyn Error>> {
    let peer = TonicPeer::start().await;
    let client = Fixtures::client(&peer)?;
    let token = CancellationToken::new();
    let view = StdGrpcCancellationFactory::new().build(BuildGrpcCancellationRequest {
        inner: &client,
        signal: TokenCancellation::from(token.clone()),
    });

    let mut stream = view
        .call_server_stream::<Tick>(CallRequest::create(
            Say::of("go"),
            CallOptions::create(Some(Duration::from_secs(120)), Metadata::default()),
        ))
        .await?;
    let first = stream.read_message().await?;
    let second = stream.read_message().await?;
    let canceller = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        token.cancel();
    });
    let started = Instant::now();
    let third = tokio::time::timeout(SHORT, stream.read_message())
        .await
        .map_err(|_| "the read did not end after the token fired")?;
    canceller.await?;

    assert_eq!(first, Some(Count { value: 1 }));
    assert_eq!(second, Some(Count { value: 2 }));
    assert!(
        matches!(
            &third,
            Err(CallError::Transport {
                code: StatusCode::Cancelled,
                ..
            })
        ),
        "{third:?}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "the token must end the read at once, took {:?}",
        started.elapsed()
    );
    Ok(())
}

/// A shared view: the same client serves a second call after the first ended by its token (the token ends calls of the
/// view, not the client).
///
/// @covers: GrpcCancellationFactory::build
#[tokio::test]
async fn test_call_raw_on_the_client_after_a_token_ended_a_call_through_a_view_still_works(
) -> Result<(), Box<dyn Error>> {
    let peer = TonicPeer::start().await;
    let client = Arc::new(Fixtures::client(&peer)?);
    let token = CancellationToken::new();
    token.cancel();
    let view = StdGrpcCancellationFactory::new().build(BuildGrpcCancellationRequest {
        inner: client.as_ref(),
        signal: TokenCancellation::from(token),
    });

    let ended = view
        .call_raw(
            CallFixture::path("meta.Peer/Hold"),
            CallFixture::request(&Fixtures::say(&Say::slow(60_000)), Duration::from_secs(120)),
        )
        .await;
    let answered = client
        .call_raw(
            CallFixture::path("meta.Peer/Reflect"),
            CallFixture::request(&Fixtures::say(&Say::of("again")), SHORT),
        )
        .await?;

    assert!(matches!(
        ended,
        Err(CallError::Transport {
            code: StatusCode::Cancelled,
            ..
        })
    ));
    assert_eq!(
        answered.get_message(),
        &Fixtures::say(&Say::of("again")),
        "the client, not the view, answers the second call"
    );
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

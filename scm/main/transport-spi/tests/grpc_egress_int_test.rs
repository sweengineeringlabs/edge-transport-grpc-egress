//! Integration tests of the client the builder returns, in all four call shapes, against real in-process peers:
//! the hand-written HTTP/2 `PeerServer` for the raw shapes and the answers a real server never sends, and the
//! real `tonic` test peer of `proto-svc` for the typed shapes.
//!
//! The old `GrpcEgress` tests of the default streaming methods, of `call_stream`, of `call_unary_with_context`, of
//! `health_check` as a connection test and of the value objects are not here: the API they tested is gone (see
//! `scm/docs/removed-tests.md`); the health call is tested in `grpc_egress_health_int_test`, the cancellation by a
//! token in `tonic_grpc_egress_real_peer_int_test`.

use proto_pattern::{
    CallBidiStreamRequest, CallBidiStreamResponse, CallClientStreamRequest,
    CallClientStreamResponse, CallError, CallOptions, CallRawBidiStreamRequestBuilder,
    CallRawBidiStreamResponse, CallRawClientStreamRequestBuilder, CallRawClientStreamResponse,
    CallRawRequestBuilder, CallRawResponse, CallRawServerStreamRequestBuilder,
    CallRawServerStreamResponse, CallRequest, CallServerStreamRequest, CallServerStreamResponse,
    GetValuesRequest, MessageStream, Metadata, RawTransport, ReadMessageRequest,
    ReadMessageResponse, StatusCode, Transport,
};
use std::convert::Infallible;
use std::error::Error;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use bytes::Bytes;
use grpc_egress_test_support::{CallFixture, Fixture, PeerBehaviour, PeerServer};
use grpc_egress_transport_api::{
    BuildGrpcEgressBuilderRequest, GrpcEgressBuilder, GrpcEgressBuilderFactory,
};
use grpc_egress_transport_spi::TonicGrpcEgressBuilderSpiFactory;
use http_body::Frame;
use http_body_util::{BodyExt as _, StreamBody};
use proto_test_peer::{Chat, Count, Echo, Join, ListStream, Numbers, RunningPeer, Say};

const SHORT: Duration = Duration::from_secs(5);

/// A client for the peer at `endpoint`, built the way a caller builds one.
fn client_for(endpoint: String) -> Result<impl Transport + RawTransport, Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    Ok(TonicGrpcEgressBuilderSpiFactory::new(endpoint)
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .build()?)
}

fn frame(payload: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8];
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    out.extend_from_slice(payload);
    out
}

fn options() -> CallOptions {
    CallOptions::create(Some(SHORT), Metadata::default())
}

/// Serves HTTP/2 on a port of its own and answers every request with `data` followed by `trailers`. The peer a
/// test needs when `PeerServer` cannot make the answer: its own trailers, or a body that is not a whole message.
async fn spawn_fixed_peer(
    data: Vec<u8>,
    trailers: http::HeaderMap,
) -> Result<SocketAddr, Box<dyn Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                break;
            };
            let data = Bytes::from(data.clone());
            let trailers = trailers.clone();
            tokio::spawn(async move {
                let io = hyper_util::rt::TokioIo::new(stream);
                let _ =
                    hyper::server::conn::http2::Builder::new(hyper_util::rt::TokioExecutor::new())
                        .serve_connection(
                            io,
                            hyper::service::service_fn(
                                move |req: http::Request<hyper::body::Incoming>| {
                                    let data = data.clone();
                                    let trailers = trailers.clone();
                                    async move {
                                        let _ = req.into_body().collect().await;
                                        let body = StreamBody::new(futures::stream::iter(vec![
                                            Ok::<Frame<Bytes>, Infallible>(Frame::data(data)),
                                            Ok(Frame::trailers(trailers)),
                                        ]));
                                        http::Response::builder()
                                            .status(200)
                                            .header(http::header::CONTENT_TYPE, "application/grpc")
                                            .body(body.boxed())
                                    }
                                },
                            ),
                        )
                        .await;
            });
        }
    });
    Ok(address)
}

fn ok_trailers() -> http::HeaderMap {
    let mut trailers = http::HeaderMap::new();
    trailers.insert("grpc-status", http::HeaderValue::from_static("0"));
    trailers
}

/// Every message of `stream`, in order, or the error that ended it.
async fn drain<S: MessageStream<Vec<u8>>>(stream: &mut S) -> Result<Vec<Vec<u8>>, CallError> {
    let mut messages = Vec::new();
    while let Some(message) = stream
        .read_message(ReadMessageRequest::new())
        .await
        .map(ReadMessageResponse::into_message)?
    {
        messages.push(message);
    }
    Ok(messages)
}

// ---------------------------------------------------------------- unary, raw

/// @covers: RawTransport::call_raw
#[tokio::test]
async fn test_call_raw_sends_the_request_and_returns_the_echoed_message(
) -> Result<(), Box<dyn Error>> {
    let peer = PeerServer::start(PeerBehaviour::Echo).await?;
    let client = client_for(peer.get_endpoint())?;

    let response = client
        .call_raw({
            let (message, options) = CallFixture::request(b"payload", SHORT).into_parts();
            CallRawRequestBuilder::new()
                .with_path(CallFixture::path("echo/Echo"))
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)?;

    assert_eq!(response.get_message(), b"payload");
    Ok(())
}

/// @covers: RawTransport::call_raw
#[tokio::test]
async fn test_call_raw_propagates_the_error_status_and_message_of_the_peer(
) -> Result<(), Box<dyn Error>> {
    let peer = PeerServer::start(PeerBehaviour::ReplyStatus {
        status: "13".to_string(),
        message: Some("server-side error".to_string()),
        retry_after_secs: None,
    })
    .await?;
    let client = client_for(peer.get_endpoint())?;

    let error = client
        .call_raw({
            let (message, options) = CallFixture::request(b"x", SHORT).into_parts();
            CallRawRequestBuilder::new()
                .with_path(CallFixture::path("echo/Echo"))
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)
        .err()
        .ok_or("the peer fails the call")?;

    assert!(
        matches!(
            &error,
            CallError::Status { code: StatusCode::Internal, message, .. }
                if message == "server-side error"
        ),
        "{error:?}"
    );
    Ok(())
}

/// The trailers of a successful call are the response's trailers (the old client read them from the headers).
///
/// @covers: RawTransport::call_raw
#[tokio::test]
async fn test_call_raw_returns_the_trailers_the_peer_sent_after_the_message(
) -> Result<(), Box<dyn Error>> {
    let mut trailers = ok_trailers();
    trailers.insert("x-response-id", http::HeaderValue::from_static("meta-42"));
    let address = spawn_fixed_peer(frame(b"hi"), trailers).await?;
    let client = client_for(format!("http://{address}"))?;

    let response = client
        .call_raw({
            let (message, options) = CallFixture::request(b"hi", SHORT).into_parts();
            CallRawRequestBuilder::new()
                .with_path(CallFixture::path("echo/Echo"))
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)?;

    assert_eq!(
        response.get_message(),
        b"hi",
        "the body must be the message"
    );
    assert_eq!(
        proto_pattern::MetadataView::get_values(
            response.get_trailers(),
            GetValuesRequest::new("x-response-id")
        )?
        .into_values(),
        ["meta-42"],
        "the x-response-id trailer must reach the caller as a trailer"
    );
    Ok(())
}

/// @covers: RawTransport::call_raw
#[tokio::test]
async fn test_call_raw_to_a_server_that_stalls_returns_status_deadline_exceeded_at_the_timeout(
) -> Result<(), Box<dyn Error>> {
    let peer = PeerServer::start(PeerBehaviour::Delay(Duration::from_secs(5))).await?;
    let client = client_for(peer.get_endpoint())?;

    let started = Instant::now();
    let result = client
        .call_raw({
            let (message, options) =
                CallFixture::request(b"x", Duration::from_millis(300)).into_parts();
            CallRawRequestBuilder::new()
                .with_path(CallFixture::path("echo/Echo"))
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response);

    assert!(
        matches!(
            &result,
            Err(CallError::Status {
                code: StatusCode::DeadlineExceeded,
                ..
            })
        ),
        "{:?}",
        result.as_ref().err()
    );
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "the 300 ms timeout must end the call long before the 5 s delay, took {:?}",
        started.elapsed()
    );
    Ok(())
}

// ---------------------------------------------------------------- server streaming, raw

/// @covers: RawTransport::call_raw_server_stream
#[tokio::test]
async fn test_call_raw_server_stream_returns_the_echoed_message_and_then_the_end(
) -> Result<(), Box<dyn Error>> {
    let peer = PeerServer::start(PeerBehaviour::Echo).await?;
    let client = client_for(peer.get_endpoint())?;

    let mut stream = client
        .call_raw_server_stream({
            let (message, options) = CallFixture::request(b"hello", SHORT).into_parts();
            CallRawServerStreamRequestBuilder::new()
                .with_path(CallFixture::path("svc/ServerStream"))
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawServerStreamResponse::into_stream)?;

    assert_eq!(drain(&mut stream).await?, vec![b"hello".to_vec()]);
    Ok(())
}

/// @covers: RawTransport::call_raw_server_stream
#[tokio::test]
async fn test_call_raw_server_stream_with_an_empty_request_returns_the_empty_echo(
) -> Result<(), Box<dyn Error>> {
    let peer = PeerServer::start(PeerBehaviour::Echo).await?;
    let client = client_for(peer.get_endpoint())?;

    let mut stream = client
        .call_raw_server_stream({
            let (message, options) = CallFixture::request(b"", SHORT).into_parts();
            CallRawServerStreamRequestBuilder::new()
                .with_path(CallFixture::path("svc/ServerStream"))
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawServerStreamResponse::into_stream)?;

    assert_eq!(drain(&mut stream).await?, vec![Vec::<u8>::new()]);
    Ok(())
}

/// A frame that ends before its announced length is never delivered as a message. The call opens, and the stream
/// is read as it arrives (the client of before read the whole body when the call was made and failed there).
///
/// @covers: RawTransport::call_raw_server_stream
#[tokio::test]
async fn test_call_raw_server_stream_truncated_response_delivers_no_message(
) -> Result<(), Box<dyn Error>> {
    // The frame header claims 100 bytes and 3 follow.
    let mut body = vec![0u8];
    body.extend_from_slice(&100u32.to_be_bytes());
    body.extend_from_slice(b"abc");
    let address = spawn_fixed_peer(body, ok_trailers()).await?;
    let client = client_for(format!("http://{address}"))?;

    let mut stream = client
        .call_raw_server_stream({
            let (message, options) = CallFixture::request(b"x", SHORT).into_parts();
            CallRawServerStreamRequestBuilder::new()
                .with_path(CallFixture::path("svc/ServerStream"))
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawServerStreamResponse::into_stream)?;
    let delivered = drain(&mut stream).await;

    assert!(
        !matches!(&delivered, Ok(messages) if !messages.is_empty()),
        "a truncated frame must never be delivered as a message, got {delivered:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------- client streaming, raw

/// @covers: RawTransport::call_raw_client_stream
#[tokio::test]
async fn test_call_raw_client_stream_sends_every_message_and_returns_the_one_reply(
) -> Result<(), Box<dyn Error>> {
    let peer = PeerServer::start(PeerBehaviour::Reply(b"the-reply".to_vec())).await?;
    let client = client_for(peer.get_endpoint())?;

    let response = client
        .call_raw_client_stream(
            CallRawClientStreamRequestBuilder::new()
                .with_path(CallFixture::path("svc/ClientStream"))
                .with_requests(ListStream::of(vec![
                    b"frame-1".to_vec(),
                    b"frame-2".to_vec(),
                ]))
                .with_options(options())
                .build(),
        )
        .await
        .map(CallRawClientStreamResponse::into_response)?;

    assert_eq!(response.get_message(), b"the-reply");
    let requests = peer.get_recorded_requests();
    let frames = requests
        .first()
        .ok_or("the peer saw no request")?
        .get_frames();
    assert_eq!(frames.len(), 2, "both request messages must be sent");
    assert_eq!(frames[0].get_payload(), b"frame-1");
    assert_eq!(frames[1].get_payload(), b"frame-2");
    Ok(())
}

/// A client-streaming call has one reply message; a peer that sends two broke the protocol: `UNIMPLEMENTED`.
///
/// @covers: RawTransport::call_raw_client_stream
#[tokio::test]
async fn test_call_raw_client_stream_peer_replying_with_two_messages_returns_status_unimplemented(
) -> Result<(), Box<dyn Error>> {
    let peer = PeerServer::start(PeerBehaviour::Echo).await?;
    let client = client_for(peer.get_endpoint())?;

    let result = client
        .call_raw_client_stream(
            CallRawClientStreamRequestBuilder::new()
                .with_path(CallFixture::path("svc/ClientStream"))
                .with_requests(ListStream::of(vec![
                    b"frame-1".to_vec(),
                    b"frame-2".to_vec(),
                ]))
                .with_options(options())
                .build(),
        )
        .await
        .map(CallRawClientStreamResponse::into_response);

    assert!(
        matches!(
            &result,
            Err(CallError::Status {
                code: StatusCode::Unimplemented,
                ..
            })
        ),
        "{:?}",
        result.as_ref().err()
    );
    Ok(())
}

/// @covers: RawTransport::call_raw_client_stream
#[tokio::test]
async fn test_call_raw_client_stream_single_message_returns_the_echo() -> Result<(), Box<dyn Error>>
{
    let peer = PeerServer::start(PeerBehaviour::Echo).await?;
    let client = client_for(peer.get_endpoint())?;

    let response = client
        .call_raw_client_stream(
            CallRawClientStreamRequestBuilder::new()
                .with_path(CallFixture::path("svc/ClientStream"))
                .with_requests(ListStream::of(vec![b"only-frame".to_vec()]))
                .with_options(options())
                .build(),
        )
        .await
        .map(CallRawClientStreamResponse::into_response)?;

    assert_eq!(response.get_message(), b"only-frame");
    Ok(())
}

// ---------------------------------------------------------------- bidirectional streaming, raw

/// @covers: RawTransport::call_raw_bidi_stream
#[tokio::test]
async fn test_call_raw_bidi_stream_returns_every_echoed_message_in_order(
) -> Result<(), Box<dyn Error>> {
    let peer = PeerServer::start(PeerBehaviour::Echo).await?;
    let client = client_for(peer.get_endpoint())?;

    let mut stream = client
        .call_raw_bidi_stream(
            CallRawBidiStreamRequestBuilder::new()
                .with_path(CallFixture::path("svc/Bidi"))
                .with_requests(ListStream::of(vec![
                    b"frame1".to_vec(),
                    b"frame2".to_vec(),
                    b"frame3".to_vec(),
                ]))
                .with_options(options())
                .build(),
        )
        .await
        .map(CallRawBidiStreamResponse::into_stream)?;

    assert_eq!(
        drain(&mut stream).await?,
        vec![b"frame1".to_vec(), b"frame2".to_vec(), b"frame3".to_vec()]
    );
    Ok(())
}

/// @covers: RawTransport::call_raw_bidi_stream
#[tokio::test]
async fn test_call_raw_bidi_stream_with_no_request_message_yields_no_response_message(
) -> Result<(), Box<dyn Error>> {
    let peer = PeerServer::start(PeerBehaviour::Echo).await?;
    let client = client_for(peer.get_endpoint())?;

    let mut stream = client
        .call_raw_bidi_stream(
            CallRawBidiStreamRequestBuilder::new()
                .with_path(CallFixture::path("svc/Bidi"))
                .with_requests(ListStream::<Vec<u8>>::of(Vec::new()))
                .with_options(options())
                .build(),
        )
        .await
        .map(CallRawBidiStreamResponse::into_stream)?;

    assert_eq!(
        stream
            .read_message(ReadMessageRequest::new())
            .await
            .map(ReadMessageResponse::into_message)?,
        None,
        "an empty request stream must yield an empty response stream"
    );
    Ok(())
}

// ---------------------------------------------------------------- the four shapes, typed, through the builder's client

/// The typed calls go through the same client value the builder returns (the interceptor and endpoint layers it is made
/// of forward them as typed calls), against the real `tonic` test peer.
///
/// @covers: Transport::call
#[tokio::test]
async fn test_call_typed_unary_through_the_builders_client_returns_the_peers_answer(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = client_for(peer.get_endpoint())?;

    let response = client
        .call::<Echo>(CallRequest::new(Say::of("hello"), options()))
        .await?;

    assert_eq!(response.get_message(), &Say::of("hello"));
    Ok(())
}

/// @covers: Transport::call_server_stream
#[tokio::test]
async fn test_call_typed_server_stream_through_the_builders_client_returns_every_message(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = client_for(peer.get_endpoint())?;

    let mut stream = client
        .call_server_stream::<Numbers>(CallServerStreamRequest::new(Say::of("3"), options()))
        .await
        .map(CallServerStreamResponse::into_stream)?;
    let mut seen = Vec::new();
    while let Some(count) = stream
        .read_message(ReadMessageRequest::new())
        .await
        .map(ReadMessageResponse::into_message)?
    {
        seen.push(count);
    }

    assert_eq!(
        seen,
        [Count { value: 1 }, Count { value: 2 }, Count { value: 3 }]
    );
    Ok(())
}

/// @covers: Transport::call_client_stream
#[tokio::test]
async fn test_call_typed_client_stream_through_the_builders_client_returns_the_one_answer(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = client_for(peer.get_endpoint())?;

    let response = client
        .call_client_stream::<Join, _>(CallClientStreamRequest::new(
            ListStream::of(vec![Say::of("a"), Say::of("b"), Say::of("c")]),
            options(),
        ))
        .await
        .map(CallClientStreamResponse::into_response)?;

    assert_eq!(response.get_message(), &Say::of("a,b,c"));
    Ok(())
}

/// @covers: Transport::call_bidi_stream
#[tokio::test]
async fn test_call_typed_bidi_stream_through_the_builders_client_answers_each_message_in_order(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = client_for(peer.get_endpoint())?;

    let mut stream = client
        .call_bidi_stream::<Chat, _>(CallBidiStreamRequest::new(
            ListStream::of(vec![Say::of("one"), Say::of("two")]),
            options(),
        ))
        .await
        .map(CallBidiStreamResponse::into_stream)?;
    let mut seen = Vec::new();
    while let Some(say) = stream
        .read_message(ReadMessageRequest::new())
        .await
        .map(ReadMessageResponse::into_message)?
    {
        seen.push(say);
    }

    assert_eq!(seen, [Say::of("ONE"), Say::of("TWO")]);
    Ok(())
}

//! Tests of the gRPC wire protocol of the client over the `proto-svc` engine, against a real in-process peer: how a
//! request is put on the wire, and how every kind of answer is read -- the status codes, the frames, and the
//! deadline. The peer is hand-written (`PeerServer`), so it can send answers a real server never would: malformed
//! frames, a status in the headers, a body with no status.

use proto_pattern::{
    CallError, CallRawRequestBuilder, CallRawResponse, CallRawServerStreamRequestBuilder,
    CallRawServerStreamResponse, GetValuesRequest, GetValuesResponse, MessageStream, Metadata,
    MetadataEntry, MetadataView, MethodPath, PathError, RawTransport, ReadMessageRequest,
    ReadMessageResponse, StatusCode, Transport,
};
use std::error::Error;
use std::time::Duration;

use grpc_egress_test_support::{CallFixture, Fixture, PeerBehaviour, PeerServer, RunningPeer};
use grpc_egress_transport_api::{
    BuildGrpcEgressBuilderRequest, GrpcChannelConfigError, GrpcEgressBuilder,
    GrpcEgressBuilderFactory,
};
use grpc_egress_transport_spi::TonicGrpcEgressBuilderSpiFactory;

fn frame(flag: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = vec![flag];
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    out.extend_from_slice(payload);
    out
}

async fn start(behaviour: PeerBehaviour) -> Result<RunningPeer, Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    Ok(PeerServer::start(behaviour).await?)
}

fn client_for(peer: &RunningPeer) -> Result<impl Transport + RawTransport, Box<dyn Error>> {
    Ok(TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .build()?)
}

/// The failure of a unary call to a peer that answers `status`.
async fn call_status(
    status: &str,
    message: Option<&str>,
    retry_after_secs: Option<u64>,
) -> Result<CallError, Box<dyn Error>> {
    let peer = start(PeerBehaviour::ReplyStatus {
        status: status.to_string(),
        message: message.map(str::to_string),
        retry_after_secs,
    })
    .await?;
    let client = client_for(&peer)?;
    match client
        .call_raw({
            let (message, options) =
                CallFixture::request(b"ping", Duration::from_secs(5)).into_parts();
            CallRawRequestBuilder::new()
                .with_path(CallFixture::path("pkg.Service/Method"))
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)
    {
        Err(error) => Ok(error),
        Ok(_) => Err("a non-zero grpc-status must be an error".into()),
    }
}

/// The one message of a unary call to a peer that answers the raw `body`.
async fn call_raw(body: Vec<u8>, encoding: Option<&str>) -> Result<Vec<u8>, CallError> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::ReplyRaw {
        body,
        encoding: encoding.map(str::to_string),
    })
    .await
    .map_err(|e| CallError::Transport {
        path: "".into(),
        code: StatusCode::Internal,
        reason: e.to_string(),
    })?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .build()
        .map_err(|e| CallError::Transport {
            path: "".into(),
            code: StatusCode::Internal,
            reason: e.to_string(),
        })?;
    client
        .call_raw({
            let (message, options) =
                CallFixture::request(b"ping", Duration::from_secs(5)).into_parts();
            CallRawRequestBuilder::new()
                .with_path(CallFixture::path("pkg.Service/Method"))
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)
        .map(|response| response.into_message())
}

// ---------------------------------------------------------------- status codes

/// @covers: grpc-status
#[tokio::test]
async fn test_call_raw_every_grpc_status_code_maps_to_its_status_code() -> Result<(), Box<dyn Error>>
{
    for code in StatusCode::ALL.into_iter().filter(|c| *c != StatusCode::Ok) {
        match call_status(&code.get_code().to_string(), Some("peer says no"), None).await? {
            CallError::Status {
                code: got, message, ..
            } => {
                assert_eq!(got, code, "grpc-status {code}");
                assert_eq!(message, "peer says no", "grpc-status {code}");
            }
            other => return Err(format!("grpc-status {code}: expected Status, got {other}").into()),
        }
    }
    Ok(())
}

/// @covers: grpc-status
#[tokio::test]
async fn test_call_raw_unknown_or_malformed_grpc_status_maps_to_unknown(
) -> Result<(), Box<dyn Error>> {
    for status in ["99", "-1", "abc"] {
        match call_status(status, None, None).await? {
            CallError::Status { code, .. } => {
                assert_eq!(code, StatusCode::Unknown, "grpc-status {status:?}")
            }
            other => return Err(format!("{status:?}: expected Status, got {other}").into()),
        }
    }
    Ok(())
}

/// The retry hint of a rate-limited answer is in the trailers, where a retry policy reads it; the message is the
/// peer's own. (The old client wrote the hint into the message as `[retry-after=Ns]`.)
///
/// @covers: grpc-status
#[tokio::test]
async fn test_call_raw_resource_exhausted_with_retry_after_carries_the_hint_in_the_trailers(
) -> Result<(), Box<dyn Error>> {
    match call_status("8", Some("slow down"), Some(7)).await? {
        CallError::Status {
            code: StatusCode::ResourceExhausted,
            message,
            trailers,
            ..
        } => {
            assert_eq!(message, "slow down");
            assert_eq!(
                trailers
                    .get_values(GetValuesRequest::new("retry-after"))
                    .map(GetValuesResponse::into_values)
                    .unwrap_or_default(),
                ["7"]
            );
            Ok(())
        }
        other => Err(format!("expected ResourceExhausted, got {other}").into()),
    }
}

/// @covers: grpc-status
#[tokio::test]
async fn test_call_raw_other_status_with_retry_after_leaves_the_message_alone(
) -> Result<(), Box<dyn Error>> {
    match call_status("14", Some("down"), Some(7)).await? {
        CallError::Status {
            code: StatusCode::Unavailable,
            message,
            ..
        } => {
            assert_eq!(message, "down");
            Ok(())
        }
        other => Err(format!("expected Unavailable, got {other}").into()),
    }
}

// ---------------------------------------------------------------- frames

/// A unary response must carry exactly one message; none is a protocol violation, `UNIMPLEMENTED` at the
/// client (`proto-pattern` ADR-007). The hyper client of before took none as an empty message.
///
/// @covers: frames
#[tokio::test]
async fn test_call_raw_empty_body_is_an_unimplemented_status_error() {
    assert!(matches!(
        call_raw(Vec::new(), None).await,
        Err(CallError::Status {
            code: StatusCode::Unimplemented,
            ..
        })
    ));
}

/// @covers: frames
#[tokio::test]
async fn test_call_raw_one_frame_is_its_payload() -> Result<(), Box<dyn Error>> {
    assert_eq!(call_raw(frame(0, b"hello"), None).await?, b"hello");
    Ok(())
}

/// A frame that ends before its announced length is never delivered as a message: the call fails.
///
/// @covers: frames
#[tokio::test]
async fn test_call_raw_truncated_frame_is_an_error_and_delivers_no_message() {
    let mut body = frame(0, b"hello");
    body.truncate(body.len() - 2);

    assert!(call_raw(body, None).await.is_err());
}

/// Bytes after the last complete message, too few to make a frame, are ignored: the message before them is
/// delivered intact. The hyper client of before refused the response.
///
/// @covers: frames
#[tokio::test]
async fn test_call_raw_bytes_after_the_last_frame_are_ignored() -> Result<(), Box<dyn Error>> {
    let mut body = frame(0, b"hello");
    body.extend_from_slice(&[1, 2, 3]);

    assert_eq!(call_raw(body, None).await?, b"hello");
    Ok(())
}

/// @covers: frames
#[tokio::test]
async fn test_call_raw_invalid_compressed_flag_is_an_internal_status_error() {
    assert!(matches!(
        call_raw(frame(2, b"hello"), None).await,
        Err(CallError::Status {
            code: StatusCode::Internal,
            ..
        })
    ));
}

/// @covers: frames
#[tokio::test]
async fn test_call_raw_compressed_flag_without_an_encoding_is_an_internal_status_error() {
    assert!(matches!(
        call_raw(frame(1, b"hello"), None).await,
        Err(CallError::Status {
            code: StatusCode::Internal,
            ..
        })
    ));
}

/// A unary response with two messages is a protocol violation, `UNIMPLEMENTED` at the client.
///
/// @covers: frames
#[tokio::test]
async fn test_call_raw_two_frames_are_an_unimplemented_status_error() {
    let mut body = frame(0, b"one");
    body.extend_from_slice(&frame(0, b"two"));

    assert!(matches!(
        call_raw(body, None).await,
        Err(CallError::Status {
            code: StatusCode::Unimplemented,
            ..
        })
    ));
}

/// A server-streaming call reads every frame as a message of its own: the old `call_stream` buffered them all.
///
/// @covers: frames
#[tokio::test]
async fn test_call_raw_server_stream_two_frames_are_two_messages() -> Result<(), Box<dyn Error>> {
    let mut body = frame(0, b"one");
    body.extend_from_slice(&frame(0, b"two"));
    let peer = start(PeerBehaviour::ReplyRaw {
        body,
        encoding: None,
    })
    .await?;
    let client = client_for(&peer)?;

    let mut stream = client
        .call_raw_server_stream({
            let (message, options) =
                CallFixture::request(b"in", Duration::from_secs(5)).into_parts();
            CallRawServerStreamRequestBuilder::new()
                .with_path(CallFixture::path("pkg.Service/Stream"))
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawServerStreamResponse::into_stream)?;
    let mut messages = Vec::new();
    while let Some(message) = stream
        .read_message(ReadMessageRequest::new())
        .await
        .map(ReadMessageResponse::into_message)?
    {
        messages.push(message);
    }

    assert_eq!(messages, vec![b"one".to_vec(), b"two".to_vec()]);
    Ok(())
}

// ---------------------------------------------------------------- the request on the wire

/// @covers: request headers
#[tokio::test]
async fn test_call_raw_puts_the_grpc_headers_the_path_and_the_framed_message_on_the_wire(
) -> Result<(), Box<dyn Error>> {
    let peer = start(PeerBehaviour::Reply(b"ok".to_vec())).await?;
    let client = client_for(&peer)?;
    let metadata = Metadata::create(vec![MetadataEntry::create("x-trace-id", "abc123")?]);

    client
        .call_raw({
            let (message, options) =
                CallFixture::request_with(b"ping", Duration::from_secs(5), metadata).into_parts();
            CallRawRequestBuilder::new()
                .with_path(CallFixture::path("pkg.Service/Method"))
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)?;

    let recorded = peer.get_recorded_requests();
    let first = recorded.first().ok_or("the peer saw no request")?;
    assert_eq!(first.get_path(), "/pkg.Service/Method");
    assert_eq!(first.get_header("content-type"), Some("application/grpc"));
    assert_eq!(first.get_header("te"), Some("trailers"));
    assert_eq!(first.get_header("x-trace-id"), Some("abc123"));
    let frames = first.get_frames();
    assert_eq!(frames.len(), 1);
    assert!(!frames[0].is_compressed());
    assert_eq!(frames[0].get_payload(), b"ping");
    Ok(())
}

/// The timeout of a call is sent as `grpc-timeout` (HTTP/2 section of the gRPC protocol): a number of at most
/// eight digits and a unit. The unit is the engine's choice; the value must be the timeout of the call.
///
/// @covers: request headers
#[tokio::test]
async fn test_call_raw_sends_the_timeout_of_the_call_as_a_grpc_timeout_header(
) -> Result<(), Box<dyn Error>> {
    for timeout in [Duration::from_millis(50), Duration::from_secs(5)] {
        let peer = start(PeerBehaviour::Reply(b"ok".to_vec())).await?;
        let client = client_for(&peer)?;

        client
            .call_raw({
                let (message, options) = CallFixture::request(b"ping", timeout).into_parts();
                CallRawRequestBuilder::new()
                    .with_path(CallFixture::path("pkg.Service/Method"))
                    .with_message(message)
                    .with_options(options)
                    .build()
            })
            .await
            .map(CallRawResponse::into_response)?;

        let recorded = peer.get_recorded_requests();
        let first = recorded.first().ok_or("the peer saw no request")?;
        let header = first
            .get_header("grpc-timeout")
            .ok_or("the call must carry grpc-timeout")?;
        let sent = parse_grpc_timeout(header)?;
        assert!(
            sent <= timeout && sent + Duration::from_millis(100) >= timeout,
            "grpc-timeout {header:?} is {sent:?} for a timeout of {timeout:?}"
        );
    }
    Ok(())
}

/// The value of a `grpc-timeout` header: digits and one of `H M S m u n`.
fn parse_grpc_timeout(header: &str) -> Result<Duration, Box<dyn Error>> {
    let (digits, unit) = header.split_at(header.len().saturating_sub(1));
    if digits.is_empty() || digits.len() > 8 {
        return Err(format!("{header:?} is not one to eight digits and a unit").into());
    }
    let value: u64 = digits.parse()?;
    Ok(match unit {
        "H" => Duration::from_secs(value * 3600),
        "M" => Duration::from_secs(value * 60),
        "S" => Duration::from_secs(value),
        "m" => Duration::from_millis(value),
        "u" => Duration::from_micros(value),
        "n" => Duration::from_nanos(value),
        other => return Err(format!("{other:?} is not a grpc-timeout unit").into()),
    })
}

/// A method that is no URI path cannot be named: the check is made when the path is created, so no call is made
/// (the old client sent it through and failed with a sanitized internal error).
///
/// @covers: request uri
#[test]
fn test_create_method_path_that_is_no_uri_path_is_refused_before_any_call() {
    let result = MethodPath::create("/pkg.Service/bad method");

    assert!(
        matches!(result, Err(PathError::InvalidCharacter { .. })),
        "{:?}",
        result.as_ref().err()
    );
}

// ---------------------------------------------------------------- deadline

/// @covers: deadline
#[tokio::test]
async fn test_call_raw_peer_slower_than_the_timeout_is_a_status_deadline_exceeded(
) -> Result<(), Box<dyn Error>> {
    let peer = start(PeerBehaviour::Delay(Duration::from_millis(500))).await?;
    let client = client_for(&peer)?;

    let result = client
        .call_raw({
            let (message, options) =
                CallFixture::request(b"ping", Duration::from_millis(50)).into_parts();
            CallRawRequestBuilder::new()
                .with_path(CallFixture::path("pkg.Service/Method"))
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response);

    assert!(
        matches!(
            result,
            Err(CallError::Status {
                code: StatusCode::DeadlineExceeded,
                ..
            })
        ),
        "{:?}",
        result.as_ref().err()
    );
    Ok(())
}

// ---------------------------------------------------------------- the scheme of the endpoint

/// @covers: plaintext endpoint
#[test]
fn test_build_http_scheme_in_any_case_is_a_plaintext_endpoint_and_is_refused(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    for endpoint in [
        "http://127.0.0.1:1",
        "HTTP://127.0.0.1:1",
        "Http://127.0.0.1:1",
    ] {
        let result = TonicGrpcEgressBuilderSpiFactory::new(endpoint)
            .build(BuildGrpcEgressBuilderRequest)
            .build();

        assert!(
            matches!(result, Err(GrpcChannelConfigError::PlaintextRejected(ref e)) if e == endpoint),
            "{endpoint}"
        );
    }
    Ok(())
}

/// @covers: plaintext endpoint
#[test]
fn test_build_https_scheme_is_not_a_plaintext_endpoint() {
    Fixture::ensure_tls_provider();

    let https = TonicGrpcEgressBuilderSpiFactory::new("https://service.example:443")
        .build(BuildGrpcEgressBuilderRequest)
        .build();
    let http = TonicGrpcEgressBuilderSpiFactory::new("http://service.example:80")
        .build(BuildGrpcEgressBuilderRequest)
        .build();

    assert!(https.is_ok());
    assert!(
        matches!(http, Err(GrpcChannelConfigError::PlaintextRejected(_))),
        "the same host over http must need the plaintext opt-in"
    );
}

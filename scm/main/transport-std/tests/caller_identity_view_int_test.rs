//! A client bound to a security context (`StdGrpcCallerIdentityFactory`) carries the headers derived from the
//! context on every call, typed and raw, on every shape; a header the call already carries is the call's;
//! a header that is not valid fails the call before anything is sent. Against a real `tonic` server
//! (`proto-test-peer`).

use proto_pattern::{
    CallBidiStreamRequest, CallBidiStreamResponse, CallClientStreamRequest,
    CallClientStreamResponse, CallError, CallOptions, CallRawClientStreamRequestBuilder,
    CallRawClientStreamResponse, CallRawRequestBuilder, CallRawResponse, CallRequest,
    CallServerStreamRequest, CallServerStreamResponse, MessageStream, Metadata, MetadataBuilder,
    MetadataEntry, RawTransport, ReadMessageRequest, ReadMessageResponse, StatusCode, Transport,
};
use std::error::Error;
use std::time::{Duration, Instant};

use grpc_egress_test_support::CallFixture;
use grpc_egress_transport::{
    BuildGrpcCallerIdentityRequest, BuildGrpcEgressBuilderRequest, GrpcCallerIdentityFactory,
    GrpcChannelConfigError, GrpcEgressBuilder, GrpcEgressBuilderFactory,
};
use grpc_egress_transport_std::{
    SecurityContext, SecurityContextIdentity, StdGrpcCallerIdentityFactory,
    TonicGrpcEgressBuilderSpiFactory,
};
use proto_test_peer::{Chat, Echo, Join, ListStream, Numbers, RunningPeer, Say};

struct Fixture;

impl Fixture {
    fn client(
        peer: &RunningPeer,
    ) -> Result<impl Transport + RawTransport + use<>, GrpcChannelConfigError> {
        TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
            .build(BuildGrpcEgressBuilderRequest)
            .with_plaintext_allowed()
            .build()
    }

    /// `client` bound to `ctx`.
    fn view<'a, T: Transport + RawTransport>(
        client: &'a T,
        ctx: &'a SecurityContext,
    ) -> impl Transport + RawTransport + use<'a, T> {
        StdGrpcCallerIdentityFactory::new().build(BuildGrpcCallerIdentityRequest {
            inner: client,
            identity: &SecurityContextIdentity::from(ctx),
        })
    }

    /// An authenticated context; `token` is what it holds, when it holds one.
    fn authenticated(token: Option<&str>) -> SecurityContext {
        let mut ctx = SecurityContext::unauthenticated();
        ctx.authenticated = true;
        ctx.token = token.map(str::to_string);
        ctx
    }

    fn typed(text: &str) -> CallRequest<Say> {
        CallRequest::new(Say::of(text), CallOptions::default())
    }

    fn raw(say: &Say) -> CallRequest<Vec<u8>> {
        CallRequest::new(prost::Message::encode_to_vec(say), CallOptions::default())
    }

    fn encode(text: &str) -> Vec<u8> {
        prost::Message::encode_to_vec(&Say::of(text))
    }

    fn with_header(key: &str, value: &str) -> Result<CallOptions, Box<dyn Error>> {
        Ok(CallOptions::create(
            None,
            Metadata::create(vec![MetadataEntry::create(key, value)?]),
        ))
    }
}

// ---- the headers derived from the context

#[tokio::test]
async fn test_call_typed_context_with_trace_and_tenant_sends_both_headers(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer)?;
    let ctx = SecurityContext::unauthenticated()
        .with_trace_id("trace-1")
        .with_tenant("tenant-9");

    Fixture::view(&client, &ctx)
        .call::<Echo>(Fixture::typed("hi"))
        .await?;

    assert_eq!(
        peer.get_last_header("x-trace-id").as_deref(),
        Some("trace-1")
    );
    assert_eq!(
        peer.get_last_header("x-tenant-id").as_deref(),
        Some("tenant-9")
    );
    Ok(())
}

#[tokio::test]
async fn test_call_raw_context_with_trace_and_tenant_sends_both_headers(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer)?;
    let ctx = SecurityContext::unauthenticated()
        .with_trace_id("trace-1")
        .with_tenant("tenant-9");

    Fixture::view(&client, &ctx)
        .call_raw({
            let (message, options) = Fixture::raw(&Say::of("hi")).into_parts();
            CallRawRequestBuilder::new()
                .with_path(CallFixture::path("/test.Peer/Echo"))
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)?;

    assert_eq!(
        peer.get_last_header("x-trace-id").as_deref(),
        Some("trace-1")
    );
    assert_eq!(
        peer.get_last_header("x-tenant-id").as_deref(),
        Some("tenant-9")
    );
    Ok(())
}

#[tokio::test]
async fn test_call_authenticated_context_with_a_token_sends_a_bearer_authorization(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer)?;
    let ctx = Fixture::authenticated(Some("jwt.value.here"));

    Fixture::view(&client, &ctx)
        .call::<Echo>(Fixture::typed("hi"))
        .await?;

    assert_eq!(
        peer.get_last_header("authorization").as_deref(),
        Some("Bearer jwt.value.here")
    );
    Ok(())
}

#[tokio::test]
async fn test_call_unauthenticated_context_that_holds_a_token_sends_no_authorization(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer)?;
    let mut ctx = SecurityContext::unauthenticated();
    ctx.token = Some("a-token-that-must-not-leak".to_string());

    Fixture::view(&client, &ctx)
        .call::<Echo>(Fixture::typed("hi"))
        .await?;

    assert_eq!(
        peer.get_last_header("authorization"),
        None,
        "an unauthenticated context never speaks for the caller"
    );
    Ok(())
}

#[tokio::test]
async fn test_call_authenticated_context_without_a_token_sends_no_authorization(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer)?;
    let ctx = Fixture::authenticated(None);

    Fixture::view(&client, &ctx)
        .call::<Echo>(Fixture::typed("hi"))
        .await?;

    assert_eq!(peer.get_last_header("authorization"), None);
    Ok(())
}

#[tokio::test]
async fn test_call_context_metadata_entries_reach_the_peer() -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer)?;
    let mut ctx = SecurityContext::unauthenticated();
    ctx.metadata.insert("x-extra".to_string(), "e1".to_string());
    ctx.metadata.insert("x-other".to_string(), "e2".to_string());

    Fixture::view(&client, &ctx)
        .call::<Echo>(Fixture::typed("hi"))
        .await?;

    assert_eq!(peer.get_last_header("x-extra").as_deref(), Some("e1"));
    assert_eq!(peer.get_last_header("x-other").as_deref(), Some("e2"));
    Ok(())
}

#[tokio::test]
async fn test_call_context_with_nothing_to_send_adds_no_identity_header(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer)?;
    let ctx = SecurityContext::unauthenticated();

    let response = Fixture::view(&client, &ctx)
        .call::<Echo>(Fixture::typed("hi"))
        .await?;

    assert_eq!(response.get_message(), &Say::of("hi"));
    for header in ["x-trace-id", "x-tenant-id", "authorization"] {
        assert_eq!(peer.get_last_header(header), None, "{header}");
    }
    Ok(())
}

// ---- the call's own headers, timeout and refusals

#[tokio::test]
async fn test_call_header_the_call_already_carries_is_the_one_the_peer_sees(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer)?;
    let ctx = SecurityContext::unauthenticated().with_trace_id("from-the-context");

    Fixture::view(&client, &ctx)
        .call::<Echo>(CallRequest::new(
            Say::of("hi"),
            Fixture::with_header("x-trace-id", "from-the-call")?,
        ))
        .await?;

    assert_eq!(
        peer.get_last_header("x-trace-id").as_deref(),
        Some("from-the-call")
    );
    Ok(())
}

#[tokio::test]
async fn test_call_context_view_keeps_the_timeout_of_the_call() -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer)?;
    let ctx = SecurityContext::unauthenticated().with_trace_id("t");
    let started = Instant::now();

    let result = Fixture::view(&client, &ctx)
        .call_raw(
            CallRawRequestBuilder::new()
                .with_path(CallFixture::path("/test.Peer/Echo"))
                .with_message(prost::Message::encode_to_vec(&Say::slow(3000)))
                .with_options(CallOptions::create(
                    Some(Duration::from_millis(150)),
                    Metadata::default(),
                ))
                .build(),
        )
        .await
        .map(CallRawResponse::into_response);

    let error = result
        .err()
        .ok_or("the peer answers after 3 s, past the timeout")?;
    assert_eq!(
        error.get_status_code(),
        StatusCode::DeadlineExceeded,
        "{error:?}"
    );
    assert!(started.elapsed() < Duration::from_millis(2500));
    Ok(())
}

#[tokio::test]
async fn test_call_typed_context_metadata_key_that_is_not_valid_fails_before_the_peer(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer)?;
    let mut ctx = SecurityContext::unauthenticated().with_trace_id("t");
    ctx.metadata.insert("Bad Key".to_string(), "v".to_string());

    let result = Fixture::view(&client, &ctx)
        .call::<Echo>(Fixture::typed("hi"))
        .await;

    assert!(
        matches!(
            &result,
            Err(CallError::Transport { path, code: StatusCode::InvalidArgument, .. })
                if path.as_ref() == "/test.Peer/Echo"
        ),
        "got {:?}",
        result.err()
    );
    assert_eq!(
        peer.get_reached(),
        0,
        "an identity header is never dropped silently: nothing is sent"
    );
    Ok(())
}

#[tokio::test]
async fn test_call_raw_context_metadata_key_that_is_not_valid_fails_before_the_peer(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer)?;
    let mut ctx = SecurityContext::unauthenticated();
    ctx.metadata
        .insert("grpc-reserved".to_string(), "v".to_string());

    let result = Fixture::view(&client, &ctx)
        .call_raw({
            let (message, options) = Fixture::raw(&Say::of("hi")).into_parts();
            CallRawRequestBuilder::new()
                .with_path(CallFixture::path("/test.Peer/Echo"))
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response);

    assert!(
        matches!(
            &result,
            Err(CallError::Transport {
                code: StatusCode::InvalidArgument,
                ..
            })
        ),
        "got {:?}",
        result.err()
    );
    assert_eq!(peer.get_reached(), 0);
    Ok(())
}

// ---- every shape

#[tokio::test]
async fn test_call_server_stream_typed_context_headers_reach_the_peer() -> Result<(), Box<dyn Error>>
{
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer)?;
    let ctx = SecurityContext::unauthenticated().with_trace_id("trace-stream");

    let view = Fixture::view(&client, &ctx);
    let mut stream = view
        .call_server_stream::<Numbers>({
            let (message, options) = Fixture::typed("1").into_parts();
            CallServerStreamRequest::new(message, options)
        })
        .await
        .map(CallServerStreamResponse::into_stream)?;

    assert!(stream
        .read_message(ReadMessageRequest::new())
        .await
        .map(ReadMessageResponse::into_message)?
        .is_some());
    assert_eq!(
        peer.get_last_header("x-trace-id").as_deref(),
        Some("trace-stream")
    );
    Ok(())
}

#[tokio::test]
async fn test_call_client_stream_raw_context_headers_reach_the_peer() -> Result<(), Box<dyn Error>>
{
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer)?;
    let ctx = SecurityContext::unauthenticated().with_tenant("tenant-stream");

    let response = Fixture::view(&client, &ctx)
        .call_raw_client_stream(
            CallRawClientStreamRequestBuilder::new()
                .with_path(CallFixture::path("/test.Peer/Join"))
                .with_requests(ListStream::of(vec![
                    Fixture::encode("a"),
                    Fixture::encode("b"),
                ]))
                .with_options(CallOptions::default())
                .build(),
        )
        .await
        .map(CallRawClientStreamResponse::into_response)?;

    assert_eq!(
        <Say as prost::Message>::decode(response.get_message().as_slice())?,
        Say::of("a,b")
    );
    assert_eq!(
        peer.get_last_header("x-tenant-id").as_deref(),
        Some("tenant-stream")
    );
    Ok(())
}

#[tokio::test]
async fn test_call_bidi_stream_typed_context_headers_reach_the_peer() -> Result<(), Box<dyn Error>>
{
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer)?;
    let ctx = Fixture::authenticated(Some("chat-token"));

    let view = Fixture::view(&client, &ctx);
    let mut stream = view
        .call_bidi_stream::<Chat, _>(CallBidiStreamRequest::new(
            ListStream::of(vec![Say::of("one")]),
            CallOptions::default(),
        ))
        .await
        .map(CallBidiStreamResponse::into_stream)?;

    assert_eq!(
        stream
            .read_message(ReadMessageRequest::new())
            .await
            .map(ReadMessageResponse::into_message)?,
        Some(Say::of("ONE"))
    );
    assert_eq!(
        peer.get_last_header("authorization").as_deref(),
        Some("Bearer chat-token")
    );
    Ok(())
}

#[tokio::test]
async fn test_call_client_stream_typed_context_headers_reach_the_peer() -> Result<(), Box<dyn Error>>
{
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer)?;
    let ctx = SecurityContext::unauthenticated().with_trace_id("trace-join");

    let response = Fixture::view(&client, &ctx)
        .call_client_stream::<Join, _>(CallClientStreamRequest::new(
            ListStream::of(vec![Say::of("x")]),
            CallOptions::default(),
        ))
        .await
        .map(CallClientStreamResponse::into_response)?;

    assert_eq!(response.get_message(), &Say::of("x"));
    assert_eq!(
        peer.get_last_header("x-trace-id").as_deref(),
        Some("trace-join")
    );
    Ok(())
}

// ---- views are independent

#[tokio::test]
async fn test_call_two_views_of_one_client_each_carry_their_own_context(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer)?;
    let first = SecurityContext::unauthenticated().with_tenant("tenant-a");
    let second = SecurityContext::unauthenticated().with_tenant("tenant-b");

    Fixture::view(&client, &first)
        .call::<Echo>(Fixture::typed("hi"))
        .await?;
    assert_eq!(
        peer.get_last_header("x-tenant-id").as_deref(),
        Some("tenant-a")
    );
    Fixture::view(&client, &second)
        .call::<Echo>(Fixture::typed("hi"))
        .await?;
    assert_eq!(
        peer.get_last_header("x-tenant-id").as_deref(),
        Some("tenant-b")
    );
    client.call::<Echo>(Fixture::typed("hi")).await?;
    assert_eq!(
        peer.get_last_header("x-tenant-id"),
        None,
        "the client itself carries no context"
    );
    Ok(())
}

// ---- the view knows the interface, not the security context: an identity written here works the same

/// An identity that is not a `SecurityContext`: fixed values, one extra header.
struct FixedIdentity;

impl grpc_egress_transport::CallerIdentity for FixedIdentity {
    fn get_trace_id(&self) -> Option<&str> {
        Some("custom-trace")
    }

    fn get_tenant_id(&self) -> Option<&str> {
        None
    }

    fn get_bearer_token(&self) -> Option<&str> {
        Some("custom-token")
    }

    fn list_headers(&self) -> Vec<(&str, &str)> {
        vec![("x-custom", "yes")]
    }
}

#[tokio::test]
async fn test_call_typed_custom_identity_sends_its_trace_bearer_and_extra_headers_and_no_tenant(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer)?;
    let view = StdGrpcCallerIdentityFactory::new().build(BuildGrpcCallerIdentityRequest {
        inner: &client,
        identity: &FixedIdentity,
    });

    view.call::<Echo>(Fixture::typed("hi")).await?;

    assert_eq!(
        peer.get_last_header("x-trace-id").as_deref(),
        Some("custom-trace")
    );
    assert_eq!(
        peer.get_last_header("authorization").as_deref(),
        Some("Bearer custom-token")
    );
    assert_eq!(peer.get_last_header("x-custom").as_deref(), Some("yes"));
    assert_eq!(peer.get_last_header("x-tenant-id"), None);
    Ok(())
}

// ---- the hand-written peer keeps the last value of a header it receives twice, so these see what the real one hides

/// A header the call carries is sent once, with the call's value: the identity does not add a second one after it.
#[tokio::test]
async fn test_call_raw_header_the_call_already_carries_is_not_sent_a_second_time_with_the_identitys_value(
) -> Result<(), Box<dyn Error>> {
    use grpc_egress_test_support::{PeerBehaviour, PeerServer};
    let peer = PeerServer::start(PeerBehaviour::Reply(b"ok".to_vec())).await?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .build()?;
    let ctx = SecurityContext::unauthenticated().with_trace_id("from-the-context");

    Fixture::view(&client, &ctx)
        .call_raw(
            CallRawRequestBuilder::new()
                .with_path(CallFixture::path("/test.Peer/Echo"))
                .with_message(Fixture::encode("hi"))
                .with_options(Fixture::with_header("x-trace-id", "from-the-call")?)
                .build(),
        )
        .await
        .map(CallRawResponse::into_response)?;

    let requests = peer.get_recorded_requests();
    let request = requests.first().ok_or("the peer received no request")?;
    assert_eq!(request.get_header("x-trace-id"), Some("from-the-call"));
    Ok(())
}

/// A binary header the call carries survives the addition of the identity's headers.
#[tokio::test]
async fn test_call_raw_binary_header_the_call_carries_still_reaches_the_peer_beside_the_identitys_headers(
) -> Result<(), Box<dyn Error>> {
    use grpc_egress_test_support::{PeerBehaviour, PeerServer};
    use MetadataBuilder;
    let peer = PeerServer::start(PeerBehaviour::Reply(b"ok".to_vec())).await?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .build()?;
    let ctx = SecurityContext::unauthenticated().with_trace_id("trace-1");
    let mut metadata = MetadataBuilder::create();
    metadata.append_binary("x-payload-bin", &[0u8, 1, 2, 255])?;

    Fixture::view(&client, &ctx)
        .call_raw(
            CallRawRequestBuilder::new()
                .with_path(CallFixture::path("/test.Peer/Echo"))
                .with_message(Fixture::encode("hi"))
                .with_options(CallOptions::create(None, metadata.build()))
                .build(),
        )
        .await
        .map(CallRawResponse::into_response)?;

    let requests = peer.get_recorded_requests();
    let request = requests.first().ok_or("the peer received no request")?;
    assert!(
        request.get_header("x-payload-bin").is_some(),
        "the binary header was lost"
    );
    assert_eq!(request.get_header("x-trace-id"), Some("trace-1"));
    Ok(())
}

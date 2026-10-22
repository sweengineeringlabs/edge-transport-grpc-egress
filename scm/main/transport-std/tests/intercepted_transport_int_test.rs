//! The client the builder returns runs its interceptors around every call: `before_call` on every shape, typed and raw,
//! `after_call` after a response with a message, in registration order, and an interceptor that refuses a call
//! stops it before anything is sent. Against a real `tonic` server (`proto-test-peer`).

use proto_pattern::{
    CallBidiStreamRequest, CallBidiStreamResponse, CallClientStreamRequest,
    CallClientStreamResponse, CallError, CallOptions, CallRawBidiStreamRequestBuilder,
    CallRawBidiStreamResponse, CallRawClientStreamRequestBuilder, CallRawClientStreamResponse,
    CallRawRequestBuilder, CallRawResponse, CallRawServerStreamRequestBuilder,
    CallRawServerStreamResponse, CallRequest, CallServerStreamRequest, CallServerStreamResponse,
    GetValuesRequest, GetValuesResponse, MessageStream, Metadata, MetadataEntry, MetadataView,
    RawTransport, ReadMessageRequest, ReadMessageResponse, StatusCode, Transport,
};
use std::error::Error;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use grpc_egress_test_support::CallFixture;
use grpc_egress_transport::{
    BuildGrpcEgressBuilderRequest, GrpcChannelConfigError, GrpcEgressBuilder,
    GrpcEgressBuilderFactory, GrpcEgressInterceptor, InboundReply, OutboundCall,
    TraceContextSource,
};
use grpc_egress_transport_std::TonicGrpcEgressBuilderSpiFactory;
use proto_test_peer::{Chat, Count, Echo, Join, ListStream, Numbers, RunningPeer, Say};

const TRACEPARENT: &str = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";

// ---- interceptors the tests run

/// Adds the header `key: value` before every call.
struct Tag {
    key: &'static str,
    value: &'static str,
}

impl GrpcEgressInterceptor for Tag {
    fn before_call(&self, call: &mut dyn OutboundCall) -> Result<(), CallError> {
        call.add_header(self.key, self.value)
    }

    fn after_call(&self, _: &InboundReply<'_>) -> Result<(), CallError> {
        Ok(())
    }
}

/// Records `before:<marker>` and `after:<marker>` in a shared log.
struct Log {
    marker: &'static str,
    log: Arc<Mutex<Vec<String>>>,
}

impl Log {
    fn note(&self, phase: &str) {
        self.log
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(format!("{phase}:{}", self.marker));
    }
}

impl GrpcEgressInterceptor for Log {
    fn before_call(&self, _: &mut dyn OutboundCall) -> Result<(), CallError> {
        self.note("before");
        Ok(())
    }

    fn after_call(&self, _: &InboundReply<'_>) -> Result<(), CallError> {
        self.note("after");
        Ok(())
    }
}

/// Refuses every call before it is sent.
struct Deny;

impl GrpcEgressInterceptor for Deny {
    fn before_call(&self, call: &mut dyn OutboundCall) -> Result<(), CallError> {
        Err(CallError::Status {
            path: call.get_path().into(),
            code: StatusCode::PermissionDenied,
            message: "denied by interceptor".to_string(),
            details: Vec::new(),
            trailers: Metadata::default(),
        })
    }

    fn after_call(&self, _: &InboundReply<'_>) -> Result<(), CallError> {
        Ok(())
    }
}

/// Lets every call go and refuses every response.
struct RefuseReply;

impl GrpcEgressInterceptor for RefuseReply {
    fn before_call(&self, _: &mut dyn OutboundCall) -> Result<(), CallError> {
        Ok(())
    }

    fn after_call(&self, reply: &InboundReply<'_>) -> Result<(), CallError> {
        Err(CallError::Status {
            path: reply.path.into(),
            code: StatusCode::DataLoss,
            message: "reply refused by interceptor".to_string(),
            details: Vec::new(),
            trailers: Metadata::default(),
        })
    }
}

/// Counts the calls of each hook, in counters the test keeps.
struct Counting {
    before: Arc<AtomicUsize>,
    after: Arc<AtomicUsize>,
}

impl GrpcEgressInterceptor for Counting {
    fn before_call(&self, _: &mut dyn OutboundCall) -> Result<(), CallError> {
        self.before.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn after_call(&self, _: &InboundReply<'_>) -> Result<(), CallError> {
        self.after.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

/// What `after_call` was given.
#[derive(Debug, Clone)]
struct SeenReply {
    path: String,
    headers: Metadata,
}

/// Keeps every reply `after_call` is given.
struct Capture(Arc<Mutex<Vec<SeenReply>>>);

impl GrpcEgressInterceptor for Capture {
    fn before_call(&self, _: &mut dyn OutboundCall) -> Result<(), CallError> {
        Ok(())
    }

    fn after_call(&self, reply: &InboundReply<'_>) -> Result<(), CallError> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(SeenReply {
                path: reply.path.to_string(),
                headers: reply.headers.clone(),
            });
        Ok(())
    }
}

/// Sets the timeout of every call.
struct SetTimeout(Duration);

impl GrpcEgressInterceptor for SetTimeout {
    fn before_call(&self, call: &mut dyn OutboundCall) -> Result<(), CallError> {
        call.set_timeout(Some(self.0));
        Ok(())
    }

    fn after_call(&self, _: &InboundReply<'_>) -> Result<(), CallError> {
        Ok(())
    }
}

// ---- what the tests send

struct Fixture;

impl Fixture {
    /// A client for `peer` with `interceptors` in the order given, built through the public builder.
    fn client(
        peer: &RunningPeer,
        interceptors: Vec<Arc<dyn GrpcEgressInterceptor>>,
    ) -> Result<impl Transport + RawTransport + use<>, GrpcChannelConfigError> {
        let mut builder = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
            .build(BuildGrpcEgressBuilderRequest)
            .with_plaintext_allowed();
        for interceptor in interceptors {
            builder = builder.with_interceptor(interceptor);
        }
        builder.build()
    }

    fn tag() -> Arc<dyn GrpcEgressInterceptor> {
        Arc::new(Tag {
            key: "x-tag",
            value: "tagged",
        })
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

    fn decode_say(bytes: &[u8]) -> Result<Say, Box<dyn Error>> {
        Ok(prost::Message::decode(bytes)?)
    }

    fn decode_count(bytes: &[u8]) -> Result<Count, Box<dyn Error>> {
        Ok(prost::Message::decode(bytes)?)
    }

    fn caller_header(key: &str, value: &str) -> Result<CallOptions, Box<dyn Error>> {
        Ok(CallOptions::create(
            None,
            Metadata::create(vec![MetadataEntry::create(key, value)?]),
        ))
    }
}

// ---- before_call runs on every shape, typed and raw

#[tokio::test]
async fn test_call_unary_typed_header_added_by_an_interceptor_reaches_the_peer(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer, vec![Fixture::tag()])?;

    let response = client.call::<Echo>(Fixture::typed("hi")).await?;

    assert_eq!(response.get_message(), &Say::of("hi"));
    assert_eq!(peer.get_last_header("x-tag").as_deref(), Some("tagged"));
    Ok(())
}

#[tokio::test]
async fn test_call_raw_unary_header_added_by_an_interceptor_reaches_the_peer(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer, vec![Fixture::tag()])?;

    let response = client
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

    assert_eq!(Fixture::decode_say(response.get_message())?, Say::of("hi"));
    assert_eq!(peer.get_last_header("x-tag").as_deref(), Some("tagged"));
    Ok(())
}

#[tokio::test]
async fn test_call_server_stream_typed_header_added_by_an_interceptor_reaches_the_peer(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer, vec![Fixture::tag()])?;

    let mut stream = client
        .call_server_stream::<Numbers>({
            let (message, options) = Fixture::typed("2").into_parts();
            CallServerStreamRequest::new(message, options)
        })
        .await
        .map(CallServerStreamResponse::into_stream)?;

    assert_eq!(
        stream
            .read_message(ReadMessageRequest::new())
            .await
            .map(ReadMessageResponse::into_message)?,
        Some(Count { value: 1 })
    );
    assert_eq!(peer.get_last_header("x-tag").as_deref(), Some("tagged"));
    Ok(())
}

#[tokio::test]
async fn test_call_raw_server_stream_header_added_by_an_interceptor_reaches_the_peer(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer, vec![Fixture::tag()])?;

    let mut stream = client
        .call_raw_server_stream({
            let (message, options) = Fixture::raw(&Say::of("2")).into_parts();
            CallRawServerStreamRequestBuilder::new()
                .with_path(CallFixture::path("/test.Peer/Numbers"))
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawServerStreamResponse::into_stream)?;

    let first = stream
        .read_message(ReadMessageRequest::new())
        .await
        .map(ReadMessageResponse::into_message)?
        .ok_or("no message")?;
    assert_eq!(Fixture::decode_count(&first)?, Count { value: 1 });
    assert_eq!(peer.get_last_header("x-tag").as_deref(), Some("tagged"));
    Ok(())
}

#[tokio::test]
async fn test_call_client_stream_typed_header_added_by_an_interceptor_reaches_the_peer(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer, vec![Fixture::tag()])?;

    let response = client
        .call_client_stream::<Join, _>(CallClientStreamRequest::new(
            ListStream::of(vec![Say::of("a"), Say::of("b")]),
            CallOptions::default(),
        ))
        .await
        .map(CallClientStreamResponse::into_response)?;

    assert_eq!(response.get_message(), &Say::of("a,b"));
    assert_eq!(peer.get_last_header("x-tag").as_deref(), Some("tagged"));
    Ok(())
}

#[tokio::test]
async fn test_call_raw_client_stream_header_added_by_an_interceptor_reaches_the_peer(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer, vec![Fixture::tag()])?;

    let response = client
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

    assert_eq!(Fixture::decode_say(response.get_message())?, Say::of("a,b"));
    assert_eq!(peer.get_last_header("x-tag").as_deref(), Some("tagged"));
    Ok(())
}

#[tokio::test]
async fn test_call_bidi_stream_typed_header_added_by_an_interceptor_reaches_the_peer(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer, vec![Fixture::tag()])?;

    let mut stream = client
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
    assert_eq!(peer.get_last_header("x-tag").as_deref(), Some("tagged"));
    Ok(())
}

#[tokio::test]
async fn test_call_raw_bidi_stream_header_added_by_an_interceptor_reaches_the_peer(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer, vec![Fixture::tag()])?;

    let mut stream = client
        .call_raw_bidi_stream(
            CallRawBidiStreamRequestBuilder::new()
                .with_path(CallFixture::path("/test.Peer/Chat"))
                .with_requests(ListStream::of(vec![Fixture::encode("one")]))
                .with_options(CallOptions::default())
                .build(),
        )
        .await
        .map(CallRawBidiStreamResponse::into_stream)?;

    let first = stream
        .read_message(ReadMessageRequest::new())
        .await
        .map(ReadMessageResponse::into_message)?
        .ok_or("no message")?;
    assert_eq!(Fixture::decode_say(&first)?, Say::of("ONE"));
    assert_eq!(peer.get_last_header("x-tag").as_deref(), Some("tagged"));
    Ok(())
}

// ---- order

#[tokio::test]
async fn test_call_interceptors_run_before_then_after_each_in_registration_order(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let log = Arc::new(Mutex::new(Vec::new()));
    let log_of = |marker| -> Arc<dyn GrpcEgressInterceptor> {
        Arc::new(Log {
            marker,
            log: Arc::clone(&log),
        })
    };
    let client = Fixture::client(&peer, vec![log_of("a"), log_of("b"), log_of("c")])?;

    client.call::<Echo>(Fixture::typed("hi")).await?;

    let seen = log
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    assert_eq!(
        seen,
        ["before:a", "before:b", "before:c", "after:a", "after:b", "after:c"]
    );
    Ok(())
}

#[tokio::test]
async fn test_call_later_interceptor_sees_the_header_an_earlier_one_added_and_the_peer_gets_both(
) -> Result<(), Box<dyn Error>> {
    /// Adds `x-second` only when `x-tag` is already on the call.
    struct Follower;
    impl GrpcEgressInterceptor for Follower {
        fn before_call(&self, call: &mut dyn OutboundCall) -> Result<(), CallError> {
            if call.has_header("x-tag") {
                call.add_header("x-second", "saw-tag")?;
            }
            Ok(())
        }

        fn after_call(&self, _: &InboundReply<'_>) -> Result<(), CallError> {
            Ok(())
        }
    }
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer, vec![Fixture::tag(), Arc::new(Follower)])?;

    client.call::<Echo>(Fixture::typed("hi")).await?;

    assert_eq!(peer.get_last_header("x-tag").as_deref(), Some("tagged"));
    assert_eq!(peer.get_last_header("x-second").as_deref(), Some("saw-tag"));
    Ok(())
}

// ---- a refusal stops the call before anything is sent

#[tokio::test]
async fn test_call_unary_typed_interceptor_that_refuses_stops_the_call_before_the_peer_and_the_later_interceptors(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let later = Arc::new(AtomicUsize::new(0));
    let client = Fixture::client(
        &peer,
        vec![
            Arc::new(Deny),
            Arc::new(Counting {
                before: Arc::clone(&later),
                after: Arc::new(AtomicUsize::new(0)),
            }),
        ],
    )?;

    let result = client.call::<Echo>(Fixture::typed("hi")).await;

    assert!(
        matches!(
            &result,
            Err(CallError::Status { path, code: StatusCode::PermissionDenied, message, .. })
                if path.as_ref() == "/test.Peer/Echo" && message == "denied by interceptor"
        ),
        "got {:?}",
        result.err()
    );
    assert_eq!(peer.get_reached(), 0, "nothing may reach the peer");
    assert_eq!(
        later.load(Ordering::SeqCst),
        0,
        "a later interceptor must not run"
    );
    Ok(())
}

#[tokio::test]
async fn test_call_raw_unary_interceptor_that_refuses_stops_the_call_before_the_peer(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer, vec![Arc::new(Deny)])?;

    let result = client
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
            Err(CallError::Status {
                code: StatusCode::PermissionDenied,
                ..
            })
        ),
        "got {:?}",
        result.err()
    );
    assert_eq!(peer.get_reached(), 0);
    Ok(())
}

#[tokio::test]
async fn test_call_server_stream_typed_interceptor_that_refuses_stops_the_call_before_the_peer(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer, vec![Arc::new(Deny)])?;

    let result = client
        .call_server_stream::<Numbers>({
            let (message, options) = Fixture::typed("2").into_parts();
            CallServerStreamRequest::new(message, options)
        })
        .await
        .map(CallServerStreamResponse::into_stream);

    assert!(
        matches!(
            result,
            Err(CallError::Status {
                code: StatusCode::PermissionDenied,
                ..
            })
        ),
        "a refused stream must not open"
    );
    assert_eq!(peer.get_reached(), 0);
    Ok(())
}

#[tokio::test]
async fn test_call_raw_bidi_stream_interceptor_that_refuses_stops_the_call_before_the_peer(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer, vec![Arc::new(Deny)])?;

    let result = client
        .call_raw_bidi_stream(
            CallRawBidiStreamRequestBuilder::new()
                .with_path(CallFixture::path("/test.Peer/Chat"))
                .with_requests(ListStream::of(vec![Fixture::encode("one")]))
                .with_options(CallOptions::default())
                .build(),
        )
        .await
        .map(CallRawBidiStreamResponse::into_stream);

    assert!(
        matches!(
            result,
            Err(CallError::Status {
                code: StatusCode::PermissionDenied,
                ..
            })
        ),
        "a refused stream must not open"
    );
    assert_eq!(peer.get_reached(), 0);
    Ok(())
}

// ---- after_call

#[tokio::test]
async fn test_call_unary_interceptor_that_refuses_the_reply_fails_the_call_after_the_peer_answered(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer, vec![Arc::new(RefuseReply)])?;

    let result = client.call::<Echo>(Fixture::typed("hi")).await;

    assert!(
        matches!(
            &result,
            Err(CallError::Status { path, code: StatusCode::DataLoss, .. })
                if path.as_ref() == "/test.Peer/Echo"
        ),
        "got {:?}",
        result.err()
    );
    assert_eq!(
        peer.get_reached(),
        1,
        "the call was made; its reply was refused"
    );
    Ok(())
}

#[tokio::test]
async fn test_call_unary_peer_answers_an_error_status_runs_no_after_call(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let (before, after) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
    let client = Fixture::client(
        &peer,
        vec![Arc::new(Counting {
            before: Arc::clone(&before),
            after: Arc::clone(&after),
        })],
    )?;

    let result = client
        .call::<Echo>(CallRequest::new(
            Say::failing(StatusCode::NotFound.get_code()),
            CallOptions::default(),
        ))
        .await;

    assert!(matches!(
        result,
        Err(CallError::Status {
            code: StatusCode::NotFound,
            ..
        })
    ));
    assert_eq!(before.load(Ordering::SeqCst), 1);
    assert_eq!(
        after.load(Ordering::SeqCst),
        0,
        "a reply that is an error has no message to show an interceptor"
    );
    Ok(())
}

#[tokio::test]
async fn test_call_after_call_runs_for_unary_and_client_stream_and_not_for_the_other_shapes(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let (before, after) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
    let client = Fixture::client(
        &peer,
        vec![Arc::new(Counting {
            before: Arc::clone(&before),
            after: Arc::clone(&after),
        })],
    )?;

    client.call::<Echo>(Fixture::typed("hi")).await?;
    assert_eq!(after.load(Ordering::SeqCst), 1, "unary");
    client
        .call_client_stream::<Join, _>(CallClientStreamRequest::new(
            ListStream::of(vec![Say::of("a")]),
            CallOptions::default(),
        ))
        .await
        .map(CallClientStreamResponse::into_response)?;
    assert_eq!(after.load(Ordering::SeqCst), 2, "client stream");
    let mut numbers = client
        .call_server_stream::<Numbers>({
            let (message, options) = Fixture::typed("2").into_parts();
            CallServerStreamRequest::new(message, options)
        })
        .await
        .map(CallServerStreamResponse::into_stream)?;
    while numbers
        .read_message(ReadMessageRequest::new())
        .await
        .map(ReadMessageResponse::into_message)?
        .is_some()
    {}
    assert_eq!(after.load(Ordering::SeqCst), 2, "server stream");
    let mut chat = client
        .call_bidi_stream::<Chat, _>(CallBidiStreamRequest::new(
            ListStream::of(vec![Say::of("a")]),
            CallOptions::default(),
        ))
        .await
        .map(CallBidiStreamResponse::into_stream)?;
    while chat
        .read_message(ReadMessageRequest::new())
        .await
        .map(ReadMessageResponse::into_message)?
        .is_some()
    {}
    assert_eq!(after.load(Ordering::SeqCst), 2, "bidi stream");
    assert_eq!(
        before.load(Ordering::SeqCst),
        4,
        "before_call runs on every shape"
    );
    Ok(())
}

#[tokio::test]
async fn test_call_unary_response_headers_and_the_path_reach_after_call(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let seen = Arc::new(Mutex::new(Vec::new()));
    // The peer reports the `x-token` request header it saw as the `x-seen` response header.
    let client = Fixture::client(
        &peer,
        vec![
            Arc::new(Tag {
                key: "x-token",
                value: "tok",
            }),
            Arc::new(Capture(Arc::clone(&seen))),
        ],
    )?;

    client.call::<Echo>(Fixture::typed("hi")).await?;

    let seen = seen
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    let reply = seen.first().ok_or("after_call was not given a reply")?;
    assert_eq!(reply.path, "/test.Peer/Echo");
    assert_eq!(
        reply
            .headers
            .get_values(GetValuesRequest::new("x-seen"))
            .map(GetValuesResponse::into_values)
            .unwrap_or_default(),
        ["tok"]
    );
    Ok(())
}

// ---- the call carries what the interceptors made of it

#[tokio::test]
async fn test_call_interceptor_that_sets_a_timeout_ends_the_call_at_it(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(
        &peer,
        vec![Arc::new(SetTimeout(Duration::from_millis(150)))],
    )?;
    let started = Instant::now();

    let result = client
        .call_raw({
            let (message, options) = Fixture::raw(&Say::slow(3000)).into_parts();
            CallRawRequestBuilder::new()
                .with_path(CallFixture::path("/test.Peer/Echo"))
                .with_message(message)
                .with_options(options)
                .build()
        })
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
    assert!(
        started.elapsed() < Duration::from_millis(2500),
        "the call must end at the timeout the interceptor set, took {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn test_call_with_no_interceptor_forwards_the_callers_headers_and_adds_none(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer, Vec::new())?;

    client
        .call::<Echo>(CallRequest::new(
            Say::of("hi"),
            Fixture::caller_header("x-caller", "mine")?,
        ))
        .await?;

    assert_eq!(peer.get_last_header("x-caller").as_deref(), Some("mine"));
    assert_eq!(peer.get_last_header("x-tag"), None);
    Ok(())
}

#[tokio::test]
async fn test_call_interceptor_keeps_the_headers_the_caller_set_and_adds_its_own(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer, vec![Fixture::tag()])?;

    client
        .call::<Echo>(CallRequest::new(
            Say::of("hi"),
            Fixture::caller_header("x-caller", "mine")?,
        ))
        .await?;

    assert_eq!(peer.get_last_header("x-caller").as_deref(), Some("mine"));
    assert_eq!(peer.get_last_header("x-tag").as_deref(), Some("tagged"));
    Ok(())
}

// ---- trace context through the builder

fn traced(
    peer: &RunningPeer,
    source: TraceContextSource,
) -> Result<impl Transport + RawTransport + use<>, GrpcChannelConfigError> {
    TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_trace_context(source)
        .build()
}

#[tokio::test]
async fn test_call_static_trace_context_sends_traceparent_and_tracestate_to_the_peer(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = traced(
        &peer,
        TraceContextSource::Static {
            traceparent: TRACEPARENT.to_string(),
            tracestate: Some("vendor=1".to_string()),
        },
    )?;

    client.call::<Echo>(Fixture::typed("hi")).await?;

    assert_eq!(
        peer.get_last_header("traceparent").as_deref(),
        Some(TRACEPARENT)
    );
    assert_eq!(
        peer.get_last_header("tracestate").as_deref(),
        Some("vendor=1")
    );
    Ok(())
}

#[tokio::test]
async fn test_call_static_trace_context_keeps_the_traceparent_the_caller_carries(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = traced(
        &peer,
        TraceContextSource::Static {
            traceparent: TRACEPARENT.to_string(),
            tracestate: Some("vendor=1".to_string()),
        },
    )?;

    client
        .call::<Echo>(CallRequest::new(
            Say::of("hi"),
            Fixture::caller_header("traceparent", "caller-value")?,
        ))
        .await?;

    assert_eq!(
        peer.get_last_header("traceparent").as_deref(),
        Some("caller-value")
    );
    assert_eq!(
        peer.get_last_header("tracestate"),
        None,
        "a call that carries a trace context gets none of ours"
    );
    Ok(())
}

#[tokio::test]
async fn test_call_pass_through_trace_context_sends_no_traceparent() -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = traced(&peer, TraceContextSource::PassThrough)?;

    client.call::<Echo>(Fixture::typed("hi")).await?;

    assert_eq!(peer.get_last_header("traceparent"), None);
    Ok(())
}

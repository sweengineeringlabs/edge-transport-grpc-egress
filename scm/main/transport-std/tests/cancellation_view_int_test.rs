//! A client bound to a `CancellationToken` (`StdGrpcCancellationFactory`): a call in flight, and every message read from
//! a stream it opened, fails with `Cancelled` when the token fires, and a token that has not fired changes nothing.
//! Against a real `tonic` server (`proto-test-peer`).

use std::error::Error;
use std::time::{Duration, Instant};

use grpc_egress_test_support::CallFixture;
use grpc_egress_transport::{
    BuildGrpcCancellationRequest, BuildGrpcEgressBuilderRequest, GrpcCancellationFactory,
    GrpcChannelConfigError, GrpcEgressBuilder, GrpcEgressBuilderFactory,
};
use grpc_egress_transport_std::{
    StdGrpcCancellationFactory, TokenCancellation, TonicGrpcEgressBuilderSpiFactory,
};
use proto_pattern::{
    CallError, CallOptions, CallRequest, MessageStream, RawTransport, ResponseStream, StatusCode,
    Transport,
};
use proto_test_peer::{Chat, Count, Echo, Join, RunningPeer, Say};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// How long a test lets a call run before it fires the token.
const FIRE_AFTER: Duration = Duration::from_millis(100);

/// The longest a call that was cancelled may take: far less than the 3 s the slow peer would take.
const PROMPT: Duration = Duration::from_millis(2000);

/// A request stream the test feeds by hand: it ends only when the test drops the sender.
struct ChannelStream<T> {
    receiver: mpsc::Receiver<T>,
}

impl<T: Send> MessageStream<T> for ChannelStream<T> {
    async fn read_message(&mut self) -> Result<Option<T>, CallError> {
        Ok(self.receiver.recv().await)
    }
}

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

    /// `client` bound to `token`.
    fn view<'a, T: Transport + RawTransport>(
        client: &'a T,
        token: &CancellationToken,
    ) -> impl Transport + RawTransport + use<'a, T> {
        StdGrpcCancellationFactory::new().build(BuildGrpcCancellationRequest {
            inner: client,
            signal: TokenCancellation::from(token.clone()),
        })
    }

    /// Fires `token` after [`FIRE_AFTER`], from another task.
    fn fire_later(token: &CancellationToken) {
        let token = token.clone();
        tokio::spawn(async move {
            tokio::time::sleep(FIRE_AFTER).await;
            token.cancel();
        });
    }

    fn typed(say: Say) -> CallRequest<Say> {
        CallRequest::create(say, CallOptions::default())
    }

    fn raw(say: &Say) -> CallRequest<Vec<u8>> {
        CallRequest::create(prost::Message::encode_to_vec(say), CallOptions::default())
    }

    /// `error` is a `Cancelled` transport failure that names `path`.
    fn is_cancelled(error: &CallError, path: &str) -> bool {
        matches!(
            error,
            CallError::Transport { path: named, code: StatusCode::Cancelled, .. }
                if named.as_ref() == path
        )
    }
}

// ---- a token that has not fired changes nothing

#[tokio::test]
async fn test_call_typed_token_that_never_fires_returns_the_peers_answer(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer)?;
    let token = CancellationToken::new();

    let response = Fixture::view(&client, &token)
        .call::<Echo>(Fixture::typed(Say::of("hi")))
        .await?;

    assert_eq!(response.get_message(), &Say::of("hi"));
    Ok(())
}

#[tokio::test]
async fn test_call_raw_server_stream_token_that_never_fires_returns_every_message(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer)?;
    let token = CancellationToken::new();
    let view = Fixture::view(&client, &token);

    let mut stream = view
        .call_raw_server_stream(
            CallFixture::path("/test.Peer/Numbers"),
            Fixture::raw(&Say::of("3")),
        )
        .await?;

    let mut seen = Vec::new();
    while let Some(bytes) = stream.read_message().await? {
        seen.push(<Count as prost::Message>::decode(bytes.as_slice())?);
    }
    assert_eq!(
        seen,
        [Count { value: 1 }, Count { value: 2 }, Count { value: 3 }]
    );
    Ok(())
}

// ---- a token that fired before the call stops it before the peer

#[tokio::test]
async fn test_call_typed_token_fired_before_the_call_fails_with_cancelled_and_nothing_reaches_the_peer(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer)?;
    let token = CancellationToken::new();
    token.cancel();

    let result = Fixture::view(&client, &token)
        .call::<Echo>(Fixture::typed(Say::of("hi")))
        .await;

    let error = result.err().ok_or("a fired token must fail the call")?;
    assert!(
        Fixture::is_cancelled(&error, "/test.Peer/Echo"),
        "got {error:?}"
    );
    assert_eq!(peer.get_reached(), 0);
    Ok(())
}

#[tokio::test]
async fn test_call_raw_token_fired_before_the_call_fails_with_cancelled_and_nothing_reaches_the_peer(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer)?;
    let token = CancellationToken::new();
    token.cancel();

    let result = Fixture::view(&client, &token)
        .call_raw(
            CallFixture::path("/test.Peer/Echo"),
            Fixture::raw(&Say::of("hi")),
        )
        .await;

    let error = result.err().ok_or("a fired token must fail the call")?;
    assert!(
        Fixture::is_cancelled(&error, "/test.Peer/Echo"),
        "got {error:?}"
    );
    assert_eq!(peer.get_reached(), 0);
    Ok(())
}

// ---- a token that fires while a call is in flight ends it

#[tokio::test]
async fn test_call_typed_token_fired_while_the_peer_is_slow_fails_with_cancelled_promptly(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer)?;
    let token = CancellationToken::new();
    Fixture::fire_later(&token);
    let started = Instant::now();

    let result = Fixture::view(&client, &token)
        .call::<Echo>(Fixture::typed(Say::slow(3000)))
        .await;

    let error = result
        .err()
        .ok_or("the peer answers after 3 s; the token fired at 100 ms")?;
    assert!(
        Fixture::is_cancelled(&error, "/test.Peer/Echo"),
        "got {error:?}"
    );
    assert!(started.elapsed() < PROMPT, "took {:?}", started.elapsed());
    assert_eq!(
        peer.get_reached(),
        1,
        "the call had started when the token fired"
    );
    Ok(())
}

#[tokio::test]
async fn test_call_raw_token_fired_while_the_peer_is_slow_fails_with_cancelled_promptly(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer)?;
    let token = CancellationToken::new();
    Fixture::fire_later(&token);
    let started = Instant::now();

    let result = Fixture::view(&client, &token)
        .call_raw(
            CallFixture::path("/test.Peer/Echo"),
            Fixture::raw(&Say::slow(3000)),
        )
        .await;

    let error = result
        .err()
        .ok_or("the peer answers after 3 s; the token fired at 100 ms")?;
    assert!(
        Fixture::is_cancelled(&error, "/test.Peer/Echo"),
        "got {error:?}"
    );
    assert!(started.elapsed() < PROMPT, "took {:?}", started.elapsed());
    Ok(())
}

#[tokio::test]
async fn test_call_client_stream_typed_token_fired_while_the_request_stream_is_open_fails_with_cancelled(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer)?;
    let token = CancellationToken::new();
    Fixture::fire_later(&token);
    // The sender stays alive, so the request stream never ends and the peer never answers.
    let (sender, receiver) = mpsc::channel::<Say>(4);
    sender.send(Say::of("a")).await?;
    let started = Instant::now();

    let result = Fixture::view(&client, &token)
        .call_client_stream::<Join, _>(ChannelStream { receiver }, CallOptions::default())
        .await;

    let error = result
        .err()
        .ok_or("an unfinished client stream cannot answer")?;
    assert!(
        Fixture::is_cancelled(&error, "/test.Peer/Join"),
        "got {error:?}"
    );
    assert!(started.elapsed() < PROMPT, "took {:?}", started.elapsed());
    drop(sender);
    Ok(())
}

// ---- a token that fires while a stream is being read ends the read

#[tokio::test]
async fn test_read_message_typed_bidi_token_fired_while_waiting_fails_with_cancelled(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer)?;
    let token = CancellationToken::new();
    let (sender, receiver) = mpsc::channel::<Say>(4);
    let view = Fixture::view(&client, &token);
    let mut stream = view
        .call_bidi_stream::<Chat, _>(ChannelStream { receiver }, CallOptions::default())
        .await?;
    // Nothing was sent, so the peer has nothing to answer: a read waits.
    Fixture::fire_later(&token);
    let started = Instant::now();

    let result = tokio::time::timeout(PROMPT, stream.read_message())
        .await
        .map_err(|_| "the read did not end after the token fired")?;

    let error = result
        .err()
        .ok_or("no message can arrive; the token fired")?;
    assert!(
        Fixture::is_cancelled(&error, "/test.Peer/Chat"),
        "got {error:?}"
    );
    assert!(started.elapsed() < PROMPT, "took {:?}", started.elapsed());
    drop(sender);
    Ok(())
}

#[tokio::test]
async fn test_read_message_raw_bidi_token_fired_while_waiting_fails_with_cancelled(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer)?;
    let token = CancellationToken::new();
    let (sender, receiver) = mpsc::channel::<Vec<u8>>(4);
    let view = Fixture::view(&client, &token);
    let mut stream = view
        .call_raw_bidi_stream(
            CallFixture::path("/test.Peer/Chat"),
            ChannelStream { receiver },
            CallOptions::default(),
        )
        .await?;
    Fixture::fire_later(&token);

    let result = tokio::time::timeout(PROMPT, stream.read_message())
        .await
        .map_err(|_| "the read did not end after the token fired")?;

    let error = result
        .err()
        .ok_or("no message can arrive; the token fired")?;
    assert!(
        Fixture::is_cancelled(&error, "/test.Peer/Chat"),
        "got {error:?}"
    );
    drop(sender);
    Ok(())
}

#[tokio::test]
async fn test_read_trailers_typed_bidi_token_fired_while_waiting_fails_with_cancelled(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer)?;
    let token = CancellationToken::new();
    let (sender, receiver) = mpsc::channel::<Say>(4);
    let view = Fixture::view(&client, &token);
    let mut stream = view
        .call_bidi_stream::<Chat, _>(ChannelStream { receiver }, CallOptions::default())
        .await?;
    // The stream has not ended, so there are no trailers to read yet.
    Fixture::fire_later(&token);

    let result = tokio::time::timeout(PROMPT, stream.read_trailers())
        .await
        .map_err(|_| "the read did not end after the token fired")?;

    let error = result
        .err()
        .ok_or("no trailers can arrive; the token fired")?;
    assert!(
        Fixture::is_cancelled(&error, "/test.Peer/Chat"),
        "got {error:?}"
    );
    drop(sender);
    Ok(())
}

#[tokio::test]
async fn test_read_message_typed_bidi_token_that_never_fires_delivers_the_answer(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer)?;
    let token = CancellationToken::new();
    let (sender, receiver) = mpsc::channel::<Say>(4);
    let view = Fixture::view(&client, &token);
    let mut stream = view
        .call_bidi_stream::<Chat, _>(ChannelStream { receiver }, CallOptions::default())
        .await?;

    sender.send(Say::of("one")).await?;

    assert_eq!(stream.read_message().await?, Some(Say::of("ONE")));
    drop(sender);
    assert_eq!(
        stream.read_message().await?,
        None,
        "the peer ends the stream when the request stream ends"
    );
    Ok(())
}

// ---- the view knows the interface, not the token: a signal written here works the same

/// A signal that is not a token: a flag the test raises, awaited through a `Notify`; clones share the flag.
#[derive(Clone)]
struct FlagSignal(std::sync::Arc<FlagState>);

struct FlagState {
    raised: std::sync::atomic::AtomicBool,
    notify: tokio::sync::Notify,
}

impl FlagSignal {
    fn create() -> Self {
        Self(std::sync::Arc::new(FlagState {
            raised: std::sync::atomic::AtomicBool::new(false),
            notify: tokio::sync::Notify::new(),
        }))
    }

    fn raise(&self) {
        self.0
            .raised
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.0.notify.notify_waiters();
    }
}

impl grpc_egress_transport::CancellationSignal for FlagSignal {
    async fn wait_cancelled(&self) {
        loop {
            let notified = self.0.notify.notified();
            if self.0.raised.load(std::sync::atomic::Ordering::SeqCst) {
                return;
            }
            notified.await;
        }
    }
}

#[tokio::test]
async fn test_call_typed_custom_signal_raised_while_the_peer_is_slow_fails_with_cancelled_promptly(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer)?;
    let signal = FlagSignal::create();
    let raiser = signal.clone();
    tokio::spawn(async move {
        tokio::time::sleep(FIRE_AFTER).await;
        raiser.raise();
    });
    let view = StdGrpcCancellationFactory::new().build(BuildGrpcCancellationRequest {
        inner: &client,
        signal,
    });
    let started = Instant::now();

    let result = view.call::<Echo>(Fixture::typed(Say::slow(3000))).await;

    let error = result
        .err()
        .ok_or("the peer answers after 3 s; the signal was raised at 100 ms")?;
    assert!(
        Fixture::is_cancelled(&error, "/test.Peer/Echo"),
        "got {error:?}"
    );
    assert!(started.elapsed() < PROMPT, "took {:?}", started.elapsed());
    Ok(())
}

#[tokio::test]
async fn test_call_typed_custom_signal_never_raised_returns_the_peers_answer(
) -> Result<(), Box<dyn Error>> {
    let peer = RunningPeer::start().await;
    let client = Fixture::client(&peer)?;
    let view = StdGrpcCancellationFactory::new().build(BuildGrpcCancellationRequest {
        inner: &client,
        signal: FlagSignal::create(),
    });

    let response = view.call::<Echo>(Fixture::typed(Say::of("hi"))).await?;

    assert_eq!(response.get_message(), &Say::of("hi"));
    Ok(())
}

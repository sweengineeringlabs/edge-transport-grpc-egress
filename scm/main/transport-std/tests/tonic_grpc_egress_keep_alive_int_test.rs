//! Tests of the HTTP/2 keep-alive option of the client over the `proto-svc` engine. A proxy in front of a
//! healthy peer is frozen, so the peer stops answering without closing anything: the only way the
//! client can notice is by a PING that is not answered, and the proxy sees the client close.

use grpc_egress_test_support::{CallFixture, ClientFixture};
use grpc_egress_transport_std::GrpcChannelConfigFactory;
use proto_pattern::{
    CallError, CallRawRequestBuilder, CallRawResponse, RawTransport, StatusCode, Transport,
};
use std::error::Error;
use std::time::{Duration, Instant};

use grpc_egress_test_support::{
    Fixture, FreezableProxy, PeerBehaviour, PeerServer, RunningPeer, RunningProxy,
};
use grpc_egress_transport::{
    BuildGrpcEgressBuilderRequest, GrpcChannelConfig, GrpcEgressBuilder, GrpcEgressBuilderFactory,
    KeepAliveConfig,
};
use grpc_egress_transport_std::TonicGrpcEgressBuilderSpiFactory;

const PING_INTERVAL: Duration = Duration::from_millis(200);
const PING_TIMEOUT: Duration = Duration::from_millis(300);

fn keep_alive(permit_without_calls: bool) -> KeepAliveConfig {
    KeepAliveConfig {
        interval: PING_INTERVAL,
        timeout: PING_TIMEOUT,
        permit_without_calls,
    }
}

async fn call<C: RawTransport>(client: &C, timeout: Duration) -> Result<(), CallError> {
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
        .map(CallRawResponse::into_response)
        .map(|_| ())
}

/// A peer behind a proxy; both stop when dropped.
async fn start_proxied(
    behaviour: PeerBehaviour,
) -> Result<(RunningPeer, RunningProxy), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(behaviour).await?;
    let proxy = FreezableProxy::start(peer.get_addr()).await?;
    Ok((peer, proxy))
}

fn client_through(
    proxy: &RunningProxy,
    keep_alive: Option<KeepAliveConfig>,
) -> Result<impl Transport + RawTransport, Box<dyn Error>> {
    let builder = TonicGrpcEgressBuilderSpiFactory::new(proxy.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed();
    Ok(match keep_alive {
        Some(config) => builder.with_keep_alive(config).build()?,
        None => builder.build()?,
    })
}

/// `true` when `error` says the connection was lost: a failure of the transport, not an answer of the peer.
fn is_lost_connection(error: &CallError) -> bool {
    matches!(
        error,
        CallError::Transport {
            code: StatusCode::Unavailable | StatusCode::Unknown,
            ..
        }
    )
}

/// Start a call that the peer answers only after 10 s, freeze the proxy once the call is in
/// flight, and return how the call ended and how long it took.
async fn freeze_during_a_call<C: RawTransport + 'static>(
    client: C,
    proxy: &RunningProxy,
    timeout: Duration,
) -> (Result<(), CallError>, Duration) {
    let started = Instant::now();
    let handle = tokio::spawn(async move { call(&client, timeout).await });
    tokio::time::sleep(Duration::from_millis(400)).await;
    proxy.freeze();
    let result = handle.await.unwrap_or_else(|e| {
        Err(CallError::Transport {
            path: "".into(),
            code: StatusCode::Internal,
            reason: format!("call task failed: {e}"),
        })
    });
    (result, started.elapsed())
}

/// @covers: GrpcEgressBuilder::with_keep_alive
#[tokio::test]
async fn test_with_keep_alive_peer_that_stops_answering_pings_ends_the_call_near_the_timeout_error(
) -> Result<(), Box<dyn Error>> {
    let (_peer, proxy) = start_proxied(PeerBehaviour::Delay(Duration::from_secs(10))).await?;
    let client = client_through(&proxy, Some(keep_alive(false)))?;

    let (result, elapsed) = freeze_during_a_call(client, &proxy, Duration::from_secs(20)).await;

    assert!(
        result.as_ref().is_err_and(is_lost_connection),
        "an unanswered PING must end the call as a lost connection, got {result:?}"
    );
    assert!(
        elapsed < Duration::from_secs(4),
        "the call must end near interval + timeout, took {elapsed:?}"
    );
    assert!(
        proxy.wait_for_client_close(Duration::from_secs(1)).await,
        "the client must close the connection"
    );
    Ok(())
}

/// The control: the same frozen peer without keep-alive is noticed only at the call's deadline.
///
/// @covers: GrpcEgressBuilder::with_keep_alive
#[tokio::test]
async fn test_build_without_keep_alive_peer_that_stops_answering_is_noticed_only_at_the_deadline_edge(
) -> Result<(), Box<dyn Error>> {
    let (_peer, proxy) = start_proxied(PeerBehaviour::Delay(Duration::from_secs(10))).await?;
    let client = client_through(&proxy, None)?;

    let (result, elapsed) = freeze_during_a_call(client, &proxy, Duration::from_millis(2500)).await;

    assert!(
        matches!(
            result,
            Err(CallError::Status {
                code: StatusCode::DeadlineExceeded,
                ..
            })
        ),
        "without keep-alive only the deadline ends the call, got {result:?}"
    );
    assert!(elapsed >= Duration::from_millis(2300), "took {elapsed:?}");
    assert_eq!(proxy.get_closed_by_client(), 0);
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_keep_alive
#[tokio::test]
async fn test_with_keep_alive_permit_without_calls_idle_connection_to_a_frozen_peer_is_closed_happy(
) -> Result<(), Box<dyn Error>> {
    let (_peer, proxy) = start_proxied(PeerBehaviour::Reply(b"ok".to_vec())).await?;
    let client = client_through(&proxy, Some(keep_alive(true)))?;
    call(&client, Duration::from_secs(5)).await?;

    proxy.freeze();

    assert!(
        proxy.wait_for_client_close(Duration::from_secs(4)).await,
        "with permit_without_calls the idle connection must be probed and closed"
    );
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_keep_alive
#[tokio::test]
async fn test_with_keep_alive_without_permit_without_calls_idle_connection_is_left_alone_edge(
) -> Result<(), Box<dyn Error>> {
    let (_peer, proxy) = start_proxied(PeerBehaviour::Reply(b"ok".to_vec())).await?;
    let client = client_through(&proxy, Some(keep_alive(false)))?;
    call(&client, Duration::from_secs(5)).await?;

    proxy.freeze();

    assert!(
        !proxy
            .wait_for_client_close(Duration::from_millis(2500))
            .await,
        "no PING may be sent while no call is in flight"
    );
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_channel_config
#[tokio::test]
async fn test_with_channel_config_keep_alive_is_applied_to_the_client_happy(
) -> Result<(), Box<dyn Error>> {
    let (_peer, proxy) = start_proxied(PeerBehaviour::Delay(Duration::from_secs(10))).await?;
    let config = GrpcChannelConfig::new(proxy.get_endpoint())
        .allow_plaintext()
        .with_keep_alive(keep_alive(false));
    let client = TonicGrpcEgressBuilderSpiFactory::new(proxy.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_channel_config(&config)
        .build()?;

    let (result, elapsed) = freeze_during_a_call(client, &proxy, Duration::from_secs(20)).await;

    assert!(
        result.as_ref().is_err_and(is_lost_connection),
        "got {result:?}"
    );
    assert!(elapsed < Duration::from_secs(4), "took {elapsed:?}");
    Ok(())
}

/// @covers: ClientFixture::from_config
#[tokio::test]
async fn test_from_config_keep_alive_is_applied_to_the_client_the_caller_gets_happy(
) -> Result<(), Box<dyn Error>> {
    let (_peer, proxy) = start_proxied(PeerBehaviour::Delay(Duration::from_secs(10))).await?;
    let config = GrpcChannelConfig::new(proxy.get_endpoint())
        .allow_plaintext()
        .with_keep_alive(keep_alive(false));
    let client = ClientFixture::from_config(&config)?;

    let (result, elapsed) = freeze_during_a_call(client, &proxy, Duration::from_secs(20)).await;

    assert!(
        result.as_ref().is_err_and(is_lost_connection),
        "got {result:?}"
    );
    assert!(elapsed < Duration::from_secs(4), "took {elapsed:?}");
    Ok(())
}

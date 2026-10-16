//! Tests of the HTTP/2 keep-alive option of the `hyper`-backed client. A proxy in front of a
//! healthy peer is frozen, so the peer stops answering without closing anything: the only way the
//! client can notice is by a PING that is not answered, and the proxy sees the client close.

use grpc_egress_test_support::ClientFixture;
use grpc_egress_transport_std::{GrpcChannelConfigFactory, GrpcRequestFactory};
use std::error::Error;
use std::sync::Arc;
use std::time::{Duration, Instant};

use grpc_egress_test_support::{
    Fixture, FreezableProxy, PeerBehaviour, PeerServer, RunningPeer, RunningProxy,
};
use grpc_egress_transport::{
    BuildGrpcEgressBuilderRequest, GrpcChannelConfig, GrpcEgress, GrpcEgressBuilder,
    GrpcEgressBuilderFactory, GrpcEgressError, GrpcRequest, KeepAliveConfig,
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

fn request(deadline: Duration) -> GrpcRequest {
    GrpcRequest::new("pkg.Service/Method", b"ping".to_vec(), deadline)
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
) -> Result<impl GrpcEgress, Box<dyn Error>> {
    let builder = TonicGrpcEgressBuilderSpiFactory::new(proxy.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed();
    Ok(match keep_alive {
        Some(config) => builder.with_keep_alive(config).build()?,
        None => builder.build()?,
    })
}

/// Start a call that the peer answers only after 10 s, freeze the proxy once the call is in
/// flight, and return how the call ended and how long it took.
async fn freeze_during_a_call(
    client: Arc<dyn GrpcEgress>,
    proxy: &RunningProxy,
    deadline: Duration,
) -> (Result<(), GrpcEgressError>, Duration) {
    let started = Instant::now();
    let call = tokio::spawn(async move { client.call_unary(request(deadline)).await.map(|_| ()) });
    tokio::time::sleep(Duration::from_millis(400)).await;
    proxy.freeze();
    let result = call
        .await
        .unwrap_or_else(|e| Err(GrpcEgressError::Internal(format!("call task failed: {e}"))));
    (result, started.elapsed())
}

/// @covers: GrpcEgressBuilder::with_keep_alive
#[tokio::test]
async fn test_with_keep_alive_peer_that_stops_answering_pings_ends_the_call_near_the_timeout_error(
) -> Result<(), Box<dyn Error>> {
    let (_peer, proxy) = start_proxied(PeerBehaviour::Delay(Duration::from_secs(10))).await?;
    let client = client_through(&proxy, Some(keep_alive(false)))?;

    let (result, elapsed) =
        freeze_during_a_call(Arc::new(client), &proxy, Duration::from_secs(20)).await;

    assert!(
        matches!(result, Err(GrpcEgressError::ConnectionFailed(_))),
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

    let (result, elapsed) =
        freeze_during_a_call(Arc::new(client), &proxy, Duration::from_millis(2500)).await;

    assert!(
        matches!(result, Err(GrpcEgressError::Timeout(_))),
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
    client.call_unary(request(Duration::from_secs(5))).await?;

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
    client.call_unary(request(Duration::from_secs(5))).await?;

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

    let (result, elapsed) =
        freeze_during_a_call(Arc::new(client), &proxy, Duration::from_secs(20)).await;

    assert!(
        matches!(result, Err(GrpcEgressError::ConnectionFailed(_))),
        "got {result:?}"
    );
    assert!(elapsed < Duration::from_secs(4), "took {elapsed:?}");
    Ok(())
}

/// @covers: TransportConstruction::create_transport_from_config
#[tokio::test]
async fn test_create_transport_from_config_keep_alive_is_applied_to_the_client_happy(
) -> Result<(), Box<dyn Error>> {
    let (_peer, proxy) = start_proxied(PeerBehaviour::Delay(Duration::from_secs(10))).await?;
    let config = GrpcChannelConfig::new(proxy.get_endpoint())
        .allow_plaintext()
        .with_keep_alive(keep_alive(false));
    let client = ClientFixture::from_config(&config)?;

    let (result, elapsed) = freeze_during_a_call(client, &proxy, Duration::from_secs(20)).await;

    assert!(
        matches!(result, Err(GrpcEgressError::ConnectionFailed(_))),
        "got {result:?}"
    );
    assert!(elapsed < Duration::from_secs(4), "took {elapsed:?}");
    Ok(())
}

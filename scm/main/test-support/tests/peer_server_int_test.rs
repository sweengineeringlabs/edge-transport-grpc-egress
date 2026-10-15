//! Tests of `PeerServer`: it listens while running and stops when dropped.

use std::error::Error;
use std::time::Duration;

use grpc_egress_test_support::{PeerBehaviour, PeerServer};
use tokio::net::TcpStream;

/// @covers: PeerServer::start
#[tokio::test]
async fn test_start_peer_running_accepts_a_connection() -> Result<(), Box<dyn Error>> {
    let peer = PeerServer::start(PeerBehaviour::Echo).await?;

    TcpStream::connect(peer.get_addr()).await?;

    assert!(peer.get_endpoint().starts_with("http://127.0.0.1:"));
    assert!(
        peer.get_recorded_requests().is_empty(),
        "nothing was sent yet"
    );
    Ok(())
}

/// @covers: RunningPeer::get_addr
#[tokio::test]
async fn test_drop_peer_stops_accepting_connections() -> Result<(), Box<dyn Error>> {
    let peer = PeerServer::start(PeerBehaviour::Echo).await?;
    let addr = peer.get_addr();

    drop(peer);
    tokio::time::sleep(Duration::from_millis(100)).await;

    let attempt = TcpStream::connect(addr).await;
    assert!(
        attempt.is_err(),
        "a dropped peer must not accept connections"
    );
    Ok(())
}

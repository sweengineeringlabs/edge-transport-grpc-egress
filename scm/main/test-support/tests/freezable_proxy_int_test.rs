//! Tests of `FreezableProxy`: it forwards in both directions, a frozen proxy answers nothing but
//! keeps the connection open, and it counts the connections its clients close. The upstream is a
//! plain TCP echo, not the peer under test.

use std::error::Error;
use std::net::SocketAddr;
use std::time::Duration;

use grpc_egress_test_support::FreezableProxy;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::{TcpListener, TcpStream};

/// A TCP server that writes back everything it reads.
async fn start_echo() -> Result<SocketAddr, Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                while let Ok(n) = stream.read(&mut buf).await {
                    if n == 0 || stream.write_all(&buf[..n]).await.is_err() {
                        return;
                    }
                }
            });
        }
    });
    Ok(addr)
}

async fn read_within(stream: &mut TcpStream, limit: Duration) -> Option<Vec<u8>> {
    let mut buf = [0u8; 1024];
    match tokio::time::timeout(limit, stream.read(&mut buf)).await {
        Ok(Ok(n)) if n > 0 => Some(buf[..n].to_vec()),
        _ => None,
    }
}

/// @covers: FreezableProxy::start
#[tokio::test]
async fn test_start_forwards_bytes_to_the_upstream_and_back() -> Result<(), Box<dyn Error>> {
    let proxy = FreezableProxy::start(start_echo().await?).await?;
    let mut client = TcpStream::connect(proxy.get_addr()).await?;

    client.write_all(b"hello").await?;

    assert_eq!(
        read_within(&mut client, Duration::from_secs(2)).await,
        Some(b"hello".to_vec())
    );
    Ok(())
}

/// @covers: RunningProxy::freeze
#[tokio::test]
async fn test_freeze_stops_answers_but_keeps_the_connection_open() -> Result<(), Box<dyn Error>> {
    let proxy = FreezableProxy::start(start_echo().await?).await?;
    let mut client = TcpStream::connect(proxy.get_addr()).await?;
    client.write_all(b"before").await?;
    assert!(read_within(&mut client, Duration::from_secs(2))
        .await
        .is_some());

    proxy.freeze();
    client.write_all(b"after").await?;

    assert_eq!(
        read_within(&mut client, Duration::from_millis(400)).await,
        None,
        "a frozen proxy must not answer"
    );
    assert_eq!(
        proxy.get_closed_by_client(),
        0,
        "freezing must not close the connection"
    );
    Ok(())
}

/// @covers: RunningProxy::wait_for_client_close
#[tokio::test]
async fn test_wait_for_client_close_client_that_drops_its_connection_is_counted(
) -> Result<(), Box<dyn Error>> {
    let proxy = FreezableProxy::start(start_echo().await?).await?;
    let client = TcpStream::connect(proxy.get_addr()).await?;

    drop(client);

    assert!(proxy.wait_for_client_close(Duration::from_secs(2)).await);
    assert_eq!(proxy.get_closed_by_client(), 1);
    Ok(())
}

/// @covers: RunningProxy::wait_for_client_close
#[tokio::test]
async fn test_wait_for_client_close_connection_still_open_returns_false_after_the_limit(
) -> Result<(), Box<dyn Error>> {
    let proxy = FreezableProxy::start(start_echo().await?).await?;
    let _client = TcpStream::connect(proxy.get_addr()).await?;

    assert!(
        !proxy
            .wait_for_client_close(Duration::from_millis(300))
            .await
    );
    Ok(())
}

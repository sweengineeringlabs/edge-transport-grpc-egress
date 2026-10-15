//! Tests of `Fixture` and `ClosedPort`: the port refuses connections, nobody else can get it while
//! it is held, and it is released when the value is dropped.

use std::error::Error;
use std::io::ErrorKind;

use grpc_egress_test_support::Fixture;
use tokio::net::{TcpListener, TcpStream};

/// @covers: Fixture::reserve_closed_port
#[tokio::test]
async fn test_reserve_closed_port_nothing_listens_refuses_the_connection(
) -> Result<(), Box<dyn Error>> {
    let closed = Fixture::reserve_closed_port()?;

    let attempt = TcpStream::connect(("127.0.0.1", closed.get_port())).await;

    let error = attempt
        .err()
        .ok_or("the connection to the closed port succeeded")?;
    assert_eq!(error.kind(), ErrorKind::ConnectionRefused);
    Ok(())
}

/// @covers: Fixture::reserve_closed_port
#[tokio::test]
async fn test_reserve_closed_port_held_cannot_be_bound_by_a_listener() -> Result<(), Box<dyn Error>>
{
    let closed = Fixture::reserve_closed_port()?;

    let attempt = TcpListener::bind(("127.0.0.1", closed.get_port())).await;

    let error = attempt
        .err()
        .ok_or("a listener took the port the fixture holds")?;
    assert_eq!(error.kind(), ErrorKind::AddrInUse);
    Ok(())
}

/// @covers: Fixture::reserve_closed_port
#[tokio::test]
async fn test_reserve_closed_port_dropped_releases_the_port() -> Result<(), Box<dyn Error>> {
    let closed = Fixture::reserve_closed_port()?;
    let port = closed.get_port();

    drop(closed);

    let listener = TcpListener::bind(("127.0.0.1", port)).await?;
    assert_eq!(listener.local_addr()?.port(), port);
    Ok(())
}

/// @covers: Fixture::reserve_closed_port
#[tokio::test]
async fn test_reserve_closed_port_two_reservations_get_different_ports(
) -> Result<(), Box<dyn Error>> {
    let first = Fixture::reserve_closed_port()?;
    let second = Fixture::reserve_closed_port()?;

    assert_ne!(first.get_port(), second.get_port());
    Ok(())
}

/// @covers: ClosedPort::get_endpoint
#[tokio::test]
async fn test_get_endpoint_is_an_http_url_of_the_reserved_port() -> Result<(), Box<dyn Error>> {
    let closed = Fixture::reserve_closed_port()?;

    assert_eq!(
        closed.get_endpoint(),
        format!("http://127.0.0.1:{}", closed.get_port())
    );
    Ok(())
}

/// @covers: Fixture::ensure_tls_provider
#[test]
fn test_ensure_tls_provider_called_twice_does_not_panic() {
    Fixture::ensure_tls_provider();
    Fixture::ensure_tls_provider();

    assert!(
        rustls::crypto::CryptoProvider::get_default().is_some(),
        "a default provider must be installed"
    );
}

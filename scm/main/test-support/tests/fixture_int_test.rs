//! Tests of `Fixture` and `ClosedPort`: the port refuses connections, nobody else can get it while
//! it is held, and it is released when the value is dropped.

use std::error::Error;
use std::io::ErrorKind;

use grpc_egress_test_support::Fixture;
use socket2::{Domain, Protocol, Socket, Type};
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
fn test_ensure_tls_provider_called_twice_installs_the_aws_lc_rs_provider() {
    Fixture::ensure_tls_provider();
    Fixture::ensure_tls_provider();

    let installed = rustls::crypto::CryptoProvider::get_default();

    let expected = rustls::crypto::aws_lc_rs::default_provider();
    let installed = installed.map(|provider| provider.cipher_suites.len());
    assert_eq!(installed, Some(expected.cipher_suites.len()));
}

/// A socket that explicitly sets `SO_REUSEADDR` cannot take the held port either: Windows refuses
/// it with `PermissionDenied` (WSAEACCES), other platforms with `AddrInUse`.
///
/// @covers: Fixture::reserve_closed_port
#[tokio::test]
async fn test_reserve_closed_port_held_cannot_be_taken_even_by_a_socket_that_reuses_addresses(
) -> Result<(), Box<dyn Error>> {
    let closed = Fixture::reserve_closed_port()?;
    let other = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;
    other.set_reuse_address(true)?;
    let held: std::net::SocketAddr = ([127, 0, 0, 1], closed.get_port()).into();

    let attempt = other.bind(&held.into());

    let error = attempt
        .err()
        .ok_or("a socket with address reuse took the held port")?;
    assert!(
        matches!(
            error.kind(),
            ErrorKind::AddrInUse | ErrorKind::PermissionDenied
        ),
        "expected the port to be refused, got {error:?}"
    );
    Ok(())
}

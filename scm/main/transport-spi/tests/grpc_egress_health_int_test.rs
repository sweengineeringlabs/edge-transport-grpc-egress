//! The `grpc.health.v1.Health/Check` call (`HealthCheck`), against a real `tonic-health` server.
//!
//! The `health_check` of the old client opened a TCP connection and said nothing about the service; this asks it.

use std::error::Error;
use std::net::SocketAddr;
use std::time::Duration;

use grpc_egress_test_support::{ClosedPort, Fixture};
use grpc_egress_transport::{
    BuildGrpcEgressBuilderRequest, GrpcEgressBuilder, GrpcEgressBuilderFactory, HealthCheck,
    HealthCheckRequest, HealthStatus,
};
use grpc_egress_transport_spi::TonicGrpcEgressBuilderSpiFactory;
use proto_pattern::{CallError, CallOptions, CallRequest, Metadata, StatusCode, Transport};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Server;
use tonic_health::server::HealthReporter;
use tonic_health::ServingStatus;

const SHORT: Duration = Duration::from_secs(5);

/// How long a test waits for a call that should have ended: a call that does not end fails the test, it does not hang it.
const LIMIT: Duration = Duration::from_secs(30);

async fn bounded<T>(
    call: impl std::future::Future<Output = Result<T, CallError>>,
) -> Result<Result<T, CallError>, Box<dyn Error>> {
    Ok(tokio::time::timeout(LIMIT, call)
        .await
        .map_err(|_| "the call did not end within the limit")?)
}

/// A running health service: the address, the reporter that sets the statuses, and the sender that stops it.
struct HealthServer {
    address: SocketAddr,
    reporter: HealthReporter,
    stop: Option<oneshot::Sender<()>>,
}

impl HealthServer {
    async fn start() -> Result<Self, Box<dyn Error>> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let (reporter, service) = tonic_health::server::health_reporter();
        let (stop, stopped) = oneshot::channel::<()>();
        tokio::spawn(async move {
            let _ = Server::builder()
                .add_service(service)
                .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
                    let _ = stopped.await;
                })
                .await;
        });
        Ok(Self {
            address,
            reporter,
            stop: Some(stop),
        })
    }

    fn get_endpoint(&self) -> String {
        format!("http://{}", self.address)
    }
}

impl Drop for HealthServer {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}

fn client_for(endpoint: String) -> Result<impl Transport, Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    Ok(TonicGrpcEgressBuilderSpiFactory::new(endpoint)
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .build()?)
}

fn ask(service: &str) -> CallRequest<HealthCheckRequest> {
    CallRequest::new(
        HealthCheckRequest {
            service: service.to_string(),
        },
        CallOptions::create(Some(SHORT), Metadata::default()),
    )
}

/// @covers: HealthCheck
#[tokio::test]
async fn test_call_health_check_server_that_is_serving_returns_serving(
) -> Result<(), Box<dyn Error>> {
    let server = HealthServer::start().await?;
    let client = client_for(server.get_endpoint())?;

    let response = bounded(client.call::<HealthCheck>(ask(""))).await??;

    assert_eq!(response.get_message().status(), HealthStatus::Serving);
    Ok(())
}

/// @covers: HealthCheck
#[tokio::test]
async fn test_call_health_check_after_the_server_reports_not_serving_returns_not_serving(
) -> Result<(), Box<dyn Error>> {
    let server = HealthServer::start().await?;
    server
        .reporter
        .set_service_status("", ServingStatus::NotServing)
        .await;
    let client = client_for(server.get_endpoint())?;

    let response = bounded(client.call::<HealthCheck>(ask(""))).await??;

    assert_eq!(response.get_message().status(), HealthStatus::NotServing);
    Ok(())
}

/// @covers: HealthCheck
#[tokio::test]
async fn test_call_health_check_named_service_reports_its_own_status_apart_from_the_server(
) -> Result<(), Box<dyn Error>> {
    let server = HealthServer::start().await?;
    server
        .reporter
        .set_service_status("pkg.Down", ServingStatus::NotServing)
        .await;
    server
        .reporter
        .set_service_status("pkg.Up", ServingStatus::Serving)
        .await;
    let client = client_for(server.get_endpoint())?;

    let down = bounded(client.call::<HealthCheck>(ask("pkg.Down"))).await??;
    let up = bounded(client.call::<HealthCheck>(ask("pkg.Up"))).await??;
    let whole = bounded(client.call::<HealthCheck>(ask(""))).await??;

    assert_eq!(down.get_message().status(), HealthStatus::NotServing);
    assert_eq!(up.get_message().status(), HealthStatus::Serving);
    assert_eq!(whole.get_message().status(), HealthStatus::Serving);
    Ok(())
}

/// @covers: HealthCheck
#[tokio::test]
async fn test_call_health_check_unknown_service_returns_status_not_found(
) -> Result<(), Box<dyn Error>> {
    let server = HealthServer::start().await?;
    let client = client_for(server.get_endpoint())?;

    let error = bounded(client.call::<HealthCheck>(ask("pkg.Nobody")))
        .await?
        .err()
        .ok_or("a service nobody registered has no health")?;

    assert!(
        matches!(
            &error,
            CallError::Status {
                code: StatusCode::NotFound,
                ..
            }
        ),
        "{error:?}"
    );
    Ok(())
}

/// @covers: HealthCheck
#[tokio::test]
async fn test_call_health_check_to_a_refused_connection_returns_transport_unavailable(
) -> Result<(), Box<dyn Error>> {
    let closed = ClosedPort::reserve()?;
    let client = client_for(closed.get_endpoint())?;

    let error = bounded(client.call::<HealthCheck>(ask("")))
        .await?
        .err()
        .ok_or("nothing listens on the port")?;

    assert!(
        matches!(
            &error,
            CallError::Transport { code: StatusCode::Unavailable, path, .. }
                if path.as_ref() == "/grpc.health.v1.Health/Check"
        ),
        "{error:?}"
    );
    Ok(())
}

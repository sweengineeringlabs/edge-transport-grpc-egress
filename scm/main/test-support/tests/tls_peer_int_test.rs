//! Tests of the TLS peer and of `TestPki`: a peer that proves itself, one that also demands a client
//! certificate, and a client that trusts the wrong authority. The requests are made with a raw
//! TLS and HTTP/2 client, not with the client under test.

use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Bytes;
use grpc_egress_test_support::{
    Fixture, GrpcFrame, PeerBehaviour, PeerServer, TestPki, TlsServerMode,
};
use http_body_util::{BodyExt as _, Full};
use hyper::client::conn::http2::handshake;
use hyper_util::rt::{TokioExecutor, TokioIo};
use rustls::pki_types::pem::PemObject as _;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

/// What a raw client presents and trusts.
struct RawTlsClient {
    trusted_ca: Vec<CertificateDer<'static>>,
    identity: Option<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)>,
}

impl RawTlsClient {
    fn trusting(pki: &TestPki) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            trusted_ca: CertificateDer::pem_file_iter(pki.get_ca_pem_path())?
                .collect::<Result<_, _>>()?,
            identity: None,
        })
    }

    fn presenting(mut self, pki: &TestPki) -> Result<Self, Box<dyn Error>> {
        let chain = CertificateDer::pem_file_iter(pki.get_client_cert_pem_path())?
            .collect::<Result<_, _>>()?;
        let key = PrivateKeyDer::from_pem_file(pki.get_client_key_pem_path())?;
        self.identity = Some((chain, key));
        Ok(self)
    }

    async fn post(&self, addr: SocketAddr) -> Result<Bytes, Box<dyn Error>> {
        Fixture::ensure_tls_provider();
        let mut roots = rustls::RootCertStore::empty();
        for ca in &self.trusted_ca {
            roots.add(ca.clone())?;
        }
        let builder = rustls::ClientConfig::builder().with_root_certificates(roots);
        let mut config = match &self.identity {
            Some((chain, key)) => builder.with_client_auth_cert(chain.clone(), key.clone_key())?,
            None => builder.with_no_client_auth(),
        };
        config.alpn_protocols = vec![b"h2".to_vec()];

        let tcp = TcpStream::connect(addr).await?;
        let tls = TlsConnector::from(Arc::new(config))
            .connect(ServerName::try_from("localhost")?, tcp)
            .await?;
        let (mut sender, connection) = handshake(TokioExecutor::new(), TokioIo::new(tls)).await?;
        tokio::spawn(async move {
            let _ = connection.await;
        });
        let request = http::Request::builder()
            .method("POST")
            .uri(format!(
                "https://localhost:{}/pkg.Service/Method",
                addr.port()
            ))
            .header("content-type", "application/grpc")
            .body(Full::new(GrpcFrame::encode(b"ping")))?;
        let response = sender.send_request(request).await?;
        Ok(response.into_body().collect().await?.to_bytes())
    }
}

fn reply_peer() -> PeerBehaviour {
    PeerBehaviour::Reply(b"secure".to_vec())
}

/// @covers: PeerServer::start_tls
#[tokio::test]
async fn test_start_tls_server_auth_only_answers_a_client_that_trusts_the_authority(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let peer = PeerServer::start_tls(reply_peer(), &pki, TlsServerMode::ServerAuthOnly).await?;

    let body = RawTlsClient::trusting(&pki)?.post(peer.get_addr()).await?;

    assert_eq!(GrpcFrame::decode_all(&body)[0].get_payload(), b"secure");
    Ok(())
}

/// @covers: PeerServer::start_tls
#[tokio::test]
async fn test_start_tls_client_that_trusts_another_authority_fails_the_handshake(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let other = TestPki::create()?;
    let peer = PeerServer::start_tls(reply_peer(), &pki, TlsServerMode::ServerAuthOnly).await?;

    let result = RawTlsClient::trusting(&other)?.post(peer.get_addr()).await;

    assert!(
        result.is_err(),
        "a client trusting another authority must refuse the peer"
    );
    Ok(())
}

/// @covers: PeerServer::start_tls
#[tokio::test]
async fn test_start_tls_client_auth_required_answers_a_client_presenting_its_certificate(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let peer = PeerServer::start_tls(reply_peer(), &pki, TlsServerMode::ClientAuthRequired).await?;

    let body = RawTlsClient::trusting(&pki)?
        .presenting(&pki)?
        .post(peer.get_addr())
        .await?;

    assert_eq!(GrpcFrame::decode_all(&body)[0].get_payload(), b"secure");
    Ok(())
}

/// @covers: PeerServer::start_tls
#[tokio::test]
async fn test_start_tls_client_auth_required_refuses_a_client_without_a_certificate(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let peer = PeerServer::start_tls(reply_peer(), &pki, TlsServerMode::ClientAuthRequired).await?;

    let result = RawTlsClient::trusting(&pki)?.post(peer.get_addr()).await;

    assert!(
        result.is_err(),
        "a peer demanding a client certificate must refuse a client without one"
    );
    assert!(
        peer.get_recorded_requests().is_empty(),
        "the refused client must not have reached the service"
    );
    Ok(())
}

/// @covers: PeerServer::start_tls
#[tokio::test]
async fn test_start_tls_client_auth_required_refuses_a_certificate_of_another_authority(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let other = TestPki::create()?;
    let peer = PeerServer::start_tls(reply_peer(), &pki, TlsServerMode::ClientAuthRequired).await?;

    let result = RawTlsClient::trusting(&pki)?
        .presenting(&other)?
        .post(peer.get_addr())
        .await;

    assert!(
        result.is_err(),
        "a certificate of another authority must be refused"
    );
    assert!(peer.get_recorded_requests().is_empty());
    Ok(())
}

/// @covers: TestPki::create
#[test]
fn test_create_writes_every_pem_file_the_code_under_test_reads() -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;

    for name in [
        "ca.pem",
        "server.pem",
        "server.key",
        "client.pem",
        "client.key",
    ] {
        let text = std::fs::read_to_string(pki.get_dir().join(name))?;
        assert!(
            text.starts_with("-----BEGIN "),
            "{name} is not PEM: {text:.40}"
        );
    }
    Ok(())
}

/// @covers: TestPki::create
#[test]
fn test_create_twice_gives_different_authorities() -> Result<(), Box<dyn Error>> {
    let first = std::fs::read_to_string(TestPki::create()?.get_ca_pem_path())?;
    let second = std::fs::read_to_string(TestPki::create()?.get_ca_pem_path())?;

    assert_ne!(first, second, "each test needs its own authority");
    Ok(())
}

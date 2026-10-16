//! Tests of the TLS options of the `hyper`-backed client against a real in-process TLS peer: the
//! client certificate is presented, the configured root CA is the only one trusted, and a file that
//! cannot be used fails when the client is built, naming its path.

use grpc_egress_test_support::ClientFixture;
use grpc_egress_transport_std::{GrpcChannelConfigFactory, GrpcRequestFactory, MtlsConfigFactory};
use std::error::Error;
use std::time::Duration;

use grpc_egress_test_support::{
    Fixture, PeerBehaviour, PeerServer, RunningPeer, TestPki, TlsServerMode,
};
use grpc_egress_transport::{
    BuildGrpcEgressBuilderRequest, GrpcChannelConfig, GrpcChannelConfigError, GrpcEgress,
    GrpcEgressBuilder, GrpcEgressBuilderFactory, GrpcEgressError, GrpcRequest, MtlsConfig,
};
use grpc_egress_transport_std::TonicGrpcEgressBuilderSpiFactory;

fn path_of(path: std::path::PathBuf) -> String {
    path.to_string_lossy().into_owned()
}

async fn start_peer(pki: &TestPki, mode: TlsServerMode) -> Result<RunningPeer, Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    Ok(PeerServer::start_tls(PeerBehaviour::Reply(b"secure".to_vec()), pki, mode).await?)
}

async fn call(client: &impl GrpcEgress) -> Result<Vec<u8>, GrpcEgressError> {
    client
        .call_unary(GrpcRequest::new(
            "pkg.Service/Method",
            b"ping".to_vec(),
            Duration::from_secs(5),
        ))
        .await
        .map(|response| response.body)
}

fn builder_for(peer: &RunningPeer) -> impl GrpcEgressBuilder {
    TonicGrpcEgressBuilderSpiFactory::new(peer.get_https_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
}

/// @covers: GrpcEgressBuilder::with_mtls
#[tokio::test]
async fn test_with_mtls_peer_demanding_a_client_certificate_answers_the_client_happy(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let peer = start_peer(&pki, TlsServerMode::ClientAuthRequired).await?;
    let client = builder_for(&peer)
        .with_mtls(MtlsConfig::with_root_ca(
            path_of(pki.get_client_cert_pem_path()),
            path_of(pki.get_client_key_pem_path()),
            path_of(pki.get_ca_pem_path()),
        ))
        .build()?;

    assert_eq!(call(&client).await?, b"secure");
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_root_ca
#[tokio::test]
async fn test_with_root_ca_peer_demanding_a_client_certificate_refuses_a_client_without_one_error(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let peer = start_peer(&pki, TlsServerMode::ClientAuthRequired).await?;
    let client = builder_for(&peer)
        .with_root_ca(path_of(pki.get_ca_pem_path()))
        .build()?;

    let result = call(&client).await;

    assert!(
        matches!(result, Err(GrpcEgressError::ConnectionFailed(_))),
        "a client without a certificate must be refused, got {result:?}"
    );
    assert!(peer.get_recorded_requests().is_empty());
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_root_ca
#[tokio::test]
async fn test_with_root_ca_peer_proving_itself_with_that_authority_answers_the_client_happy(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let peer = start_peer(&pki, TlsServerMode::ServerAuthOnly).await?;
    let client = builder_for(&peer)
        .with_root_ca(path_of(pki.get_ca_pem_path()))
        .build()?;

    assert_eq!(call(&client).await?, b"secure");
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_root_ca
#[tokio::test]
async fn test_with_root_ca_peer_proving_itself_with_another_authority_is_refused_error(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let other = TestPki::create()?;
    let peer = start_peer(&pki, TlsServerMode::ServerAuthOnly).await?;
    let client = builder_for(&peer)
        .with_root_ca(path_of(other.get_ca_pem_path()))
        .build()?;

    let result = call(&client).await;

    assert!(
        matches!(result, Err(GrpcEgressError::ConnectionFailed(_))),
        "a peer of another authority must be refused, got {result:?}"
    );
    Ok(())
}

/// Without a root CA the web PKI roots are used, and a throwaway authority is not among them.
///
/// @covers: GrpcEgressBuilder::build
#[tokio::test]
async fn test_build_without_a_root_ca_refuses_a_peer_of_an_unknown_authority_error(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let peer = start_peer(&pki, TlsServerMode::ServerAuthOnly).await?;
    let client = builder_for(&peer).build()?;

    let result = call(&client).await;

    assert!(
        matches!(result, Err(GrpcEgressError::ConnectionFailed(_))),
        "got {result:?}"
    );
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_mtls
#[tokio::test]
async fn test_with_mtls_certificate_file_that_does_not_exist_fails_the_build_naming_the_path_error(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let peer = start_peer(&pki, TlsServerMode::ClientAuthRequired).await?;
    let missing = path_of(pki.get_dir().join("no-such-client.pem"));

    let result = builder_for(&peer)
        .with_mtls(MtlsConfig::new(
            missing.clone(),
            path_of(pki.get_client_key_pem_path()),
        ))
        .build();

    match result {
        Err(GrpcChannelConfigError::UnusableTlsFile { path, .. }) => assert_eq!(path, missing),
        other => panic!(
            "expected UnusableTlsFile naming {missing}, got {:?}",
            other.err()
        ),
    }
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_mtls
#[tokio::test]
async fn test_with_mtls_key_file_without_a_key_fails_the_build_naming_the_path_error(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let peer = start_peer(&pki, TlsServerMode::ClientAuthRequired).await?;
    // The certificate file holds no private key.
    let not_a_key = path_of(pki.get_client_cert_pem_path());

    let result = builder_for(&peer)
        .with_mtls(MtlsConfig::new(
            path_of(pki.get_client_cert_pem_path()),
            not_a_key.clone(),
        ))
        .build();

    match result {
        Err(GrpcChannelConfigError::UnusableTlsFile { path, .. }) => assert_eq!(path, not_a_key),
        other => panic!(
            "expected UnusableTlsFile naming {not_a_key}, got {:?}",
            other.err()
        ),
    }
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_root_ca
#[tokio::test]
async fn test_with_root_ca_file_without_a_certificate_fails_the_build_error(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let peer = start_peer(&pki, TlsServerMode::ServerAuthOnly).await?;
    let empty = pki.get_dir().join("empty-ca.pem");
    std::fs::write(&empty, "")?;
    let empty = path_of(empty);

    let result = builder_for(&peer).with_root_ca(empty.clone()).build();

    match result {
        Err(GrpcChannelConfigError::UnusableTlsFile { path, .. }) => assert_eq!(path, empty),
        other => panic!(
            "expected UnusableTlsFile naming {empty}, got {:?}",
            other.err()
        ),
    }
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_root_ca
#[tokio::test]
async fn test_with_root_ca_on_a_plaintext_endpoint_fails_the_build_error(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;

    let result = TonicGrpcEgressBuilderSpiFactory::new("http://127.0.0.1:1")
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_root_ca(path_of(pki.get_ca_pem_path()))
        .build();

    assert!(
        matches!(
            result,
            Err(GrpcChannelConfigError::TlsOptionOnPlaintextEndpoint {
                option: "root_ca",
                ..
            })
        ),
        "a root CA on a plaintext endpoint would be ignored, got {:?}",
        result.err()
    );
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_channel_config
#[tokio::test]
async fn test_with_channel_config_mtls_is_applied_to_the_client_happy() -> Result<(), Box<dyn Error>>
{
    let pki = TestPki::create()?;
    let peer = start_peer(&pki, TlsServerMode::ClientAuthRequired).await?;
    let config =
        GrpcChannelConfig::new(peer.get_https_endpoint()).with_mtls(MtlsConfig::with_root_ca(
            path_of(pki.get_client_cert_pem_path()),
            path_of(pki.get_client_key_pem_path()),
            path_of(pki.get_ca_pem_path()),
        ));

    let client = builder_for(&peer).with_channel_config(&config).build()?;

    assert_eq!(call(&client).await?, b"secure");
    Ok(())
}

/// The issue #13 defect: the configuration's mtls was stored and never read by the client built
/// from it.
///
/// @covers: TransportConstruction::create_transport_from_config
#[tokio::test]
async fn test_create_transport_from_config_mtls_is_applied_to_the_client_happy(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let peer = start_peer(&pki, TlsServerMode::ClientAuthRequired).await?;
    let config =
        GrpcChannelConfig::new(peer.get_https_endpoint()).with_mtls(MtlsConfig::with_root_ca(
            path_of(pki.get_client_cert_pem_path()),
            path_of(pki.get_client_key_pem_path()),
            path_of(pki.get_ca_pem_path()),
        ));
    let client = ClientFixture::from_config(&config)?;

    let response = client
        .call_unary(GrpcRequest::new(
            "pkg.Service/Method",
            b"ping".to_vec(),
            Duration::from_secs(5),
        ))
        .await?;

    assert_eq!(response.body, b"secure");
    Ok(())
}

/// @covers: TransportConstruction::create_transport_from_config
#[tokio::test]
async fn test_create_transport_from_config_unusable_mtls_file_fails_at_construction_error(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let missing = path_of(pki.get_dir().join("gone.pem"));
    let config = GrpcChannelConfig::new("https://127.0.0.1:1").with_mtls(MtlsConfig::new(
        missing.clone(),
        path_of(pki.get_client_key_pem_path()),
    ));

    let result = ClientFixture::from_config(&config);

    match result {
        Err(GrpcChannelConfigError::UnusableTlsFile { path, .. }) => assert_eq!(path, missing),
        other => panic!("expected UnusableTlsFile, got {:?}", other.err()),
    }
    Ok(())
}

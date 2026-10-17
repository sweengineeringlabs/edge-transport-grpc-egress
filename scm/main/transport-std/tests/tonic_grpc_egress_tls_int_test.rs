//! Tests of the TLS options of the client over the `proto-svc` engine against a real in-process TLS peer: the
//! client certificate is presented, the configured root CA is the only one trusted, the server name is checked,
//! and a file that cannot be used fails when the client is built, naming its path.

use std::error::Error;
use std::time::Duration;

use grpc_egress_test_support::{
    CallFixture, ClientFixture, Fixture, PeerBehaviour, PeerServer, RunningPeer, TestPki,
    TlsServerMode,
};
use grpc_egress_transport::{
    BuildGrpcEgressBuilderRequest, GrpcChannelConfig, GrpcChannelConfigError, GrpcEgressBuilder,
    GrpcEgressBuilderFactory, MtlsConfig,
};
use grpc_egress_transport_std::{
    GrpcChannelConfigFactory, MtlsConfigFactory, TlsSettingsResolver,
    TonicGrpcEgressBuilderSpiFactory,
};
use proto_pattern::{CallError, RawTransport, TlsIdentity, TlsSettings};

fn path_of(path: std::path::PathBuf) -> String {
    path.to_string_lossy().into_owned()
}

fn read(path: std::path::PathBuf) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(std::fs::read(path)?)
}

async fn start_peer(pki: &TestPki, mode: TlsServerMode) -> Result<RunningPeer, Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    Ok(PeerServer::start_tls(PeerBehaviour::Reply(b"secure".to_vec()), pki, mode).await?)
}

/// What the client under test makes: one raw call, and the bytes of the answer.
async fn call(client: &impl RawTransport) -> Result<Vec<u8>, CallError> {
    client
        .call_raw(
            CallFixture::path("pkg.Service/Method"),
            CallFixture::request(b"ping", Duration::from_secs(5)),
        )
        .await
        .map(|response| response.into_message())
}

fn builder_for(peer: &RunningPeer) -> impl GrpcEgressBuilder {
    TonicGrpcEgressBuilderSpiFactory::new(peer.get_https_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
}

/// The client identity the test authority issued, as bytes.
fn identity_of(pki: &TestPki) -> Result<TlsIdentity, Box<dyn Error>> {
    Ok(TlsIdentity::create(
        read(pki.get_client_cert_pem_path())?,
        read(pki.get_client_key_pem_path())?,
    ))
}

/// The paths of the client identity and the authority, as a configuration file names them.
fn mtls_files_of(pki: &TestPki) -> MtlsConfig {
    MtlsConfig::with_root_ca(
        path_of(pki.get_client_cert_pem_path()),
        path_of(pki.get_client_key_pem_path()),
        path_of(pki.get_ca_pem_path()),
    )
}

/// `true` when `error` is a failure of the transport, not an answer of the peer.
fn is_transport_failure<T>(result: &Result<T, CallError>) -> bool {
    matches!(result, Err(CallError::Transport { .. }))
}

// ---------------------------------------------------------------- the settings, as bytes

/// @covers: GrpcEgressBuilder::with_tls
#[tokio::test]
async fn test_with_tls_identity_peer_demanding_a_client_certificate_answers_the_client_happy(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let peer = start_peer(&pki, TlsServerMode::ClientAuthRequired).await?;
    let client = builder_for(&peer)
        .with_tls(TlsSettings::create(
            None,
            vec![read(pki.get_ca_pem_path())?],
            Some(identity_of(&pki)?),
        ))
        .build()?;

    assert_eq!(call(&client).await?, b"secure");
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_tls
#[tokio::test]
async fn test_with_tls_roots_without_an_identity_peer_demanding_a_client_certificate_refuses_error(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let peer = start_peer(&pki, TlsServerMode::ClientAuthRequired).await?;
    let client = builder_for(&peer)
        .with_tls(TlsSettings::create(
            None,
            vec![read(pki.get_ca_pem_path())?],
            None,
        ))
        .build()?;

    let result = call(&client).await;

    assert!(
        is_transport_failure(&result),
        "a client without a certificate must be refused, got {result:?}"
    );
    assert!(
        peer.get_recorded_requests().is_empty(),
        "a refused client must not reach the service"
    );
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_tls
#[tokio::test]
async fn test_with_tls_pinned_ca_peer_proving_itself_with_that_authority_answers_the_client_happy(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let peer = start_peer(&pki, TlsServerMode::ServerAuthOnly).await?;
    let client = builder_for(&peer)
        .with_tls(TlsSettings::create(
            None,
            vec![read(pki.get_ca_pem_path())?],
            None,
        ))
        .build()?;

    assert_eq!(call(&client).await?, b"secure");
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_tls
#[tokio::test]
async fn test_with_tls_pinned_ca_peer_proving_itself_with_another_authority_is_refused_error(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let other = TestPki::create()?;
    let peer = start_peer(&pki, TlsServerMode::ServerAuthOnly).await?;
    let client = builder_for(&peer)
        .with_tls(TlsSettings::create(
            None,
            vec![read(other.get_ca_pem_path())?],
            None,
        ))
        .build()?;

    let result = call(&client).await;

    assert!(
        is_transport_failure(&result),
        "a peer of another authority must be refused, got {result:?}"
    );
    assert!(peer.get_recorded_requests().is_empty());
    Ok(())
}

/// Without roots the platform's are used, and a throwaway authority is not among them.
///
/// @covers: GrpcEgressBuilder::build
#[tokio::test]
async fn test_build_without_tls_settings_refuses_a_peer_of_an_unknown_authority_error(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let peer = start_peer(&pki, TlsServerMode::ServerAuthOnly).await?;
    let client = builder_for(&peer).build()?;

    let result = call(&client).await;

    assert!(is_transport_failure(&result), "got {result:?}");
    Ok(())
}

/// The certificate of the test peer is valid for `localhost` and `127.0.0.1`.
///
/// @covers: GrpcEgressBuilder::with_tls
#[tokio::test]
async fn test_with_tls_server_name_the_certificate_is_valid_for_answers_the_client_happy(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let peer = start_peer(&pki, TlsServerMode::ServerAuthOnly).await?;
    let client = builder_for(&peer)
        .with_tls(TlsSettings::create(
            Some("localhost".to_string()),
            vec![read(pki.get_ca_pem_path())?],
            None,
        ))
        .build()?;

    assert_eq!(call(&client).await?, b"secure");
    Ok(())
}

/// The endpoint is `127.0.0.1`, which the certificate is valid for: the client would accept the peer if the
/// server name of the settings were ignored, so only a client that checks that name fails here.
///
/// @covers: GrpcEgressBuilder::with_tls
#[tokio::test]
async fn test_with_tls_server_name_the_certificate_is_not_valid_for_is_refused_error(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let peer = start_peer(&pki, TlsServerMode::ServerAuthOnly).await?;
    let client = builder_for(&peer)
        .with_tls(TlsSettings::create(
            Some("wrong.example".to_string()),
            vec![read(pki.get_ca_pem_path())?],
            None,
        ))
        .build()?;

    let result = call(&client).await;

    assert!(
        is_transport_failure(&result),
        "a peer whose certificate is not valid for the name asked for must be refused, got {result:?}"
    );
    assert!(peer.get_recorded_requests().is_empty());
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_tls
#[tokio::test]
async fn test_with_tls_on_a_plaintext_endpoint_fails_the_build_error() -> Result<(), Box<dyn Error>>
{
    let pki = TestPki::create()?;

    let result = TonicGrpcEgressBuilderSpiFactory::new("http://127.0.0.1:1")
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_tls(TlsSettings::create(
            None,
            vec![read(pki.get_ca_pem_path())?],
            None,
        ))
        .build();

    assert!(
        matches!(
            result,
            Err(GrpcChannelConfigError::TlsOptionOnPlaintextEndpoint { option: "tls", .. })
        ),
        "TLS settings on a plaintext endpoint would be ignored, got {:?}",
        result.err()
    );
    Ok(())
}

// ---------------------------------------------------------------- the files of a configuration

/// The issue #13 defect: the configuration's mtls was stored and never read by the client built from it.
///
/// @covers: GrpcEgressBuilder::with_channel_config
#[tokio::test]
async fn test_with_channel_config_mtls_files_are_read_and_the_identity_is_presented_happy(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let peer = start_peer(&pki, TlsServerMode::ClientAuthRequired).await?;
    let config = GrpcChannelConfig::new(peer.get_https_endpoint()).with_mtls(mtls_files_of(&pki));

    let client = builder_for(&peer).with_channel_config(&config).build()?;

    assert_eq!(call(&client).await?, b"secure");
    Ok(())
}

/// @covers: ClientFixture::from_config
#[tokio::test]
async fn test_from_config_mtls_files_are_applied_to_the_client_the_caller_gets_happy(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let peer = start_peer(&pki, TlsServerMode::ClientAuthRequired).await?;
    let config = GrpcChannelConfig::new(peer.get_https_endpoint()).with_mtls(mtls_files_of(&pki));
    let client = ClientFixture::from_config(&config)?;

    assert_eq!(call(&client).await?, b"secure");
    Ok(())
}

/// The CA named in the file configuration is the only authority trusted.
///
/// @covers: GrpcEgressBuilder::with_channel_config
#[tokio::test]
async fn test_with_channel_config_mtls_ca_file_of_another_authority_refuses_the_peer_error(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let other = TestPki::create()?;
    let peer = start_peer(&pki, TlsServerMode::ClientAuthRequired).await?;
    let config =
        GrpcChannelConfig::new(peer.get_https_endpoint()).with_mtls(MtlsConfig::with_root_ca(
            path_of(pki.get_client_cert_pem_path()),
            path_of(pki.get_client_key_pem_path()),
            path_of(other.get_ca_pem_path()),
        ));
    let client = builder_for(&peer).with_channel_config(&config).build()?;

    let result = call(&client).await;

    assert!(is_transport_failure(&result), "got {result:?}");
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_channel_config
#[tokio::test]
async fn test_build_mtls_certificate_file_that_does_not_exist_fails_naming_the_path_error(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let missing = path_of(pki.get_dir().join("no-such-client.pem"));
    let config = GrpcChannelConfig::new("https://127.0.0.1:1").with_mtls(MtlsConfig::new(
        missing.clone(),
        path_of(pki.get_client_key_pem_path()),
    ));

    let result = ClientFixture::from_config(&config);

    match result {
        Err(GrpcChannelConfigError::UnusableTlsFile { path, .. }) => assert_eq!(path, missing),
        other => panic!(
            "expected UnusableTlsFile naming {missing}, got {:?}",
            other.err()
        ),
    }
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_channel_config
#[tokio::test]
async fn test_build_mtls_key_file_without_a_key_fails_naming_the_path_error(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    // The certificate file holds no private key.
    let not_a_key = path_of(pki.get_client_cert_pem_path());
    let config = GrpcChannelConfig::new("https://127.0.0.1:1").with_mtls(MtlsConfig::new(
        path_of(pki.get_client_cert_pem_path()),
        not_a_key.clone(),
    ));

    let result = ClientFixture::from_config(&config);

    match result {
        Err(GrpcChannelConfigError::UnusableTlsFile { path, .. }) => assert_eq!(path, not_a_key),
        other => panic!(
            "expected UnusableTlsFile naming {not_a_key}, got {:?}",
            other.err()
        ),
    }
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_channel_config
#[tokio::test]
async fn test_build_mtls_ca_file_without_a_certificate_fails_naming_the_path_error(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let empty = pki.get_dir().join("empty-ca.pem");
    std::fs::write(&empty, "")?;
    let empty = path_of(empty);
    let config = GrpcChannelConfig::new("https://127.0.0.1:1").with_mtls(MtlsConfig::with_root_ca(
        path_of(pki.get_client_cert_pem_path()),
        path_of(pki.get_client_key_pem_path()),
        empty.clone(),
    ));

    let result = ClientFixture::from_config(&config);

    match result {
        Err(GrpcChannelConfigError::UnusableTlsFile { path, .. }) => assert_eq!(path, empty),
        other => panic!(
            "expected UnusableTlsFile naming {empty}, got {:?}",
            other.err()
        ),
    }
    Ok(())
}

// ---------------------------------------------------------------- the resolver

/// @covers: TlsSettingsResolver::resolve
#[test]
fn test_resolve_identity_and_root_ca_reads_the_bytes_of_every_file_happy(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let mtls = mtls_files_of(&pki);

    let settings = TlsSettingsResolver::resolve(Some(&mtls), mtls.ca_pem_path.as_deref())?;

    assert_eq!(
        settings.get_trust_roots_pem(),
        [read(pki.get_ca_pem_path())?].as_slice()
    );
    let identity = settings.get_identity().ok_or("no identity")?;
    assert_eq!(
        identity.get_certificate_pem(),
        read(pki.get_client_cert_pem_path())?.as_slice()
    );
    assert_eq!(
        identity.get_private_key_pem(),
        read(pki.get_client_key_pem_path())?.as_slice()
    );
    assert_eq!(settings.get_server_name(), None);
    Ok(())
}

/// @covers: TlsSettingsResolver::resolve
#[test]
fn test_resolve_root_ca_alone_has_a_root_and_no_identity_edge() -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let ca = path_of(pki.get_ca_pem_path());

    let settings = TlsSettingsResolver::resolve(None, Some(&ca))?;

    assert_eq!(settings.get_trust_roots_pem().len(), 1);
    assert!(settings.get_identity().is_none());
    Ok(())
}

/// @covers: TlsSettingsResolver::resolve
#[test]
fn test_resolve_nothing_returns_the_system_roots_and_no_identity_edge() -> Result<(), Box<dyn Error>>
{
    let settings = TlsSettingsResolver::resolve(None, None)?;

    assert!(
        settings.get_trust_roots_pem().is_empty(),
        "no root means the platform's roots"
    );
    assert!(settings.get_identity().is_none());
    assert_eq!(settings.get_server_name(), None);
    Ok(())
}

/// @covers: TlsSettingsResolver::resolve
#[test]
fn test_resolve_root_ca_file_that_does_not_exist_returns_unusable_tls_file_naming_it_error(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let missing = path_of(pki.get_dir().join("gone.pem"));

    let result = TlsSettingsResolver::resolve(None, Some(&missing));

    match result {
        Err(GrpcChannelConfigError::UnusableTlsFile { path, .. }) => assert_eq!(path, missing),
        other => panic!("expected UnusableTlsFile, got {:?}", other.err()),
    }
    Ok(())
}

/// A file with text in it but no PEM item is as unusable as a missing one.
///
/// @covers: TlsSettingsResolver::resolve
#[test]
fn test_resolve_identity_key_file_with_text_but_no_key_returns_unusable_tls_file_error(
) -> Result<(), Box<dyn Error>> {
    let pki = TestPki::create()?;
    let text = pki.get_dir().join("not-a-key.pem");
    std::fs::write(&text, "this is not a key")?;
    let text = path_of(text);
    let mtls = MtlsConfig::new(path_of(pki.get_client_cert_pem_path()), text.clone());

    let result = TlsSettingsResolver::resolve(Some(&mtls), None);

    match result {
        Err(GrpcChannelConfigError::UnusableTlsFile { path, reason }) => {
            assert_eq!(path, text);
            assert!(reason.contains("no private key"), "reason: {reason}");
        }
        other => panic!("expected UnusableTlsFile, got {:?}", other.err()),
    }
    Ok(())
}

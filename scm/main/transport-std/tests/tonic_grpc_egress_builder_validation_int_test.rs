//! Tests of the combinations and values `build` refuses. Each error names the option, and a
//! conflict names both options, so that a caller knows what to change. Nothing panics.

use grpc_egress_transport::GrpcChannelConfigBuilder as _;
use std::error::Error;
use std::time::Duration;

use grpc_egress_test_support::TestPki;
use grpc_egress_transport::{
    BackendTarget, BalancingStrategy, BuildGrpcEgressBuilderRequest, GrpcChannelConfigError,
    GrpcEgressBuilder, GrpcEgressBuilderFactory, LoadbalancerSettings,
};
use grpc_egress_transport_std::TonicGrpcEgressBuilderSpiFactory;
use proto_pattern::{TlsIdentity, TlsSettings};

fn builder() -> impl GrpcEgressBuilder {
    TonicGrpcEgressBuilderSpiFactory::new("https://service.example:443")
        .build(BuildGrpcEgressBuilderRequest)
}

/// Settings that ask the client to present a certificate. The bytes are never read: the conflict is refused
/// before any TLS setting is used.
fn settings_with_identity() -> TlsSettings {
    TlsSettings::create(
        None,
        Vec::new(),
        Some(TlsIdentity::create(
            b"client certificate".to_vec(),
            b"client key".to_vec(),
        )),
    )
}

fn settings(urls: &[&str]) -> LoadbalancerSettings {
    LoadbalancerSettings::try_new(
        BalancingStrategy::RoundRobin,
        urls.iter()
            .map(|url| BackendTarget::try_new(*url, 1).unwrap_or_else(|e| panic!("{e}")))
            .collect(),
    )
    .unwrap_or_else(|e| panic!("{e}"))
}

fn describe<T>(result: Result<T, GrpcChannelConfigError>) -> String {
    match result {
        Ok(_) => "built".to_string(),
        Err(e) => e.to_string(),
    }
}

/// @covers: GrpcEgressBuilder::build
#[test]
fn test_build_tls_identity_with_plaintext_allowed_returns_conflict_naming_both_options_error() {
    let result = builder()
        .with_plaintext_allowed()
        .with_tls(settings_with_identity())
        .build();

    match result {
        Err(GrpcChannelConfigError::ConflictingOptions { first, second, .. }) => {
            assert_eq!((first, second), ("mtls", "plaintext_allowed"));
        }
        other => panic!("expected ConflictingOptions, got {}", describe(other)),
    }
}

/// @covers: GrpcChannelConfigError
#[test]
fn test_build_tls_identity_with_plaintext_allowed_message_contains_both_option_names_error() {
    let message = describe(
        builder()
            .with_plaintext_allowed()
            .with_tls(settings_with_identity())
            .build(),
    );

    assert!(
        message.contains("'mtls'") && message.contains("'plaintext_allowed'"),
        "the message must name both options, got: {message}"
    );
}

/// A root to trust with no client identity is not mutual TLS, so plaintext being allowed is no conflict for it
/// (the endpoint decides whether the roots are used).
///
/// @covers: GrpcEgressBuilder::build
#[tokio::test]
async fn test_build_tls_roots_only_with_plaintext_allowed_and_an_https_endpoint_is_accepted_edge() {
    let pki = TestPki::create().unwrap_or_else(|e| panic!("{e}"));
    let ca = std::fs::read(pki.get_ca_pem_path()).unwrap_or_else(|e| panic!("{e}"));
    let roots_only = TlsSettings::create(None, vec![ca], None);

    let result = builder()
        .with_plaintext_allowed()
        .with_tls(roots_only)
        .build();

    assert_eq!(describe(result), "built");
}

/// @covers: GrpcEgressBuilder::build
#[test]
fn test_build_max_decoding_message_size_zero_returns_invalid_option_value_error() {
    let result = builder().with_max_decoding_message_size(0).build();

    assert!(
        matches!(
            result,
            Err(GrpcChannelConfigError::InvalidOptionValue {
                option: "max_decoding_message_size",
                ..
            })
        ),
        "got {}",
        describe(result)
    );
}

/// @covers: GrpcEgressBuilder::build
#[test]
fn test_build_max_encoding_message_size_zero_returns_invalid_option_value_error() {
    let result = builder().with_max_encoding_message_size(0).build();

    assert!(
        matches!(
            result,
            Err(GrpcChannelConfigError::InvalidOptionValue {
                option: "max_encoding_message_size",
                ..
            })
        ),
        "got {}",
        describe(result)
    );
}

/// @covers: GrpcEgressBuilder::build
#[test]
fn test_build_channel_config_max_message_bytes_zero_returns_invalid_option_value_error(
) -> Result<(), Box<dyn std::error::Error>> {
    let result = config_builder("https://service.example:443")
        .with_max_message_bytes(0)
        .build();

    let error = result
        .err()
        .unwrap_or_else(|| panic!("a zero limit must be refused during config construction"));
    assert!(error.contains("max_message_bytes"), "got {error}");
    Ok(())
}

/// @covers: GrpcEgressBuilder::build
#[tokio::test]
async fn test_build_message_size_limits_of_one_byte_are_accepted_edge() {
    let result = builder()
        .with_max_decoding_message_size(1)
        .with_max_encoding_message_size(1)
        .build();

    assert_eq!(describe(result), "built");
}

/// @covers: GrpcEgressBuilder::build
#[test]
fn test_build_request_timeout_zero_returns_invalid_option_value_error() {
    let result = builder().with_request_timeout(Duration::ZERO).build();

    assert!(
        matches!(
            result,
            Err(GrpcChannelConfigError::InvalidOptionValue {
                option: "request_timeout",
                ..
            })
        ),
        "got {}",
        describe(result)
    );
}

/// @covers: GrpcEgressBuilder::build
#[tokio::test]
async fn test_build_request_timeout_one_nanosecond_is_accepted_edge() {
    let result = builder()
        .with_request_timeout(Duration::from_nanos(1))
        .build();

    assert_eq!(describe(result), "built");
}

/// @covers: GrpcEgressBuilder::build
#[test]
fn test_build_loadbalancer_with_no_backends_returns_invalid_option_value_error() {
    assert!(LoadbalancerSettings::try_new(BalancingStrategy::RoundRobin, Vec::new()).is_err());
}

/// @covers: GrpcEgressBuilder::build
#[test]
fn test_build_loadbalancer_with_an_invalid_backend_url_names_the_url_error() {
    let message = describe(
        TonicGrpcEgressBuilderSpiFactory::new("")
            .build(BuildGrpcEgressBuilderRequest)
            .with_loadbalancer(settings(&["!! not a valid url !!"]))
            .build(),
    );

    assert!(
        message.contains("!! not a valid url !!"),
        "the message must name the backend, got: {message}"
    );
}

/// @covers: GrpcEgressBuilder::build
#[test]
fn test_build_loadbalancer_with_an_endpoint_returns_conflict_naming_both_options_error() {
    let result = builder()
        .with_endpoint("https://other.example:443")
        .with_loadbalancer(settings(&["https://a.example:443"]))
        .build();

    match result {
        Err(GrpcChannelConfigError::ConflictingOptions { first, second, .. }) => {
            assert_eq!((first, second), ("endpoint", "loadbalancer"));
        }
        other => panic!("expected ConflictingOptions, got {}", describe(other)),
    }
}

/// @covers: GrpcEgressBuilder::build
#[test]
fn test_build_loadbalancer_with_a_channel_config_endpoint_returns_conflict_error(
) -> Result<(), Box<dyn std::error::Error>> {
    let config = config_builder("https://other.example:443").build()?;

    let result = builder()
        .with_channel_config(&config)
        .with_loadbalancer(settings(&["https://a.example:443"]))
        .build();

    assert!(
        matches!(
            result,
            Err(GrpcChannelConfigError::ConflictingOptions {
                first: "endpoint",
                second: "loadbalancer",
                ..
            })
        ),
        "got {}",
        describe(result)
    );
    Ok(())
}

/// The endpoint the factory was created with is not an option the caller set.
///
/// @covers: GrpcEgressBuilder::build
#[tokio::test]
async fn test_build_loadbalancer_with_only_the_factory_endpoint_is_accepted_edge(
) -> Result<(), Box<dyn Error>> {
    let result = builder()
        .with_loadbalancer(settings(&[
            "https://a.example:443",
            "https://b.example:443",
        ]))
        .build();

    assert_eq!(describe(result), "built");
    Ok(())
}

/// @covers: GrpcEgressBuilder::build
#[test]
fn test_build_loadbalancer_with_a_plaintext_backend_without_plaintext_allowed_is_rejected_error() {
    let result = builder()
        .with_loadbalancer(settings(&["https://a.example:443", "http://b.example:80"]))
        .build();

    assert!(
        matches!(&result, Err(GrpcChannelConfigError::PlaintextRejected(url)) if url == "http://b.example:80"),
        "the plaintext backend must be named, got {}",
        describe(result)
    );
}

/// A client configuration that names mutual-TLS files while it lets the client fall back to plaintext is refused, as
/// one that carries the identity in memory is: the files are an identity too.
///
/// @covers: GrpcEgressBuilder::build
#[test]
fn test_build_channel_config_mtls_files_with_plaintext_allowed_returns_conflict_naming_both_options_error(
) -> Result<(), Box<dyn std::error::Error>> {
    let config = grpc_egress_transport::GrpcChannelConfig::try_new(
        "https://service.example:443",
        false,
        Some(mtls_config("client.pem", "client.key")?),
        None,
        grpc_egress_transport::DEFAULT_MAX_MESSAGE_BYTES,
        grpc_egress_transport::CompressionMode::None,
        None,
    )?;

    let result = builder().with_channel_config(&config).build();

    assert!(
        matches!(
            result,
            Err(GrpcChannelConfigError::ConflictingOptions {
                first: "mtls",
                second: "plaintext_allowed",
                ..
            })
        ),
        "got {}",
        describe(result)
    );
    Ok(())
}

fn config_builder(
    endpoint: impl Into<String>,
) -> impl grpc_egress_transport::GrpcChannelConfigBuilder {
    new_config_builder().with_endpoint(endpoint)
}

fn new_config_builder() -> impl grpc_egress_transport::GrpcChannelConfigBuilder {
    use grpc_egress_transport::GrpcChannelConfigBuilderFactory;
    grpc_egress_transport_std::StdGrpcChannelConfigBuilderFactory
        .build(grpc_egress_transport::BuildGrpcChannelConfigBuilderRequest)
}

fn mtls_config(
    cert: impl Into<String>,
    key: impl Into<String>,
) -> Result<grpc_egress_transport::MtlsConfig, grpc_egress_transport::GrpcChannelConfigError> {
    use grpc_egress_transport::MtlsConfigFactory;
    grpc_egress_transport_std::StdMtlsConfigFactory.build(
        grpc_egress_transport::BuildMtlsConfigRequest {
            cert_pem_path: cert.into(),
            key_pem_path: key.into(),
            ca_pem_path: None,
        },
    )
}

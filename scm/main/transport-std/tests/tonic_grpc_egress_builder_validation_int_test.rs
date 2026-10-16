//! Tests of the combinations and values `build` refuses. Each error names the option, and a
//! conflict names both options, so that a caller knows what to change. Nothing panics.

use grpc_egress_transport_std::{GrpcChannelConfigFactory, MtlsConfigFactory};
use std::error::Error;
use std::time::Duration;

use grpc_egress_transport::{
    BackendTarget, BalancingStrategy, BuildGrpcEgressBuilderRequest, GrpcChannelConfig,
    GrpcChannelConfigError, GrpcEgressBuilder, GrpcEgressBuilderFactory, LoadbalancerSettings,
    MtlsConfig,
};
use grpc_egress_transport_std::TonicGrpcEgressBuilderSpiFactory;

fn builder() -> impl GrpcEgressBuilder {
    TonicGrpcEgressBuilderSpiFactory::new("https://service.example:443")
        .build(BuildGrpcEgressBuilderRequest)
}

fn settings(urls: &[&str]) -> LoadbalancerSettings {
    LoadbalancerSettings {
        strategy: BalancingStrategy::RoundRobin,
        backends: urls
            .iter()
            .map(|url| BackendTarget {
                url: (*url).to_string(),
                weight: 1,
            })
            .collect(),
    }
}

fn describe<T>(result: Result<T, GrpcChannelConfigError>) -> String {
    match result {
        Ok(_) => "built".to_string(),
        Err(e) => e.to_string(),
    }
}

/// @covers: GrpcEgressBuilder::build
#[test]
fn test_build_mtls_with_plaintext_allowed_returns_conflict_naming_both_options_error() {
    let result = builder()
        .with_plaintext_allowed()
        .with_mtls(MtlsConfig::new("client.pem", "client.key"))
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
fn test_build_mtls_with_plaintext_allowed_message_contains_both_option_names_error() {
    let message = describe(
        builder()
            .with_plaintext_allowed()
            .with_mtls(MtlsConfig::new("client.pem", "client.key"))
            .build(),
    );

    assert!(
        message.contains("'mtls'") && message.contains("'plaintext_allowed'"),
        "the message must name both options, got: {message}"
    );
}

/// @covers: GrpcEgressBuilder::build
#[test]
fn test_build_max_message_bytes_zero_returns_invalid_option_value_error() {
    let result = builder().with_max_message_bytes(0).build();

    assert!(
        matches!(
            result,
            Err(GrpcChannelConfigError::InvalidOptionValue {
                option: "max_message_bytes",
                ..
            })
        ),
        "got {}",
        describe(result)
    );
}

/// @covers: GrpcEgressBuilder::build
#[tokio::test]
async fn test_build_max_message_bytes_one_is_accepted_edge() {
    assert_eq!(
        describe(builder().with_max_message_bytes(1).build()),
        "built"
    );
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
    let result = TonicGrpcEgressBuilderSpiFactory::new("")
        .build(BuildGrpcEgressBuilderRequest)
        .with_loadbalancer(settings(&[]))
        .build();

    match result {
        Err(GrpcChannelConfigError::InvalidOptionValue { option, reason }) => {
            assert_eq!(option, "loadbalancer");
            assert!(reason.contains("no backends"), "reason: {reason}");
        }
        other => panic!("expected InvalidOptionValue, got {}", describe(other)),
    }
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
fn test_build_loadbalancer_with_a_channel_config_endpoint_returns_conflict_error() {
    let config = GrpcChannelConfig::new("https://other.example:443");

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

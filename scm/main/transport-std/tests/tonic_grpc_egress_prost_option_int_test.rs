//! Tests of the `with_prost_codec` option: it is always present, it works when the Cargo feature
//! `prost` is on, and without the feature `build` refuses it and names the option and the feature.

use std::collections::BTreeSet;
use std::error::Error;
use std::time::Duration;

use grpc_egress_test_support::{Fixture, PeerBehaviour, PeerServer};
use grpc_egress_transport::{
    BuildGrpcEgressBuilderRequest, GrpcEgress, GrpcEgressBuilder, GrpcEgressBuilderFactory,
    GrpcRequest,
};
use grpc_egress_transport_std::{GrpcRequestFactory, TonicGrpcEgressBuilderSpiFactory};

/// The Cargo features that have a `with_*` option on the builder, as `(feature, option)`. A feature
/// that is added to the Cargo manifest must get its option and its entry here.
const FEATURE_OPTIONS: &[(&str, &str)] = &[("prost", "with_prost_codec")];

async fn content_type_sent(prost: bool) -> Result<Option<String>, Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(b"ok".to_vec())).await?;
    let builder = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed();
    let builder = if prost {
        builder.with_prost_codec()
    } else {
        builder
    };
    let client = builder.build()?;
    client
        .call_unary(GrpcRequest::new(
            "pkg.Service/Method",
            b"ping".to_vec(),
            Duration::from_secs(5),
        ))
        .await?;
    let recorded = peer.get_recorded_requests();
    Ok(recorded
        .first()
        .and_then(|request| request.get_header("content-type"))
        .map(str::to_string))
}

/// @covers: GrpcEgressBuilder::with_prost_codec
#[cfg(feature = "prost")]
#[tokio::test]
async fn test_build_with_prost_codec_feature_on_sends_the_proto_content_type(
) -> Result<(), Box<dyn Error>> {
    assert_eq!(
        content_type_sent(true).await?.as_deref(),
        Some("application/grpc+proto")
    );
    Ok(())
}

/// @covers: GrpcEgressBuilder::build
#[tokio::test]
async fn test_build_without_prost_codec_sends_the_plain_content_type() -> Result<(), Box<dyn Error>>
{
    assert_eq!(
        content_type_sent(false).await?.as_deref(),
        Some("application/grpc")
    );
    Ok(())
}

/// @covers: GrpcEgressBuilder::build
#[cfg(not(feature = "prost"))]
#[test]
fn test_build_with_prost_codec_feature_off_names_the_option_and_the_feature() {
    use grpc_egress_transport::GrpcChannelConfigError;

    let result = TonicGrpcEgressBuilderSpiFactory::new("http://127.0.0.1:1")
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_prost_codec()
        .build();

    match result {
        Err(GrpcChannelConfigError::DisabledFeatureOptions { options }) => {
            assert_eq!(options, vec![("prost_codec", "prost")]);
        }
        Err(other) => panic!("expected DisabledFeatureOptions, got {other}"),
        Ok(_) => panic!("with_prost_codec must be refused while the feature `prost` is off"),
    }
}

/// @covers: GrpcChannelConfigError::DisabledFeatureOptions
#[test]
fn test_display_disabled_feature_options_lists_every_option_with_its_feature() {
    use grpc_egress_transport::GrpcChannelConfigError;

    let message = GrpcChannelConfigError::DisabledFeatureOptions {
        options: vec![("prost_codec", "prost"), ("other", "extra")],
    }
    .to_string();

    assert!(
        message.contains("option 'prost_codec' needs the Cargo feature 'prost'"),
        "{message}"
    );
    assert!(
        message.contains("option 'other' needs the Cargo feature 'extra'"),
        "{message}"
    );
}

/// @covers: GrpcEgressBuilder::with_prost_codec
#[test]
fn test_cargo_features_every_feature_has_a_builder_option() -> Result<(), Box<dyn Error>> {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))?;
    let section = manifest
        .split("[features]")
        .nth(1)
        .and_then(|rest| rest.split("\n[").next())
        .ok_or("Cargo.toml has no [features] section")?;
    let declared: BTreeSet<&str> = section
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .filter_map(|line| line.split('=').next().map(str::trim))
        .filter(|name| !name.is_empty() && *name != "default")
        .collect();
    let covered: BTreeSet<&str> = FEATURE_OPTIONS
        .iter()
        .map(|(feature, _)| *feature)
        .collect();

    assert_eq!(
        declared, covered,
        "every Cargo feature needs a builder option that `build` refuses while it is off"
    );
    Ok(())
}

//! Tests of `LoadbalancerSettingsProvider::from_config`: the `[loadbalancer]` table of a configuration
//! file becomes the settings of a balanced client.

use proto_pattern::{CallRawRequestBuilder, CallRawResponse, RawTransport};
use std::error::Error;

use grpc_egress_transport::{BackendTarget, BalancingStrategy, GrpcChannelConfigError};

const FULL: &str = r#"
[loadbalancer]
strategy = "weighted"

[[loadbalancer.backends]]
url    = "https://grpc-1.internal:50051"
weight = 1

[[loadbalancer.backends]]
url    = "https://grpc-2.internal:50051"
weight = 2
"#;

/// @covers: LoadbalancerSettingsProvider::from_config
#[test]
fn test_from_config_full_table_returns_the_strategy_and_every_backend() -> Result<(), Box<dyn Error>>
{
    let settings = read_settings(FULL)?;

    assert_eq!(settings.strategy(), BalancingStrategy::Weighted);
    assert_eq!(
        settings.backends(),
        vec![
            BackendTarget::try_new("https://grpc-1.internal:50051", 1)
                .map_err(std::io::Error::other)?,
            BackendTarget::try_new("https://grpc-2.internal:50051", 2)
                .map_err(std::io::Error::other)?,
        ]
    );
    Ok(())
}

/// @covers: LoadbalancerSettingsProvider::from_config
#[test]
fn test_from_config_every_strategy_name_is_understood() -> Result<(), Box<dyn Error>> {
    for (name, expected) in [
        ("round-robin", BalancingStrategy::RoundRobin),
        ("weighted", BalancingStrategy::Weighted),
        ("least-connections", BalancingStrategy::LeastConnections),
    ] {
        let text = format!("[loadbalancer]\nstrategy = \"{name}\"\n\n[[loadbalancer.backends]]\nurl = \"https://svc.internal\"\nweight = 1\n");

        assert_eq!(read_settings(&text)?.strategy(), expected, "{name}");
    }
    Ok(())
}

/// @covers: LoadbalancerSettingsProvider::from_config
#[test]
fn test_from_config_unknown_strategy_names_the_option() {
    let text = "[loadbalancer]\nstrategy = \"random\"\nbackends = []\n";

    match read_settings(text) {
        Err(GrpcChannelConfigError::InvalidOptionValue { option, reason }) => {
            assert_eq!(option, "loadbalancer");
            assert!(reason.contains("random"), "{reason}");
        }
        other => panic!("expected InvalidOptionValue, got {other:?}"),
    }
}

/// @covers: LoadbalancerSettingsProvider::from_config
#[test]
fn test_from_config_unknown_key_is_refused() {
    let text = "[loadbalancer]\nstrategy = \"weighted\"\nbackends = []\nretries = 3\n";

    assert!(matches!(
        read_settings(text),
        Err(GrpcChannelConfigError::InvalidOptionValue { .. })
    ));
}

/// @covers: LoadbalancerSettingsProvider::from_config
#[test]
fn test_from_config_text_without_the_table_is_refused() {
    assert!(matches!(
        read_settings("[other]\nkey = 1\n"),
        Err(GrpcChannelConfigError::InvalidOptionValue { .. })
    ));
}

/// @covers: LoadbalancerSettingsProvider::from_config
#[tokio::test]
async fn test_from_config_settings_drive_a_balanced_client_that_reaches_every_backend(
) -> Result<(), Box<dyn Error>> {
    use std::time::Duration;

    use grpc_egress_test_support::CallFixture;
    use grpc_egress_test_support::{Fixture, PeerBehaviour, PeerServer};
    use grpc_egress_transport::{
        BuildGrpcEgressBuilderRequest, GrpcEgressBuilder, GrpcEgressBuilderFactory,
    };
    use grpc_egress_transport_spi::TonicGrpcEgressBuilderSpiFactory;
    use RawTransport;

    Fixture::ensure_tls_provider();
    let one = PeerServer::start(PeerBehaviour::Reply(b"one".to_vec())).await?;
    let two = PeerServer::start(PeerBehaviour::Reply(b"two".to_vec())).await?;
    let text = format!(
        "[loadbalancer]\nstrategy = \"round-robin\"\n\n[[loadbalancer.backends]]\nurl = \"{}\"\nweight = 1\n\n[[loadbalancer.backends]]\nurl = \"{}\"\nweight = 1\n",
        one.get_endpoint(),
        two.get_endpoint()
    );
    let client = TonicGrpcEgressBuilderSpiFactory::new("")
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_loadbalancer(read_settings(&text)?)
        .build()?;

    for _ in 0..6 {
        client
            .call_raw({
                let (message, options) =
                    CallFixture::request(b"ping", Duration::from_secs(5)).into_parts();
                CallRawRequestBuilder::new()
                    .with_path(CallFixture::path("pkg.Service/Method"))
                    .with_message(message)
                    .with_options(options)
                    .build()
            })
            .await
            .map(CallRawResponse::into_response)?;
    }

    assert!(
        !one.get_recorded_requests().is_empty(),
        "backend one saw no call"
    );
    assert!(
        !two.get_recorded_requests().is_empty(),
        "backend two saw no call"
    );
    Ok(())
}

fn read_settings(
    text: &str,
) -> Result<
    grpc_egress_transport::LoadbalancerSettings,
    grpc_egress_transport::GrpcChannelConfigError,
> {
    use grpc_egress_transport::{
        BuildLoadbalancerSettingsProviderRequest, LoadbalancerSettingsProvider,
        LoadbalancerSettingsProviderFactory,
    };
    grpc_egress_transport_spi::StdLoadbalancerSettingsProviderFactory
        .build(BuildLoadbalancerSettingsProviderRequest)
        .parse_config(text)
}

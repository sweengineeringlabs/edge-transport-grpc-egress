//! Tests of `LoadbalancerSettingsProvider::from_config`: the `[loadbalancer]` table of a configuration
//! file becomes the settings of a balanced client.

use std::error::Error;

use grpc_egress_transport::{
    BackendTarget, BalancingStrategy, GrpcChannelConfigError, LoadbalancerSettings,
};
use grpc_egress_transport_std::LoadbalancerSettingsProvider;

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
    let settings = LoadbalancerSettings::from_config(FULL)?;

    assert_eq!(settings.strategy, BalancingStrategy::Weighted);
    assert_eq!(
        settings.backends,
        vec![
            BackendTarget {
                url: "https://grpc-1.internal:50051".to_string(),
                weight: 1
            },
            BackendTarget {
                url: "https://grpc-2.internal:50051".to_string(),
                weight: 2
            },
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
        let text = format!("[loadbalancer]\nstrategy = \"{name}\"\nbackends = []\n");

        assert_eq!(
            LoadbalancerSettings::from_config(&text)?.strategy,
            expected,
            "{name}"
        );
    }
    Ok(())
}

/// @covers: LoadbalancerSettingsProvider::from_config
#[test]
fn test_from_config_unknown_strategy_names_the_option() {
    let text = "[loadbalancer]\nstrategy = \"random\"\nbackends = []\n";

    match LoadbalancerSettings::from_config(text) {
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
        LoadbalancerSettings::from_config(text),
        Err(GrpcChannelConfigError::InvalidOptionValue { .. })
    ));
}

/// @covers: LoadbalancerSettingsProvider::from_config
#[test]
fn test_from_config_text_without_the_table_is_refused() {
    assert!(matches!(
        LoadbalancerSettings::from_config("[other]\nkey = 1\n"),
        Err(GrpcChannelConfigError::InvalidOptionValue { .. })
    ));
}

/// @covers: LoadbalancerSettingsProvider::from_config
#[tokio::test]
async fn test_from_config_settings_drive_a_balanced_client_that_reaches_every_backend(
) -> Result<(), Box<dyn Error>> {
    use std::time::Duration;

    use grpc_egress_test_support::{Fixture, PeerBehaviour, PeerServer};
    use grpc_egress_transport::{
        BuildGrpcEgressBuilderRequest, GrpcEgress, GrpcEgressBuilder, GrpcEgressBuilderFactory,
        GrpcRequest,
    };
    use grpc_egress_transport_std::{GrpcRequestFactory, TonicGrpcEgressBuilderSpiFactory};

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
        .with_loadbalancer(LoadbalancerSettings::from_config(&text)?)
        .build()?;

    for _ in 0..6 {
        client
            .call_unary(GrpcRequest::new(
                "pkg.Service/Method",
                b"ping".to_vec(),
                Duration::from_secs(5),
            ))
            .await?;
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

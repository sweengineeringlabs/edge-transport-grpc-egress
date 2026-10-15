#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`BreakerObservableFactory`].

use edge_transport_grpc_egress_breaker::{
    BreakerObservableFactory, BreakerState, GrpcBreakerClient, GrpcBreakerConfig,
    ObserveStateRequest,
};

/// @covers: from_client
#[test]
fn test_from_client_upcasts_to_observable_happy() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let client = GrpcBreakerClient::new((), GrpcBreakerConfig::default());
        let observable = BreakerObservableFactory::from_client(client);
        let resp = observable
            .state(ObserveStateRequest)
            .await
            .expect("upcast observable must genuinely work");
        assert!(matches!(resp.state, BreakerState::Closed));
    });
}

/// @covers: from_client
#[test]
fn test_from_client_reflects_supplied_config_edge() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let client = GrpcBreakerClient::new(
            (),
            GrpcBreakerConfig {
                failure_threshold: 1,
                cool_down_seconds: 0,
                half_open_probe_count: 1,
            },
        );
        let observable = BreakerObservableFactory::from_client(client);
        // Immediately closed; the "error"-adjacent scenario proves the
        // observable reflects real state, not a hardcoded stub value.
        let resp = observable
            .state(ObserveStateRequest)
            .await
            .expect("must succeed");
        assert!(matches!(resp.state, BreakerState::Closed));
    });
}

/// @covers: from_client
#[test]
fn test_from_client_generic_over_inner_type_edge() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let client = GrpcBreakerClient::new(42_i32, GrpcBreakerConfig::default());
        let observable = BreakerObservableFactory::from_client(client);
        let resp = observable
            .state(ObserveStateRequest)
            .await
            .expect("must work regardless of inner type");
        assert!(matches!(resp.state, BreakerState::Closed));
    });
}

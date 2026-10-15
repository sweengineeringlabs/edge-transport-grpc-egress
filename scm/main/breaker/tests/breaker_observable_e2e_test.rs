#![allow(clippy::unwrap_used, clippy::expect_used)]
//! End-to-end tests for [`BreakerObservable`] via a test-double
//! implementation.

use edge_transport_grpc_egress_breaker::{
    BreakerObservable, BreakerState, Error, GrpcBreakerClient, GrpcBreakerConfig,
    ObserveStateRequest, ObserveStateResponse,
};
use futures::future::BoxFuture;

struct MockObservable {
    state: BreakerState,
    fail: bool,
}

impl BreakerObservable for MockObservable {
    fn state(
        &self,
        _req: ObserveStateRequest,
    ) -> BoxFuture<'_, Result<ObserveStateResponse, Error>> {
        let state = self.state;
        let fail = self.fail;
        Box::pin(async move {
            if fail {
                return Err(Error::InvalidConfig(
                    "mock observable forced failure".into(),
                ));
            }
            Ok(ObserveStateResponse { state })
        })
    }
}

/// @covers: state
#[test]
fn test_state_returns_configured_snapshot_happy() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let observable = MockObservable {
            state: BreakerState::Closed,
            fail: false,
        };
        let resp = observable
            .state(ObserveStateRequest)
            .await
            .expect("happy path");
        assert!(matches!(resp.state, BreakerState::Closed));
    });
}

/// @covers: state
#[test]
fn test_state_propagates_failure_error() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let observable = MockObservable {
            state: BreakerState::Closed,
            fail: true,
        };
        let err = observable
            .state(ObserveStateRequest)
            .await
            .err()
            .expect("forced failure must surface");
        assert!(err.to_string().contains("mock observable forced failure"));
    });
}

/// @covers: state
#[test]
fn test_state_open_snapshot_edge() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let observable = MockObservable {
            state: BreakerState::Open {
                since: std::time::Instant::now(),
            },
            fail: false,
        };
        let resp = observable
            .state(ObserveStateRequest)
            .await
            .expect("happy path");
        assert!(matches!(resp.state, BreakerState::Open { .. }));
    });
}

/// @covers: default_client
#[test]
fn test_default_client_constructs_a_closed_client_happy() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let client = <GrpcBreakerClient<()> as BreakerObservable>::default_client(
            (),
            GrpcBreakerConfig::default(),
        );
        assert!(matches!(client.state().await, BreakerState::Closed));
    });
}

/// @covers: default_client
#[test]
fn test_default_client_applies_supplied_config_error() {
    // "error" scenario for an infallible constructor: prove it does NOT
    // silently substitute a default config when one is supplied.
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let client = <GrpcBreakerClient<()> as BreakerObservable>::default_client(
            (),
            GrpcBreakerConfig {
                failure_threshold: 42,
                cool_down_seconds: 1,
                half_open_probe_count: 1,
            },
        );
        assert_eq!(client.config().failure_threshold, 42);
    });
}

/// @covers: default_client
#[test]
fn test_default_client_zero_threshold_config_edge() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let client = <GrpcBreakerClient<()> as BreakerObservable>::default_client(
            (),
            GrpcBreakerConfig {
                failure_threshold: 0,
                cool_down_seconds: 0,
                half_open_probe_count: 0,
            },
        );
        assert_eq!(client.config().failure_threshold, 0);
    });
}

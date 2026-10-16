#![allow(clippy::unwrap_used, clippy::expect_used)]
//! End-to-end tests for [`BreakerObservable`] via a test-double
//! implementation.

use futures::future::BoxFuture;
use grpc_egress_breaker::{
    BreakerObservable, BreakerState, Error, GrpcBreakerConfig, ObserveStateRequest,
    ObserveStateResponse,
};
use grpc_egress_breaker_std::GrpcBreakerClient;

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

/// @covers: GrpcBreakerClient::new
#[test]
fn test_new_constructs_a_closed_client_happy() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let client = GrpcBreakerClient::new((), GrpcBreakerConfig::default());
        assert!(matches!(client.state().await, BreakerState::Closed));
    });
}

/// @covers: GrpcBreakerClient::new
#[test]
fn test_new_applies_supplied_config_error() {
    // An infallible constructor must not silently substitute a default config for the one supplied.
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let client = GrpcBreakerClient::new(
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

/// @covers: GrpcBreakerClient::new
#[test]
fn test_new_zero_threshold_config_is_kept_edge() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let client = GrpcBreakerClient::new(
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

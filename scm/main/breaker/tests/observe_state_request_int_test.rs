#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`ObserveStateRequest`].

use edge_transport_grpc_egress_breaker::{
    BreakerObservable, BreakerState, GrpcBreakerClient, GrpcBreakerConfig, ObserveStateRequest,
};

/// @covers: ObserveStateRequest
#[test]
fn test_observe_state_request_is_constructible_happy() {
    let req = ObserveStateRequest;
    assert_eq!(std::mem::size_of_val(&req), 0);
}

/// @covers: ObserveStateRequest
#[test]
fn test_observe_state_request_used_by_real_observable_error() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let client = GrpcBreakerClient::new((), GrpcBreakerConfig::default());
        let resp = BreakerObservable::state(&client, ObserveStateRequest)
            .await
            .expect("real observable must accept this request type");
        assert!(matches!(resp.state, BreakerState::Closed));
    });
}

/// @covers: ObserveStateRequest
#[test]
fn test_observe_state_request_reusable_edge() {
    let a = ObserveStateRequest;
    let b = ObserveStateRequest;
    assert_eq!(std::mem::size_of_val(&a), std::mem::size_of_val(&b));
}

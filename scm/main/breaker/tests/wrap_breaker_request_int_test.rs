//! Integration tests for [`WrapBreakerRequest`].

use edge_transport_grpc_egress_breaker::{GrpcBreakerConfig, WrapBreakerRequest};

/// @covers: WrapBreakerRequest
#[test]
fn test_wrap_breaker_request_preserves_inner_happy() {
    let req = WrapBreakerRequest {
        inner: "stub-client",
        config: GrpcBreakerConfig::default(),
    };
    assert_eq!(req.inner, "stub-client");
}

/// @covers: WrapBreakerRequest
#[test]
fn test_wrap_breaker_request_preserves_config_error() {
    let req = WrapBreakerRequest {
        inner: (),
        config: GrpcBreakerConfig {
            failure_threshold: 0,
            cool_down_seconds: 0,
            half_open_probe_count: 0,
        },
    };
    assert_eq!(req.config.failure_threshold, 0);
}

/// @covers: WrapBreakerRequest
#[test]
fn test_wrap_breaker_request_generic_over_inner_type_edge() {
    let req = WrapBreakerRequest {
        inner: 42_i32,
        config: GrpcBreakerConfig::default(),
    };
    assert_eq!(req.inner, 42);
}

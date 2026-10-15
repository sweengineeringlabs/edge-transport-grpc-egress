//! Coverage stub for `src/api/breaker/grpc/grpc_breaker_client.rs`.

use edge_transport_grpc_egress_breaker::{GrpcBreakerClient, GrpcBreakerConfig};

/// @covers: GrpcBreakerClient — type is accessible and holds real state (not zero-sized)
#[test]
fn breaker_struct_grpc_breaker_client_is_accessible_int_test() {
    assert!(
        std::mem::size_of::<GrpcBreakerClient<()>>() > 0,
        "GrpcBreakerClient holds inner/config/node fields and must not be zero-sized"
    );
}

/// @covers: GrpcBreakerClient::new — config is stored and accessible
#[test]
fn breaker_struct_grpc_breaker_client_new_stores_config_int_test() {
    let cfg = GrpcBreakerConfig {
        failure_threshold: 3,
        cool_down_seconds: 10,
        half_open_probe_count: 2,
    };
    let client = GrpcBreakerClient::new((), cfg);
    assert_eq!(client.config().failure_threshold, 3);
    assert_eq!(client.config().cool_down_seconds, 10);
    assert_eq!(client.config().half_open_probe_count, 2);
}

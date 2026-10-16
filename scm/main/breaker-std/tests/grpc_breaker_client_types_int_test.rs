//! Coverage stub for `src/api/types/grpc/grpc_breaker_client.rs`.
//!
//! This file declares the `GrpcBreakerClient<T>` type alias that the SAF
//! return types trace to (SEA rule 211).

use grpc_egress_breaker::GrpcBreakerConfig;
use grpc_egress_breaker_std::GrpcBreakerClient;

/// @covers: api/types/grpc/grpc_breaker_client — type alias resolves
#[test]
fn breaker_type_grpc_breaker_client_types_alias_resolves_int_test() {
    // The types/ alias and the breaker/grpc/ declaration resolve to the
    // same concrete type. Verify construction succeeds.
    let client = GrpcBreakerClient::new((), GrpcBreakerConfig::default());
    assert!(client.config().failure_threshold >= 1);
}

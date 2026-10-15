#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Coverage stub for `src/api/types/grpc/grpc_retry_config_builder.rs`.

use edge_transport_grpc_egress_retry::GrpcRetryConfigBuilder;

/// @covers: GrpcRetryConfigBuilder — type is accessible and holds real fields (not zero-sized)
#[test]
fn retry_struct_grpc_retry_config_builder_is_accessible_int_test() {
    assert!(
        std::mem::size_of::<GrpcRetryConfigBuilder>() > 0,
        "GrpcRetryConfigBuilder accumulates real config fields and must not be zero-sized"
    );
}

/// @covers: GrpcRetryConfigBuilder::new — pre-seeded with SWE defaults
#[test]
fn retry_struct_grpc_retry_config_builder_new_builds_valid_config_int_test() {
    let cfg = GrpcRetryConfigBuilder::new()
        .build()
        .expect("default builder must produce valid config");
    assert!(cfg.max_attempts >= 1);
}

/// @covers: GrpcRetryConfigBuilder::max_attempts — overrides default
#[test]
fn retry_struct_grpc_retry_config_builder_max_attempts_override_int_test() {
    let cfg = GrpcRetryConfigBuilder::new()
        .max_attempts(3)
        .build()
        .expect("valid config");
    assert_eq!(cfg.max_attempts, 3);
}

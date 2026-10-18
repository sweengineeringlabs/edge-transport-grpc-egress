//! Coverage stub for `src/api/types/grpc/grpc_retry_config_builder.rs`.

/// @covers: GrpcRetryConfigBuilder — type is accessible and holds real fields (not zero-sized)
use grpc_egress_retry::GrpcRetryConfigBuilder as _;
#[test]
fn retry_struct_grpc_retry_config_builder_is_accessible_int_test() {
    assert!(
        std::mem::size_of_val(&retry_config_builder()) > 0,
        "GrpcRetryConfigBuilder accumulates real config fields and must not be zero-sized"
    );
}

/// @covers: GrpcRetryConfigBuilder::new — pre-seeded with SWE defaults
#[test]
fn retry_struct_grpc_retry_config_builder_new_builds_valid_config_int_test(
) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = retry_config_builder().build()?;
    assert!(cfg.max_attempts >= 1);
    Ok(())
}

/// @covers: GrpcRetryConfigBuilder::max_attempts — overrides default
#[test]
fn retry_struct_grpc_retry_config_builder_max_attempts_override_int_test(
) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = retry_config_builder().max_attempts(3).build()?;
    assert_eq!(cfg.max_attempts, 3);
    Ok(())
}

fn retry_config_builder() -> impl grpc_egress_retry::GrpcRetryConfigBuilder {
    use grpc_egress_retry::GrpcRetryConfigBuilderFactory;
    grpc_egress_retry_std::StdGrpcRetryConfigBuilderFactory
        .build(grpc_egress_retry::BuildGrpcRetryConfigBuilderRequest)
}

//! Coverage stub for `api/bearer/bearer_egress_config_builder.rs`.

use edge_transport_grpc_egress_auth_bearer::{BearerEgressConfigBuilder, BearerSecret};

/// @covers: BearerEgressConfigBuilder — fluent builder produces valid config
#[test]
fn bearer_struct_bearer_egress_config_builder_builds_valid_config_int_test() -> Result<(), String> {
    let config = BearerEgressConfigBuilder::new()
        .secret(BearerSecret::Hs256 {
            secret: b"key".to_vec(),
        })
        .issuer("iss")
        .audience("aud")
        .subject("sub")
        .lifetime_seconds(3600)
        .build()?;
    assert_eq!(config.issuer, "iss");
    assert_eq!(config.lifetime_seconds, 3600);
    Ok(())
}

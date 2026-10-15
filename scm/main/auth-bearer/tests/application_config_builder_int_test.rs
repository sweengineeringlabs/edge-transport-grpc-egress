//! Integration tests for [`ApplicationConfigBuilder`].

use edge_transport_grpc_egress_auth_bearer::GrpcAuthBearerSvc;

/// @covers: ApplicationConfigBuilder (via GrpcAuthBearerSvc::create_config_builder)
#[test]
fn test_application_config_builder_is_constructible_via_svc() {
    // ApplicationConfigBuilder is the concrete type returned by create_config_builder.
    let builder = GrpcAuthBearerSvc::create_config_builder();
    let _ = builder;
}

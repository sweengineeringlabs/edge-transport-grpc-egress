//! Integration tests for `api/value/mtls_config.rs`.

#[test]
fn transport_struct_new_leaves_root_ca_unset_int_test() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = mtls_config("c.pem", "k.pem")?;
    assert!(cfg.ca_pem_path().is_none());
    assert_eq!(cfg.cert_pem_path(), "c.pem");
    assert_eq!(cfg.key_pem_path(), "k.pem");
    Ok(())
}

#[test]
fn transport_struct_with_root_ca_stores_all_three_paths_int_test(
) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = mtls_config_with_ca("c.pem", "k.pem", "ca.pem")?;
    assert_eq!(cfg.cert_pem_path(), "c.pem");
    assert_eq!(cfg.key_pem_path(), "k.pem");
    assert_eq!(cfg.ca_pem_path(), Some("ca.pem"));
    Ok(())
}

#[test]
fn mtls_config_rejects_empty_paths_on_construction_and_deserialization() {
    use grpc_egress_transport::MtlsConfig;

    assert!(MtlsConfig::try_new("", "key.pem", None).is_err());
    assert!(MtlsConfig::try_new("cert.pem", " ", None).is_err());
    assert!(MtlsConfig::try_new("cert.pem", "key.pem", Some(String::new())).is_err());
    assert!(
        toml::from_str::<MtlsConfig>("cert_pem_path = \"\"\nkey_pem_path = \"key.pem\"\n").is_err()
    );
}

fn mtls_config(
    cert: impl Into<String>,
    key: impl Into<String>,
) -> Result<grpc_egress_transport::MtlsConfig, grpc_egress_transport::GrpcChannelConfigError> {
    use grpc_egress_transport::MtlsConfigFactory;
    grpc_egress_transport_std::StdMtlsConfigFactory.build(
        grpc_egress_transport::BuildMtlsConfigRequest {
            cert_pem_path: cert.into(),
            key_pem_path: key.into(),
            ca_pem_path: None,
        },
    )
}

fn mtls_config_with_ca(
    cert: impl Into<String>,
    key: impl Into<String>,
    ca: impl Into<String>,
) -> Result<grpc_egress_transport::MtlsConfig, grpc_egress_transport::GrpcChannelConfigError> {
    use grpc_egress_transport::MtlsConfigFactory;
    grpc_egress_transport_std::StdMtlsConfigFactory.build(
        grpc_egress_transport::BuildMtlsConfigRequest {
            cert_pem_path: cert.into(),
            key_pem_path: key.into(),
            ca_pem_path: Some(ca.into()),
        },
    )
}

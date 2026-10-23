//! `StdMtlsConfigFactory` -- creates the mutual-TLS configuration of an outbound identity.

use grpc_egress_transport_api::{BuildMtlsConfigRequest, MtlsConfig, MtlsConfigFactory};

/// Creates mutual-TLS configurations.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdMtlsConfigFactory;

impl MtlsConfigFactory for StdMtlsConfigFactory {
    fn build(
        &self,
        req: BuildMtlsConfigRequest,
    ) -> Result<MtlsConfig, grpc_egress_transport_api::GrpcChannelConfigError> {
        MtlsConfig::try_new(req.cert_pem_path, req.key_pem_path, req.ca_pem_path)
    }
}

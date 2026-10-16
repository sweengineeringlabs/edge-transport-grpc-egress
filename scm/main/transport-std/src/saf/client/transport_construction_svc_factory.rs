//! [`TransportConstruction`] — wires together `core/` and `spi/` adapters
//! into a [`TransportSvc`](crate::std::TransportSvc)-adjacent transport.
//!
//! Declared here (not on `TransportSvc` itself) because:
//! - `no_inherent_impl_on_api_type` bans inherent impls in saf/ for types
//!   declared in api/ (`TransportSvc` lives in `api/types/`).
//! - `boundary_peer_isolation` bans core/ and spi/ from importing each
//!   other directly, and these methods construct concrete `spi/` adapters
//!   (`TonicGrpcEgress`, `TonicLbGrpcEgress`) — genuine core+spi wiring
//!   belongs in saf/, the composition layer.
//! - `layers_no_free_standing_fn` bans bare functions in SEA layer files —
//!   methods must live on a type declared in the same layer, hence this
//!   saf/-declared marker struct.

use std::sync::Arc;
use std::time::Duration;

use crate::spi::client::tonic::{TonicGrpcEgress, TonicGrpcEgressProtocol};
use grpc_egress_transport::{
    GrpcChannelConfig, GrpcChannelConfigError, GrpcEgress, Processor, DEFAULT_REQUEST_TIMEOUT_SECS,
};

/// Namespace for the construction methods wiring `core/` + `spi/` adapters
/// into transports. Zero-sized — never instantiated.
pub struct TransportConstruction;

impl TransportConstruction {
    /// Construct a boxed [`GrpcEgress`] from a validated [`GrpcChannelConfig`].
    pub fn create_transport_from_config(
        config: &GrpcChannelConfig,
    ) -> Result<Arc<dyn GrpcEgress>, GrpcChannelConfigError> {
        let client = Self::create_tonic_client_from_config(config)?;
        let transport: Arc<dyn GrpcEgress> = Arc::new(client);
        Ok(transport)
    }

    /// Construct a concrete [`GrpcEgress`] + [`Processor`] transport from a
    /// validated [`GrpcChannelConfig`].
    ///
    /// Returns an opaque `impl GrpcEgress + Processor` rather than naming the
    /// concrete adapter type directly (see SEA rule `pub_types_in_api_only`);
    /// callers that need to compose further (e.g. wrapping in a retry/breaker
    /// decorator) can still do so generically.
    pub fn create_tonic_client_from_config(
        config: &GrpcChannelConfig,
    ) -> Result<impl GrpcEgress + Processor, GrpcChannelConfigError> {
        if config.tls_required && TonicGrpcEgressProtocol::is_plaintext_endpoint(&config.endpoint) {
            return Err(GrpcChannelConfigError::PlaintextRejected(
                config.endpoint.clone(),
            ));
        }
        let timeout = Duration::from_secs(
            config
                .request_timeout_secs
                .unwrap_or(DEFAULT_REQUEST_TIMEOUT_SECS),
        );
        let root_ca = config.mtls.as_ref().and_then(|m| m.ca_pem_path.as_deref());
        let mut client = TonicGrpcEgress::new(&config.endpoint)
            .with_tls(config.mtls.as_ref(), root_ca)?
            .with_request_timeout(timeout);
        if let Some(keep_alive) = config.keep_alive {
            client = client.with_keep_alive(keep_alive);
        }
        client.max_message_bytes = config.max_message_bytes;
        client.compression = config.compression;
        Ok(client)
    }
}

#[cfg(feature = "prost")]
impl TransportConstruction {
    /// Construct a [`grpc_egress_transport::GrpcEgressProstCodec`] transport from a validated
    /// [`GrpcChannelConfig`] — the prost-enabled counterpart of
    /// [`Self::create_tonic_client_from_config`], boxed as a trait object since the
    /// concrete adapter type only implements `GrpcEgressProstCodec` under this feature.
    pub fn create_prost_transport_from_config(
        config: &GrpcChannelConfig,
    ) -> Result<Box<dyn grpc_egress_transport::GrpcEgressProstCodec>, GrpcChannelConfigError> {
        if config.tls_required && TonicGrpcEgressProtocol::is_plaintext_endpoint(&config.endpoint) {
            return Err(GrpcChannelConfigError::PlaintextRejected(
                config.endpoint.clone(),
            ));
        }
        let timeout = Duration::from_secs(
            config
                .request_timeout_secs
                .unwrap_or(DEFAULT_REQUEST_TIMEOUT_SECS),
        );
        let root_ca = config.mtls.as_ref().and_then(|m| m.ca_pem_path.as_deref());
        let mut client = TonicGrpcEgress::new(&config.endpoint)
            .with_tls(config.mtls.as_ref(), root_ca)?
            .with_request_timeout(timeout);
        if let Some(keep_alive) = config.keep_alive {
            client = client.with_keep_alive(keep_alive);
        }
        client.max_message_bytes = config.max_message_bytes;
        client.compression = config.compression;
        Ok(Box::new(client))
    }
}

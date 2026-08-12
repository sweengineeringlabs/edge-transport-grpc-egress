//! `impl` blocks for [`TransportSvc`] — the type is declared in `api/types/`;
//! per `no_inherent_impl_on_api_type`, its inherent methods live here in
//! core/ (the primary-implementation layer), not in saf/. Only methods with
//! no `spi/` dependency live here — per `boundary_peer_isolation`, core/ and
//! spi/ must not import each other, so the methods that wire in concrete
//! `spi/` adapters (`create_transport_from_config` and friends) live as
//! methods on `TransportConstruction` in `saf/transport_construction.rs` instead.
//!
//! All methods here are `pub` with no private helper of their own, so their
//! tests live externally in `tests/` per `unit_tests_colocated`.

use crate::api::{ApplicationConfigBuilder, TransportSvc};

impl TransportSvc {
    /// Create a config builder pre-populated with this crate's name and version.
    pub fn create_config_builder() -> ApplicationConfigBuilder {
        let mut b = swe_edge_configbuilder::ConfigBuilderImpl::new();
        b = b.with_name(env!("CARGO_PKG_NAME"));
        b = b.with_version(env!("CARGO_PKG_VERSION"));
        ApplicationConfigBuilder(b)
    }
}

#[cfg(feature = "prost")]
mod prost_codec {
    use std::time::Duration;

    use crate::api::TransportSvc;
    use crate::api::{GrpcEgressError, GrpcEgressProstCodec, GrpcEgressResult, GrpcRequest};

    impl TransportSvc {
        /// Encode `req` via prost, dispatch `method` on `client` with `deadline`,
        /// then decode the response body via prost.
        ///
        /// Transport- and status-level errors from the underlying
        /// [`call_unary_encoded`](GrpcEgressProstCodec::call_unary_encoded) propagate
        /// unchanged. A response body that cannot be decoded is an unexpected
        /// client-side condition, mapped to [`GrpcEgressError::Internal`].
        pub async fn call_unary_typed<C, Req, Resp>(
            client: &C,
            method: &str,
            req: &Req,
            deadline: Duration,
        ) -> GrpcEgressResult<Resp>
        where
            C: GrpcEgressProstCodec + ?Sized,
            Req: prost::Message,
            Resp: prost::Message + Default + 'static,
        {
            let request = GrpcRequest::new(method, req.encode_to_vec(), deadline);
            let response = client.call_unary_encoded(request).await?;
            Resp::decode(response.body.as_slice())
                .map_err(|e| GrpcEgressError::Internal(format!("response decode failed: {e}")))
        }
    }
}

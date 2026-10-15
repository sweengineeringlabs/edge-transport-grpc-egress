//! `GrpcEgressProstCodec` — opt-in prost codec marker for outbound gRPC (ADR-026).
//!
//! Feature-gated extension trait, available only under the `prost` feature;
//! the default build gains no `prost` dependency.
//!
//! The dispatch method itself is byte-oriented — encoding and decoding via
//! `prost` happen in the generic ergonomics helper
//! [`TransportSvc::call_unary_typed`](crate::TransportSvc::call_unary_typed),
//! not here. A generic `call_unary_typed<Req, Resp>` method on this trait
//! can't be made object-safe or satisfy the api/ port-contract shape rules
//! (concrete, named `*Request`/`*Response` types); keeping the trait itself
//! concrete lets it live in api/ like every other port contract, while the
//! generic convenience wrapper lives in saf/ where genericity is unrestricted.

use futures::future::BoxFuture;

use crate::api::error::GrpcEgressError;
use crate::api::{GrpcEgress, GrpcRequest, GrpcResponse};

/// Marks a [`GrpcEgress`] implementor as usable with the generic prost
/// ergonomics helper. Blanket-implemented for every `GrpcEgress`.
pub trait GrpcEgressProstCodec: GrpcEgress {
    /// Dispatch a unary call carrying an already-prost-encoded request body,
    /// returning the still-encoded response body for the caller to decode.
    ///
    /// The default implementation simply delegates to
    /// [`call_unary`](GrpcEgress::call_unary) — this method exists so the
    /// `prost` feature has its own stable, feature-gated entry point,
    /// independent of `GrpcEgress` itself.
    fn call_unary_encoded(
        &self,
        request: GrpcRequest,
    ) -> BoxFuture<'_, Result<GrpcResponse, GrpcEgressError>> {
        self.call_unary(request)
    }
}

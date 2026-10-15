//! `DefaultResilientGrpcClientPort` — structural implementor of `ResilientGrpcClientPort`.
//!
//! Genuine circuit-breaker/retry composition is provided by the sibling
//! `edge-transport-grpc-egress-resilient` crate; this wrapper delegates every
//! `GrpcEgress` call to `inner` unchanged and always reports a closed circuit.

use std::sync::Arc;

use futures::future::BoxFuture;

use crate::api::{
    CallStreamRequest, CallUnaryWithContextRequest, CircuitStateRequest, CircuitStateResponse,
    ConsecutiveFailuresRequest, ConsecutiveFailuresResponse, GrpcEgress, GrpcEgressError,
    GrpcMessageStreamResponse, GrpcRequest, GrpcResponse, HealthCheckRequest, LastErrorRequest,
    LastErrorResponse, ResilientGrpcClientPort,
};

/// Wraps any [`GrpcEgress`] and reports a permanently closed circuit.
pub(crate) struct DefaultResilientGrpcClientPort {
    inner: Arc<dyn GrpcEgress>,
}

impl DefaultResilientGrpcClientPort {
    /// Wrap `inner` with a no-op resilience surface.
    pub(crate) fn new(inner: Arc<dyn GrpcEgress>) -> Self {
        Self { inner }
    }
}

impl GrpcEgress for DefaultResilientGrpcClientPort {
    fn call_unary(
        &self,
        request: GrpcRequest,
    ) -> BoxFuture<'_, Result<GrpcResponse, GrpcEgressError>> {
        self.inner.call_unary(request)
    }

    fn call_unary_with_context(
        &self,
        req: CallUnaryWithContextRequest,
    ) -> BoxFuture<'_, Result<GrpcResponse, GrpcEgressError>> {
        self.inner.call_unary_with_context(req)
    }

    fn call_stream(
        &self,
        req: CallStreamRequest,
    ) -> BoxFuture<'_, Result<GrpcMessageStreamResponse, GrpcEgressError>> {
        self.inner.call_stream(req)
    }

    fn call_server_stream(
        &self,
        request: GrpcRequest,
    ) -> BoxFuture<'_, Result<GrpcMessageStreamResponse, GrpcEgressError>> {
        self.inner.call_server_stream(request)
    }

    fn call_client_stream(
        &self,
        req: CallStreamRequest,
    ) -> BoxFuture<'_, Result<GrpcResponse, GrpcEgressError>> {
        self.inner.call_client_stream(req)
    }

    fn call_bidi_stream(
        &self,
        req: CallStreamRequest,
    ) -> BoxFuture<'_, Result<GrpcMessageStreamResponse, GrpcEgressError>> {
        self.inner.call_bidi_stream(req)
    }

    fn health_check(&self, req: HealthCheckRequest) -> BoxFuture<'_, Result<(), GrpcEgressError>> {
        self.inner.health_check(req)
    }
}

#[cfg(feature = "prost")]
impl crate::api::GrpcEgressProstCodec for DefaultResilientGrpcClientPort {}

impl ResilientGrpcClientPort for DefaultResilientGrpcClientPort {
    fn circuit_state(
        &self,
        _req: CircuitStateRequest,
    ) -> Result<CircuitStateResponse, GrpcEgressError> {
        Ok(CircuitStateResponse { state: "Closed" })
    }

    fn consecutive_failures(
        &self,
        _req: ConsecutiveFailuresRequest,
    ) -> Result<ConsecutiveFailuresResponse, GrpcEgressError> {
        Ok(ConsecutiveFailuresResponse { count: 0 })
    }

    fn last_error(&self, _req: LastErrorRequest) -> Result<LastErrorResponse, GrpcEgressError> {
        Ok(LastErrorResponse { error: None })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::GrpcChannelConfig;
    use crate::saf::TransportConstruction;

    fn wrapped() -> DefaultResilientGrpcClientPort {
        let cfg = GrpcChannelConfig::new("http://127.0.0.1:50999").allow_plaintext();
        let inner =
            TransportConstruction::create_transport_from_config(&cfg).expect("create transport");
        DefaultResilientGrpcClientPort::new(inner)
    }

    #[test]
    fn test_circuit_state_new_client_reports_closed() {
        let client = wrapped();
        let resp = client.circuit_state(CircuitStateRequest).expect("state");
        assert_eq!(resp.state, "Closed");
    }

    #[test]
    fn test_consecutive_failures_new_client_reports_zero() {
        let client = wrapped();
        let resp = client
            .consecutive_failures(ConsecutiveFailuresRequest)
            .expect("count");
        assert_eq!(resp.count, 0);
    }

    #[test]
    fn test_last_error_new_client_reports_none() {
        let client = wrapped();
        let resp = client.last_error(LastErrorRequest).expect("last error");
        assert!(resp.error.is_none());
    }
}

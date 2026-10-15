//! `GrpcEgress` trait — makes outbound gRPC calls.

use futures::future::BoxFuture;

use crate::api::error::GrpcEgressError;
use crate::api::types::GrpcMessageStreamResponse;
use crate::api::types::{
    CallStreamRequest, CallUnaryWithContextRequest, GrpcRequest, GrpcResponse, GrpcStatusCode,
    HealthCheckRequest,
};
use crate::api::{CompressionMode, Conversions};

/// Makes outbound gRPC calls to remote services.
pub trait GrpcEgress: Send + Sync {
    /// Send a single unary gRPC request and receive a single response.
    fn call_unary(
        &self,
        request: GrpcRequest,
    ) -> BoxFuture<'_, Result<GrpcResponse, GrpcEgressError>>;

    /// Send a single unary gRPC request, propagating the caller's security context.
    ///
    /// The default implementation delegates to [`call_unary`] and ignores the context.
    /// Override to inject context-derived metadata (e.g. `x-trace-id`, JWT forwarding).
    ///
    /// [`call_unary`]: GrpcEgress::call_unary
    fn call_unary_with_context(
        &self,
        req: CallUnaryWithContextRequest,
    ) -> BoxFuture<'_, Result<GrpcResponse, GrpcEgressError>> {
        self.call_unary(req.request)
    }

    /// Send a streaming gRPC request and receive a response stream.
    ///
    /// The default implementation returns `Unimplemented` — override to enable streaming.
    fn call_stream(
        &self,
        req: CallStreamRequest,
    ) -> BoxFuture<'_, Result<GrpcMessageStreamResponse, GrpcEgressError>> {
        let _ = req;
        Box::pin(futures::future::ready(Err(GrpcEgressError::Status(
            GrpcStatusCode::Unimplemented,
            "streaming not supported".into(),
        ))))
    }

    /// Send a server-streaming request — single request, streaming response.
    ///
    /// The default implementation returns `Unimplemented` — override to enable
    /// server-streaming.
    fn call_server_stream(
        &self,
        request: GrpcRequest,
    ) -> BoxFuture<'_, Result<GrpcMessageStreamResponse, GrpcEgressError>> {
        let _ = request;
        Box::pin(futures::future::ready(Err(GrpcEgressError::Status(
            GrpcStatusCode::Unimplemented,
            "server streaming not supported".into(),
        ))))
    }

    /// Send a client-streaming request — streaming request messages, single response.
    ///
    /// The default implementation returns `Unimplemented` — override to enable
    /// client-streaming.
    fn call_client_stream(
        &self,
        req: CallStreamRequest,
    ) -> BoxFuture<'_, Result<GrpcResponse, GrpcEgressError>> {
        let _ = req;
        Box::pin(futures::future::ready(Err(GrpcEgressError::Status(
            GrpcStatusCode::Unimplemented,
            "client streaming not supported".into(),
        ))))
    }

    /// Send a bidirectional-streaming request — streaming in both directions.
    ///
    /// The default implementation delegates to [`call_stream`].
    ///
    /// [`call_stream`]: GrpcEgress::call_stream
    fn call_bidi_stream(
        &self,
        req: CallStreamRequest,
    ) -> BoxFuture<'_, Result<GrpcMessageStreamResponse, GrpcEgressError>> {
        self.call_stream(req)
    }

    /// Check that the remote endpoint is reachable.
    fn health_check(&self, req: HealthCheckRequest) -> BoxFuture<'_, Result<(), GrpcEgressError>>;

    /// Report whether `mode` applies wire-level compression — gives
    /// [`CompressionMode`] a genuine role in this trait's signature set,
    /// not just an internal channel-config field. `Self: Sized` keeps this
    /// trait dyn-compatible for `Box<dyn Trait>`.
    fn describe_compression(mode: CompressionMode) -> bool
    where
        Self: Sized,
    {
        !matches!(mode, CompressionMode::None)
    }

    /// Construct the status-code conversion marker — gives [`Conversions`]
    /// a genuine role in this trait's signature set, not just an
    /// impl-site helper. `Self: Sized` keeps this trait dyn-compatible for
    /// `Box<dyn Trait>`.
    fn default_conversions() -> Conversions
    where
        Self: Sized,
    {
        Conversions
    }
}

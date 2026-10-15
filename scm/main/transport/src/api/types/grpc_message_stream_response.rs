//! `GrpcMessageStreamResponse` — response wrapping a stream of gRPC message payloads.

use std::pin::Pin;

use futures::Stream;

use crate::api::types::GrpcEgressResult;

/// A stream of gRPC message payloads (each item is a raw decoded frame body).
///
/// Wraps `futures::Stream` as a local newtype so the external crate type
/// never appears directly in an api/ trait signature.
pub struct GrpcMessageStreamResponse {
    /// The underlying stream of decoded frame payloads.
    pub stream: Pin<Box<dyn Stream<Item = GrpcEgressResult<Vec<u8>>> + Send>>,
}

//! `GrpcRetryFactory` -- wraps a client with retry.

use proto_pattern::{RawTransport, Transport};

use crate::BuildGrpcRetryRequest;

/// Wraps a client with retry.
///
/// The client is a [`Transport`] and a [`RawTransport`]: the wrapper retries a unary call, typed or raw, and
/// forwards every streaming call as it is. The product is a [`Transport`] and a [`RawTransport`] too, so it composes
/// with the other wrappers in any order.
pub trait GrpcRetryFactory {
    /// A client that retries the unary calls of `req.inner` as its policy says.
    fn build<T: Transport + RawTransport + Send + Sync + 'static>(
        &self,
        req: BuildGrpcRetryRequest<T>,
    ) -> impl Transport + RawTransport + use<Self, T>;
}
